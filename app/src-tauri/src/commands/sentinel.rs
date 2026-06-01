//! Sentinel bridge — the GUI's consumer of the shared judgment lib.
//!
//! Sentinel is a lib-first bash/Python layer (`sentinel/`) that any shield
//! calls directly against local Ollama (ADR-0015, spec 01 §5). This module is
//! the **GUI binding**: it exposes the rung-2 `judge` call as a Tauri command
//! and drives the user-visible **activity indicator** so the user always knows
//! when their machine is doing semantic judgment ("never wonder why it got
//! busy" — spec 01 §6).
//!
//! It stays generic-backend-clean (CLAUDE.md §5): it locates and runs the
//! shared lib and shuttles opaque JSON; it contains no concern-specific logic
//! (it doesn't know what a "skill" or a "feed post" is — the lib judges opaque
//! fragments + a typed context).
//!
//! Slice 1 (this module): the `judge` bridge + the activity indicator. The
//! `score`/`drift` rung-1 bridges and the rich rung-3 escalation UX land with
//! the skills/social GUI legs.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;

use crate::util::shell::find_bash;

/// How long to wait on the local judge before giving up (matches the lib's
/// own SENTINEL_TIMEOUT default headroom; the judge is load-on-demand so a
/// cold start can take a few seconds).
const JUDGE_TIMEOUT: Duration = Duration::from_secs(90);

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

/// The Verdict the lib returns (mirrors `sentinel/verdict-schema.json`). The
/// `reason` is user-facing plain language.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Verdict {
    pub decision: String,
    pub confidence: f64,
    pub resolved_at_rung: u8,
    pub reason: String,
}

impl Verdict {
    /// The fail-safe verdict when the lib can't be run or its output can't be
    /// parsed — mirrors judge.sh's own escalate-on-uncertainty contract so the
    /// caller decides its policy rather than defaulting to allow.
    fn escalate(reason: &str) -> Self {
        Self {
            decision: "escalate".to_string(),
            confidence: 0.0,
            resolved_at_rung: 2,
            reason: reason.to_string(),
        }
    }
}

/// Parse the lib's stdout into a Verdict. Pure + unit-testable. Any malformed
/// output becomes an `escalate` verdict (never a silent `allow`).
pub fn parse_verdict(stdout: &str) -> Verdict {
    let line = stdout.trim();
    if line.is_empty() {
        return Verdict::escalate("The local check produced no answer; please review manually.");
    }
    match serde_json::from_str::<Verdict>(line) {
        Ok(v) if matches!(v.decision.as_str(), "allow" | "block" | "escalate") => v,
        _ => Verdict::escalate("The local check returned an unreadable answer; please review manually."),
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

/// Locate the shared `sentinel/` lib. Candidate order mirrors the manifest
/// resolver (`manifest_cmds.rs`): bundled resource → staged runtime dir → a
/// dev fallback relative to the crate (so `cargo`/`npm run dev` work from the
/// repo). Returns the dir that actually contains `judge.sh`.
fn locate_sentinel_dir(app: &AppHandle) -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        cands.push(res.join("sentinel"));
    }
    cands.push(crate::orchestrator::podman::resource_dir().join("sentinel"));
    // Dev fallback: repo-root sentinel/ relative to app/src-tauri.
    cands.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sentinel"));
    cands.into_iter().find(|p| p.join("judge.sh").is_file())
}

// ─── the judge bridge ─────────────────────────────────────────────────

/// Run the rung-2 judge on an opaque request. The request is the same JSON the
/// CLI path uses (`context` / `fragment` / `task_hint` / `static_signal`); it
/// is passed straight to `sentinel/judge.sh` on stdin. Drives the activity
/// indicator: `thinking` while the judge runs, back to `watching` after.
#[tauri::command]
pub async fn sentinel_judge(app: AppHandle, request: serde_json::Value) -> Result<Verdict, String> {
    let dir = locate_sentinel_dir(&app)
        .ok_or_else(|| "The local judgment helper is not available.".to_string())?;
    let bash = find_bash().ok_or_else(|| "Could not find a shell to run the local check.".to_string())?;
    let request_json = serde_json::to_string(&request).map_err(|e| e.to_string())?;

    set_activity(&app, SentinelRung::Thinking);
    let result = run_judge(&bash, &dir, &request_json).await;
    set_activity(&app, SentinelRung::Watching);

    Ok(result)
}

async fn run_judge(bash: &PathBuf, dir: &PathBuf, request_json: &str) -> Verdict {
    let spawn = TokioCommand::new(bash)
        .arg(dir.join("judge.sh"))
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match spawn {
        Ok(c) => c,
        Err(_) => return Verdict::escalate("The local check could not be started."),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request_json.as_bytes()).await;
        // Drop closes stdin so judge.sh's `cat` returns.
    }

    let out = match tokio::time::timeout(JUDGE_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(_)) => return Verdict::escalate("The local check failed to run."),
        Err(_) => return Verdict::escalate("The local check took too long and was stopped."),
    };

    parse_verdict(&String::from_utf8_lossy(&out.stdout))
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
    fn parse_verdict_accepts_a_valid_verdict() {
        let v = parse_verdict(r#"{"decision":"block","confidence":0.9,"resolved_at_rung":2,"reason":"reads your saved passwords"}"#);
        assert_eq!(v.decision, "block");
        assert_eq!(v.resolved_at_rung, 2);
    }

    #[test]
    fn parse_verdict_escalates_on_empty() {
        assert_eq!(parse_verdict("").decision, "escalate");
        assert_eq!(parse_verdict("   \n").decision, "escalate");
    }

    #[test]
    fn parse_verdict_escalates_on_garbage_never_allows() {
        // A malformed answer must NEVER silently become allow.
        let v = parse_verdict("not json at all");
        assert_eq!(v.decision, "escalate");
        let v2 = parse_verdict(r#"{"decision":"yolo","confidence":1.0,"resolved_at_rung":2,"reason":"x"}"#);
        assert_eq!(v2.decision, "escalate");
    }
}
