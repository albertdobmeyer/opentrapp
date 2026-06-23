//! Sentinel bridge — the GUI's consumer of the shared judgment lib.
//!
//! Sentinel is a lib-first bash/Python layer (`sentinel/`) that any shield
//! calls directly against local Ollama (ADR-0015, spec 01 §5). This module is
//! the **GUI binding**: it exposes the rung-2 `judge` call as a Tauri command
//! and drives the user-visible **activity indicator** so the user always knows
//! when their machine is doing semantic judgment ("never wonder why it got
//! busy" — spec 01 §6).
//!
//! The transport-neutral judge **run + parse** (`Verdict`, `parse_verdict`,
//! `run_judge`, `judge_with`) was lifted to `opentrapp_core::sentinel` (ADR-0022
//! migration step 1) so the future loopback web route can run a judgment too.
//! What stays here is the GUI-specific binding: the activity indicator
//! (`SentinelActivityStore` + the Tauri `emit`) and lib **location**
//! (`sentinel_candidates` bakes in a crate-relative dev fallback, so it must
//! resolve against the GUI crate). `judge` locates the dir, drives the activity
//! indicator, and delegates the run to core.
//!
//! It stays generic-backend-clean (CLAUDE.md §5): it locates and runs the
//! shared lib and shuttles opaque JSON; it contains no concern-specific logic.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use opentrapp_core::sentinel::{judge_with, Verdict};

/// The active rung Sentinel is working at — drives the activity indicator.
/// User-facing labels are plain language (the 28-term banned-vocabulary rule
/// applies to everything rendered, so these must never leak jargon).
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SentinelRung {
    /// Rung 0/1 only — idle-cheap, no model running. The resting state.
    Watching,
    /// Rung 2 — the tiny local judge is loaded/active. Brief.
    Thinking,
    /// Rung 3 — a heavier, user-triggered analysis is in progress.
    DeepAnalysis,
}

/// User-facing label for a rung. Plain words only (banned-vocabulary rule).
pub fn rung_label(rung: SentinelRung) -> &'static str {
    match rung {
        SentinelRung::Watching => "watching",
        SentinelRung::Thinking => "thinking",
        SentinelRung::DeepAnalysis => "deep analysis",
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SentinelActivity {
    pub rung: SentinelRung,
    pub label: String,
    pub since_unix_ms: u64,
}

impl SentinelActivity {
    fn at(rung: SentinelRung) -> Self {
        Self {
            rung,
            label: rung_label(rung).to_string(),
            since_unix_ms: now_unix_ms(),
        }
    }
    fn watching() -> Self {
        Self::at(SentinelRung::Watching)
    }
}

/// Tauri-managed shared state. Read by `get_sentinel_activity`, updated by the
/// judge command around each call.
pub struct SentinelActivityStore(pub Mutex<SentinelActivity>);

impl SentinelActivityStore {
    pub fn new() -> Self {
        Self(Mutex::new(SentinelActivity::watching()))
    }
}

impl Default for SentinelActivityStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── activity helpers ─────────────────────────────────────────────────

fn set_activity(app: &AppHandle, rung: SentinelRung) {
    let activity = SentinelActivity::at(rung);
    if let Some(store) = app.try_state::<SentinelActivityStore>() {
        if let Ok(mut guard) = store.0.lock() {
            *guard = activity.clone();
        }
    }
    let _ = app.emit("sentinel-activity-changed", &activity);
}

#[tauri::command]
pub fn get_sentinel_activity(store: State<'_, SentinelActivityStore>) -> SentinelActivity {
    store
        .0
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| SentinelActivity::watching())
}

// ─── lib location ─────────────────────────────────────────────────────

/// Build the ordered candidate dirs for the shared `sentinel/` lib. Pure +
/// unit-testable (no `AppHandle`). Order:
///   1. bundle-direct — `<resource_dir>/perimeter/sentinel`: the verified copy
///      packaged inside the signed AppImage (`tauri.conf.json` bundles
///      `resources/perimeter`), resolvable before bootstrap staging runs.
///   2. flat resource — `<resource_dir>/sentinel`: any future flat-resource layout.
///   3. staged runtime — `<runtime_perimeter>/sentinel`: copied from the bundle
///      at first launch by `stage_resources_from_bundle` (self-heals tampering).
///   4. dev fallback — repo-root `sentinel/` relative to the crate, so
///      `cargo`/`npm run dev` work from a source checkout. (Crate-relative — the
///      reason `sentinel_candidates` stays in the GUI crate, not core.)
fn sentinel_candidates(resource_dir: Option<&Path>, runtime_perimeter: &Path) -> Vec<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(res) = resource_dir {
        cands.push(res.join("perimeter").join("sentinel"));
        cands.push(res.join("sentinel"));
    }
    cands.push(runtime_perimeter.join("sentinel"));
    cands.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sentinel"));
    cands
}

/// Locate the shared `sentinel/` lib — the first candidate that contains
/// `judge.sh`. See [`sentinel_candidates`] for the search order.
fn locate_sentinel_dir(app: &AppHandle) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().ok();
    let runtime_perimeter = crate::orchestrator::podman::resource_dir();
    sentinel_candidates(resource_dir.as_deref(), &runtime_perimeter)
        .into_iter()
        .find(|p| p.join("judge.sh").is_file())
}

// ─── the judge bridge ─────────────────────────────────────────────────

/// Run the rung-2 judge on an opaque request and return a [`Verdict`]. Reusable
/// by any GUI consumer — the `sentinel_judge` command and the egress-approval
/// path (`commands::egress`). Locates the lib (GUI, crate-relative dev fallback),
/// drives the activity indicator (`thinking` while the judge runs, back to
/// `watching` after), and delegates the run+parse to `opentrapp_core::sentinel`.
/// A missing lib returns an `escalate` verdict — never a silent allow.
pub(crate) async fn judge(app: &AppHandle, request: serde_json::Value) -> Verdict {
    let Some(dir) = locate_sentinel_dir(app) else {
        return Verdict::escalate(
            "The local judgment helper is not available; please review manually.",
        );
    };

    set_activity(app, SentinelRung::Thinking);
    let result = judge_with(&dir, request).await;
    set_activity(app, SentinelRung::Watching);
    result
}

#[tauri::command]
pub async fn sentinel_judge(app: AppHandle, request: serde_json::Value) -> Result<Verdict, String> {
    Ok(judge(&app, request).await)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ─── tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rung_labels_are_plain_language() {
        // No banned jargon — these render directly to the user.
        assert_eq!(rung_label(SentinelRung::Watching), "watching");
        assert_eq!(rung_label(SentinelRung::Thinking), "thinking");
        assert_eq!(rung_label(SentinelRung::DeepAnalysis), "deep analysis");
    }

    #[test]
    fn default_activity_is_watching() {
        let store = SentinelActivityStore::new();
        let a = store.0.lock().unwrap().clone();
        assert_eq!(a.rung, SentinelRung::Watching);
        assert_eq!(a.label, "watching");
    }

    #[test]
    fn candidates_prefer_bundled_perimeter_layout() {
        // The verified copy inside the signed AppImage lives at
        // <resource_dir>/perimeter/sentinel — it must be tried first, and the
        // staged runtime dir must also be a candidate (post-bootstrap).
        let res = Path::new("/bundle/res");
        let runtime = Path::new("/home/u/.opentrapp/perimeter");
        let cands = sentinel_candidates(Some(res), runtime);
        assert_eq!(cands[0], PathBuf::from("/bundle/res/perimeter/sentinel"));
        assert!(cands.contains(&PathBuf::from("/home/u/.opentrapp/perimeter/sentinel")));
    }

    #[test]
    fn resolves_lib_from_packaged_resource_layout_no_dev_fallback() {
        // Simulate a packaged build: a resource dir with perimeter/sentinel/judge.sh
        // present. Resolution (the same find() locate_sentinel_dir uses) must
        // pick it without depending on the crate-relative dev fallback.
        let base = std::env::temp_dir().join("opentrapp-sentinel-resolve-test");
        let staged = base.join("perimeter").join("sentinel");
        std::fs::create_dir_all(&staged).unwrap();
        std::fs::write(staged.join("judge.sh"), "#!/usr/bin/env bash\n").unwrap();

        let found = sentinel_candidates(Some(&base), Path::new("/nonexistent/perimeter"))
            .into_iter()
            .find(|p| p.join("judge.sh").is_file());

        assert_eq!(found.as_deref(), Some(staged.as_path()));
        let _ = std::fs::remove_dir_all(&base);
    }
}
