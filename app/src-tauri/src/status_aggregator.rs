//! Backend status aggregator + alerts evaluator (Pass 7 Day 2).
//!
//! Combines the perimeter watchdog's container health with two host-side
//! signals — `.env` key presence and an Anthropic auth probe — into a
//! single `AssistantStatus` enum that drives the Home hero state machine,
//! plus a list of `Alert`s that the proactive alerts banner subscribes to.
//!
//! Design choices:
//!
//! - **The watchdog stays the source of truth for container state.** This
//!   evaluator reads from `PerimeterStateStore` rather than re-polling
//!   containers. Two timers, one mechanical, one semantic.
//! - **Auth probe uses `/v1/models` (free) not `/v1/messages` (billable).**
//!   Same auth signal, no ongoing token cost. Cached for 5 minutes; a
//!   key rotation invalidates the cache immediately.
//! - **Inconclusive probes stay optimistic.** If Anthropic is unreachable
//!   (network, 5xx, timeout), we don't flip to `error_key` — better to
//!   stay quiet on transient issues than to alarm Karen falsely.
//! - **No spending-limit rule.** Per the 2026-05-02 vision recheck,
//!   billing alerts are Anthropic Console's job.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::lifecycle::{PerimeterState, PerimeterStateStore};
use crate::orchestrator::state::AppState;

const ANTHROPIC_MODELS_URL: &str = "https://api.anthropic.com/v1/models";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const AUTH_PROBE_TTL: Duration = Duration::from_secs(300);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

// ─── Public types (frontend-visible) ──────────────────────────────────

/// Aggregated user-facing status. Maps directly to the 7-state hero
/// machine in `docs/specs/2026-04-29-delightful-sloth-target-ux.md`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssistantStatus {
    /// Wizard hasn't run, or .env is missing the Anthropic key.
    NotSetup,
    /// Compose-up is in progress; not all containers are visible yet.
    /// Reserved for future wiring; this evaluator currently maps any
    /// partial-perimeter state to `Recovering` and lets the frontend's
    /// `hasBeenRunning` ref flip the first occurrence to "Starting".
    #[allow(dead_code)]
    Starting,
    /// 1–3 of 4 containers running.
    Recovering,
    /// All 4 containers running, key probe says auth is valid.
    Ok,
    /// All 4 containers stopped.
    ErrorPerimeter,
    /// Containers are healthy but Anthropic rejected the key.
    ErrorKey,
    /// User-initiated pause via `pause_perimeter`. Persists across app
    /// restarts via the `~/.lobster-trapp/paused` marker. Cleared by
    /// `resume_perimeter`.
    PausedByUser,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Danger,
    Warning,
    #[allow(dead_code)]
    Info,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub body: Option<String>,
    pub cta_label: Option<String>,
    pub cta_to: Option<String>,
    pub dismissable: bool,
    /// True when this alert should NOT show during the wizard. The
    /// missing-key alerts have this set so they don't clutter the
    /// onboarding flow that's literally there to fix them.
    pub suppress_during_wizard: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct AssistantStatusSnapshot {
    pub status: AssistantStatus,
    pub alerts: Vec<Alert>,
    pub last_checked_unix_ms: u64,
}

impl AssistantStatusSnapshot {
    fn empty() -> Self {
        Self {
            status: AssistantStatus::NotSetup,
            alerts: Vec::new(),
            last_checked_unix_ms: 0,
        }
    }
}

/// Tauri-managed shared state. Read by the `get_assistant_status`
/// command and updated by the evaluator task.
pub struct AssistantStatusStore(pub Mutex<AssistantStatusSnapshot>);

impl AssistantStatusStore {
    pub fn new() -> Self {
        Self(Mutex::new(AssistantStatusSnapshot::empty()))
    }
}

impl Default for AssistantStatusStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tauri command ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_assistant_status(
    store: State<'_, AssistantStatusStore>,
) -> AssistantStatusSnapshot {
    store
        .0
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| AssistantStatusSnapshot::empty())
}

// ─── Evaluator task ───────────────────────────────────────────────────

/// Spawn the 60-second status evaluator. Reads perimeter state from the
/// existing `PerimeterStateStore`, probes the Anthropic auth endpoint
/// (cached), reads `.env` for key presence, and emits an
/// `assistant-status-changed` event when the snapshot transitions.
pub fn spawn_status_evaluator(handle: AppHandle, interval: Duration) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // Skip ticks if a probe runs slow — don't pile up backlogged work.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut last_emitted: Option<AssistantStatusSnapshot> = None;
        let mut auth_cache = AuthProbeCache::new();

        loop {
            ticker.tick().await;
            let snapshot = evaluate(&handle, &mut auth_cache).await;

            // Update the store so on-demand `get_assistant_status` reads
            // are always fresh, even between transitions.
            if let Some(store) = handle.try_state::<AssistantStatusStore>() {
                if let Ok(mut guard) = store.0.lock() {
                    *guard = snapshot.clone();
                }
            }

            let changed = last_emitted.as_ref() != Some(&snapshot);
            if changed {
                let _ = handle.emit("assistant-status-changed", &snapshot);
                last_emitted = Some(snapshot);
            }
        }
    });
}

async fn evaluate(handle: &AppHandle, auth_cache: &mut AuthProbeCache) -> AssistantStatusSnapshot {
    // 1. Snapshot the perimeter state (set by the watchdog every 30s).
    let perimeter = handle
        .try_state::<PerimeterStateStore>()
        .and_then(|store| store.status.lock().ok().map(|g| g.state.clone()))
        .unwrap_or(PerimeterState::NotSetup);

    let paused = handle
        .try_state::<PerimeterStateStore>()
        .map(|store| store.is_paused())
        .unwrap_or(false);

    // 2. Read .env for key presence + Anthropic key value.
    let env_path = vault_env_path(handle);
    let (has_anthropic, has_telegram, anthropic_key) = read_env_keys(&env_path);

    // 3. Auth probe (cached, 5-min TTL, key-rotation aware). Only runs
    //    when we actually have a key to probe with.
    let key_valid = if let Some(key) = anthropic_key.as_deref() {
        probe_anthropic_auth(key, auth_cache).await
    } else {
        None
    };

    // 4. Derive aggregated status. Pause overrides everything else: a
    //    user-initiated stop should never read as "didn't recover".
    let status = derive_status(&perimeter, has_anthropic, key_valid, paused);

    // 5. Compose the alerts list. Suppressed entirely when paused — the
    //    user knows their assistant is off; nothing to alarm about.
    let alerts = if paused {
        Vec::new()
    } else {
        build_alerts(&perimeter, has_anthropic, has_telegram, key_valid)
    };

    AssistantStatusSnapshot {
        status,
        alerts,
        last_checked_unix_ms: now_unix_ms(),
    }
}

// ─── Pure helpers (unit-testable) ─────────────────────────────────────

fn derive_status(
    perimeter: &PerimeterState,
    has_anthropic: bool,
    key_valid: Option<bool>,
    paused: bool,
) -> AssistantStatus {
    if paused {
        return AssistantStatus::PausedByUser;
    }
    match perimeter {
        PerimeterState::NotSetup => AssistantStatus::NotSetup,
        PerimeterState::Starting => AssistantStatus::Starting,
        PerimeterState::Recovering => AssistantStatus::Recovering,
        PerimeterState::Stopped => AssistantStatus::ErrorPerimeter,
        PerimeterState::RunningSafely => {
            if !has_anthropic {
                AssistantStatus::NotSetup
            } else if key_valid == Some(false) {
                AssistantStatus::ErrorKey
            } else {
                AssistantStatus::Ok
            }
        }
    }
}

fn build_alerts(
    perimeter: &PerimeterState,
    has_anthropic: bool,
    has_telegram: bool,
    key_valid: Option<bool>,
) -> Vec<Alert> {
    let mut alerts = Vec::new();

    if !has_anthropic {
        alerts.push(Alert {
            id: "missing-anthropic-key".to_string(),
            severity: AlertSeverity::Danger,
            title: "Your Anthropic key isn't set".to_string(),
            body: Some(
                "Your assistant needs an Anthropic key to think. Add one in Preferences."
                    .to_string(),
            ),
            cta_label: Some("Open Preferences".to_string()),
            cta_to: Some("/preferences".to_string()),
            dismissable: false,
            suppress_during_wizard: true,
        });
    } else if key_valid == Some(false) {
        alerts.push(Alert {
            id: "invalid-anthropic-key".to_string(),
            severity: AlertSeverity::Danger,
            title: "Your Anthropic key isn't working".to_string(),
            body: Some(
                "Anthropic didn't accept the key. Update it in Preferences.".to_string(),
            ),
            cta_label: Some("Open Preferences".to_string()),
            cta_to: Some("/preferences".to_string()),
            dismissable: false,
            suppress_during_wizard: true,
        });
    }

    if !has_telegram {
        alerts.push(Alert {
            id: "missing-telegram-token".to_string(),
            severity: AlertSeverity::Warning,
            title: "Telegram isn't connected yet".to_string(),
            body: Some(
                "Add a bot token in Preferences so you can chat with your assistant."
                    .to_string(),
            ),
            cta_label: Some("Open Preferences".to_string()),
            cta_to: Some("/preferences".to_string()),
            dismissable: true,
            suppress_during_wizard: true,
        });
    }

    if matches!(perimeter, PerimeterState::Stopped) {
        alerts.push(Alert {
            id: "perimeter-error".to_string(),
            severity: AlertSeverity::Danger,
            title: "Your assistant didn't recover".to_string(),
            body: Some("Try restarting the app. If it keeps happening, get help.".to_string()),
            cta_label: Some("Get help".to_string()),
            cta_to: Some("/help".to_string()),
            dismissable: false,
            suppress_during_wizard: false,
        });
    }

    alerts
}

// ─── .env reader ──────────────────────────────────────────────────────

fn vault_env_path(handle: &AppHandle) -> PathBuf {
    let root = handle
        .try_state::<AppState>()
        .and_then(|state| state.monorepo_root.read().ok().map(|r| r.clone()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    root.join("components").join("openclaw-vault").join(".env")
}

fn read_env_keys(path: &PathBuf) -> (bool, bool, Option<String>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (false, false, None),
    };
    parse_env_keys(&content)
}

fn parse_env_keys(content: &str) -> (bool, bool, Option<String>) {
    let mut anthropic_key: Option<String> = None;
    let mut telegram_token: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let v = value.trim().trim_matches(|c| c == '"' || c == '\'');
        if v.is_empty() || v.contains("REPLACE") {
            continue;
        }
        match key.trim() {
            "ANTHROPIC_API_KEY" => anthropic_key = Some(v.to_string()),
            "TELEGRAM_BOT_TOKEN" => telegram_token = Some(v.to_string()),
            _ => {}
        }
    }

    let has_anthropic = anthropic_key.is_some();
    let has_telegram = telegram_token.is_some();
    (has_anthropic, has_telegram, anthropic_key)
}

// ─── Auth probe ───────────────────────────────────────────────────────

struct AuthProbeCache {
    last_result: Option<bool>,
    probed_at: Option<Instant>,
    last_key: Option<String>,
}

impl AuthProbeCache {
    fn new() -> Self {
        Self {
            last_result: None,
            probed_at: None,
            last_key: None,
        }
    }
}

/// Returns `Some(true)` if the key authenticates, `Some(false)` if
/// Anthropic explicitly rejected it (401/403), and `None` if the result
/// is inconclusive (network error, 5xx, timeout). Inconclusive results
/// don't fire the `error_key` alert — better to stay optimistic on
/// transient issues than to alarm falsely.
async fn probe_anthropic_auth(key: &str, cache: &mut AuthProbeCache) -> Option<bool> {
    // Cache hit when the key matches AND we're still inside the TTL.
    // A key rotation invalidates immediately because last_key won't match.
    if let (Some(last_key), Some(probed_at)) = (cache.last_key.as_ref(), cache.probed_at) {
        if last_key == key && probed_at.elapsed() < AUTH_PROBE_TTL {
            return cache.last_result;
        }
    }

    let client = match reqwest::Client::builder().timeout(HTTP_TIMEOUT).build() {
        Ok(c) => c,
        Err(_) => return None,
    };

    let resp = client
        .get(ANTHROPIC_MODELS_URL)
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .send()
        .await;

    let result = match resp {
        Ok(r) if r.status().is_success() => Some(true),
        Ok(r) => {
            let code = r.status().as_u16();
            if code == 401 || code == 403 {
                Some(false)
            } else {
                None
            }
        }
        Err(_) => None,
    };

    cache.last_key = Some(key.to_string());
    cache.probed_at = Some(Instant::now());
    cache.last_result = result;
    result
}

// ─── Misc ─────────────────────────────────────────────────────────────

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ---- derive_status ----

    #[test]
    fn derive_running_with_valid_key_is_ok() {
        assert_eq!(
            derive_status(&PerimeterState::RunningSafely, true, Some(true), false),
            AssistantStatus::Ok
        );
    }

    #[test]
    fn derive_running_with_invalid_key_is_error_key() {
        assert_eq!(
            derive_status(&PerimeterState::RunningSafely, true, Some(false), false),
            AssistantStatus::ErrorKey
        );
    }

    #[test]
    fn derive_running_with_inconclusive_probe_stays_ok() {
        // Optimistic on transient issues — don't alarm on network errors.
        assert_eq!(
            derive_status(&PerimeterState::RunningSafely, true, None, false),
            AssistantStatus::Ok
        );
    }

    #[test]
    fn derive_running_without_anthropic_key_is_not_setup() {
        assert_eq!(
            derive_status(&PerimeterState::RunningSafely, false, None, false),
            AssistantStatus::NotSetup
        );
    }

    #[test]
    fn derive_stopped_perimeter_is_error_perimeter() {
        assert_eq!(
            derive_status(&PerimeterState::Stopped, true, Some(true), false),
            AssistantStatus::ErrorPerimeter
        );
    }

    #[test]
    fn derive_recovering_passes_through() {
        assert_eq!(
            derive_status(&PerimeterState::Recovering, true, Some(true), false),
            AssistantStatus::Recovering
        );
    }

    #[test]
    fn derive_not_setup_passes_through() {
        assert_eq!(
            derive_status(&PerimeterState::NotSetup, false, None, false),
            AssistantStatus::NotSetup
        );
    }

    #[test]
    fn derive_paused_overrides_running() {
        // Even when containers are healthy, pause wins.
        assert_eq!(
            derive_status(&PerimeterState::RunningSafely, true, Some(true), true),
            AssistantStatus::PausedByUser
        );
    }

    #[test]
    fn derive_paused_overrides_stopped() {
        // The whole point of pause: a stopped perimeter that the user
        // chose isn't an error.
        assert_eq!(
            derive_status(&PerimeterState::Stopped, true, Some(true), true),
            AssistantStatus::PausedByUser
        );
    }

    #[test]
    fn derive_paused_overrides_error_key() {
        // Pause is louder than auth issues; user clearly opted out.
        assert_eq!(
            derive_status(&PerimeterState::RunningSafely, true, Some(false), true),
            AssistantStatus::PausedByUser
        );
    }

    // ---- build_alerts ----

    #[test]
    fn alerts_when_no_anthropic_key() {
        let alerts = build_alerts(&PerimeterState::NotSetup, false, true, None);
        assert!(alerts.iter().any(|a| a.id == "missing-anthropic-key"));
        assert!(!alerts.iter().any(|a| a.id == "invalid-anthropic-key"));
    }

    #[test]
    fn alerts_when_key_invalid_but_present() {
        let alerts =
            build_alerts(&PerimeterState::RunningSafely, true, true, Some(false));
        assert!(alerts.iter().any(|a| a.id == "invalid-anthropic-key"));
        assert!(!alerts.iter().any(|a| a.id == "missing-anthropic-key"));
    }

    #[test]
    fn alerts_when_no_telegram_token() {
        let alerts =
            build_alerts(&PerimeterState::RunningSafely, true, false, Some(true));
        assert!(alerts.iter().any(|a| a.id == "missing-telegram-token"));
    }

    #[test]
    fn alerts_when_perimeter_stopped() {
        let alerts = build_alerts(&PerimeterState::Stopped, true, true, Some(true));
        assert!(alerts.iter().any(|a| a.id == "perimeter-error"));
    }

    #[test]
    fn no_alerts_when_all_good() {
        let alerts =
            build_alerts(&PerimeterState::RunningSafely, true, true, Some(true));
        assert!(alerts.is_empty());
    }

    #[test]
    fn missing_key_alert_suppresses_during_wizard() {
        let alerts = build_alerts(&PerimeterState::NotSetup, false, true, None);
        let missing = alerts.iter().find(|a| a.id == "missing-anthropic-key").unwrap();
        assert!(missing.suppress_during_wizard);
    }

    #[test]
    fn perimeter_error_alert_does_not_suppress_during_wizard() {
        let alerts = build_alerts(&PerimeterState::Stopped, true, true, Some(true));
        let p = alerts.iter().find(|a| a.id == "perimeter-error").unwrap();
        assert!(!p.suppress_during_wizard);
    }

    // ---- parse_env_keys ----

    #[test]
    fn parse_env_extracts_both_keys() {
        let env = "ANTHROPIC_API_KEY=sk-ant-real\nTELEGRAM_BOT_TOKEN=123:abc\n";
        let (anthropic, telegram, key) = parse_env_keys(env);
        assert!(anthropic);
        assert!(telegram);
        assert_eq!(key, Some("sk-ant-real".to_string()));
    }

    #[test]
    fn parse_env_skips_replace_placeholder() {
        let env = "ANTHROPIC_API_KEY=REPLACE_ME\n";
        let (anthropic, _, key) = parse_env_keys(env);
        assert!(!anthropic);
        assert_eq!(key, None);
    }

    #[test]
    fn parse_env_skips_comments() {
        let env = "# ANTHROPIC_API_KEY=sk-ant-leaked\n";
        let (anthropic, _, key) = parse_env_keys(env);
        assert!(!anthropic);
        assert_eq!(key, None);
    }

    #[test]
    fn parse_env_strips_quotes() {
        let env = "ANTHROPIC_API_KEY=\"sk-ant-quoted\"\n";
        let (_, _, key) = parse_env_keys(env);
        assert_eq!(key, Some("sk-ant-quoted".to_string()));
    }

    #[test]
    fn parse_env_handles_empty_input() {
        let (a, t, k) = parse_env_keys("");
        assert!(!a);
        assert!(!t);
        assert_eq!(k, None);
    }

    // ---- AuthProbeCache TTL behavior ----

    #[test]
    fn auth_probe_cache_starts_empty() {
        let cache = AuthProbeCache::new();
        assert!(cache.last_key.is_none());
        assert!(cache.last_result.is_none());
    }
}
