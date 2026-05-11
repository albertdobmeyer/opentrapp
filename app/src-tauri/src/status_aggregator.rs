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

use crate::lifecycle::{BootstrapProgress, BootstrapState, TenantState, PerimeterStateStore};
use crate::orchestrator::state::AppState;

const ANTHROPIC_MODELS_URL: &str = "https://api.anthropic.com/v1/models";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const AUTH_PROBE_TTL: Duration = Duration::from_secs(300);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

// ─── Public types (frontend-visible) ──────────────────────────────────

/// Aggregated user-facing status. Each value maps to one hero card state
/// in `HeroStatusCard.tsx`. Snake-case matches Rust serde rename.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssistantStatus {
    /// First-launch setup; .env being written. Passive — no user action.
    Installing,
    /// First-time image build/pull/shell-up in progress (~5 min).
    Bootstrapping,
    /// Shell ready; user hasn't activated their assistant yet.
    ShellReadyAbsent,
    /// Shell failed or partially up; recovery card surfaced.
    ShellFailed,
    /// Wizard committed, bringing vault-agent up. Brief.
    #[allow(dead_code)]
    Starting,
    /// Agent running, key probe says auth is valid.
    Ok,
    /// User-initiated stop; persists via `~/.lobster-trapp/paused`.
    PausedByUser,
    /// Agent expected up but absent; auto-restart hasn't recovered.
    ErrorPerimeter,
    /// Containers healthy but Anthropic rejected the key.
    ErrorKey,
    /// Wizard hasn't run, or .env is missing the Anthropic key.
    /// Retained for backward compatibility; v0.4 flow reaches this via
    /// `(ShellReady, Running)` with no Anthropic key present.
    NotSetup,
    /// 1–3 of 4 containers running. Legacy; retained for compat.
    #[allow(dead_code)]
    Recovering,
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

/// Summary of a bootstrap pipeline failure. Surfaced to the frontend so
/// the recovery card can show cause-appropriate copy and a "Show details"
/// disclosure without requiring a separate IPC call.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BootstrapFailureSummary {
    pub cause: String,
    pub message: String,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct AssistantStatusSnapshot {
    pub status: AssistantStatus,
    pub alerts: Vec<Alert>,
    pub last_checked_unix_ms: u64,
    /// Populated only when `status == "shell_failed"`. Lets the frontend
    /// show cause-appropriate recovery copy without a separate IPC call.
    pub bootstrap_failure: Option<BootstrapFailureSummary>,
}

impl AssistantStatusSnapshot {
    fn empty() -> Self {
        Self {
            status: AssistantStatus::Installing,
            alerts: Vec::new(),
            last_checked_unix_ms: 0,
            bootstrap_failure: None,
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
    let (bootstrap, tenant) = handle
        .try_state::<PerimeterStateStore>()
        .and_then(|store| {
            store
                .status
                .lock()
                .ok()
                .map(|g| (g.bootstrap.clone(), g.tenant.clone()))
        })
        .unwrap_or((BootstrapState::Installing, TenantState::Absent));

    let paused = matches!(tenant, TenantState::Paused);

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

    // 4. Derive aggregated status. Pause is already encoded in TenantState::Paused.
    let status = derive_status(&bootstrap, &tenant, has_anthropic, key_valid);

    // 5. Compose the alerts list. Suppressed entirely when paused — the
    //    user knows their assistant is off; nothing to alarm about.
    let migration_warn = handle
        .try_state::<PerimeterStateStore>()
        .and_then(|s| s.migration_credential_warning.read().ok().map(|g| *g))
        .unwrap_or(false);

    let alerts = if paused {
        Vec::new()
    } else {
        build_alerts(&bootstrap, &tenant, has_anthropic, has_telegram, key_valid, migration_warn)
    };

    // When the shell failed, surface the failure cause from the store so the
    // frontend recovery card can show appropriate copy without a separate call.
    let bootstrap_failure = if matches!(status, AssistantStatus::ShellFailed) {
        handle
            .try_state::<PerimeterStateStore>()
            .and_then(|store| {
                store.bootstrap_progress.read().ok().and_then(|g| {
                    match g.as_ref() {
                        Some(BootstrapProgress::Failed { cause, message, last_error }) => {
                            Some(BootstrapFailureSummary {
                                cause: cause.clone(),
                                message: message.clone(),
                                last_error: last_error.clone(),
                            })
                        }
                        _ => None,
                    }
                })
            })
    } else {
        None
    };

    AssistantStatusSnapshot {
        status,
        alerts,
        last_checked_unix_ms: now_unix_ms(),
        bootstrap_failure,
    }
}

// ─── Pure helpers (unit-testable) ─────────────────────────────────────

fn derive_status(
    bootstrap: &BootstrapState,
    tenant: &TenantState,
    has_anthropic: bool,
    key_valid: Option<bool>,
) -> AssistantStatus {
    match (bootstrap, tenant) {
        (BootstrapState::Installing, _) => AssistantStatus::Installing,
        (BootstrapState::Bootstrapping, _) => AssistantStatus::Bootstrapping,
        (BootstrapState::ShellFailed, _) => AssistantStatus::ShellFailed,
        (BootstrapState::ShellReady, TenantState::Absent) => AssistantStatus::ShellReadyAbsent,
        (BootstrapState::ShellReady, TenantState::Activating) => AssistantStatus::Starting,
        (BootstrapState::ShellReady, TenantState::Paused) => AssistantStatus::PausedByUser,
        (BootstrapState::ShellReady, TenantState::Errored) => AssistantStatus::ErrorPerimeter,
        (BootstrapState::ShellReady, TenantState::Running) => {
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
    bootstrap: &BootstrapState,
    tenant: &TenantState,
    has_anthropic: bool,
    has_telegram: bool,
    key_valid: Option<bool>,
    migration_credential_warning: bool,
) -> Vec<Alert> {
    let mut alerts = Vec::new();

    // No actionable alerts while bootstrapping or installing.
    if matches!(
        bootstrap,
        BootstrapState::Installing | BootstrapState::Bootstrapping
    ) {
        return alerts;
    }

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

    if matches!(bootstrap, BootstrapState::ShellFailed)
        || matches!(tenant, TenantState::Errored)
    {
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

    // Migration: existing v0.3 install found a revoked Anthropic key on first launch.
    if migration_credential_warning
        && matches!(bootstrap, BootstrapState::ShellReady)
        && matches!(tenant, TenantState::Absent)
    {
        alerts.push(Alert {
            id: "migration-credential-warning".to_string(),
            severity: AlertSeverity::Warning,
            title: "Your Anthropic key needs updating".to_string(),
            body: Some(
                "It used to work, but it doesn't anymore — likely the key was rotated or revoked. Tap Launch to update it.".to_string(),
            ),
            cta_label: None,
            cta_to: None,
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
            derive_status(&BootstrapState::ShellReady, &TenantState::Running, true, Some(true)),
            AssistantStatus::Ok
        );
    }

    #[test]
    fn derive_running_with_invalid_key_is_error_key() {
        assert_eq!(
            derive_status(&BootstrapState::ShellReady, &TenantState::Running, true, Some(false)),
            AssistantStatus::ErrorKey
        );
    }

    #[test]
    fn derive_running_with_inconclusive_probe_stays_ok() {
        // Optimistic on transient issues — don't alarm on network errors.
        assert_eq!(
            derive_status(&BootstrapState::ShellReady, &TenantState::Running, true, None),
            AssistantStatus::Ok
        );
    }

    #[test]
    fn derive_running_without_anthropic_key_is_not_setup() {
        assert_eq!(
            derive_status(&BootstrapState::ShellReady, &TenantState::Running, false, None),
            AssistantStatus::NotSetup
        );
    }

    #[test]
    fn derive_errored_tenant_is_error_perimeter() {
        assert_eq!(
            derive_status(&BootstrapState::ShellReady, &TenantState::Errored, true, Some(true)),
            AssistantStatus::ErrorPerimeter
        );
    }

    #[test]
    fn derive_shell_failed_is_shell_failed() {
        assert_eq!(
            derive_status(&BootstrapState::ShellFailed, &TenantState::Absent, true, Some(true)),
            AssistantStatus::ShellFailed
        );
    }

    #[test]
    fn derive_shell_ready_absent_is_shell_ready_absent() {
        assert_eq!(
            derive_status(&BootstrapState::ShellReady, &TenantState::Absent, true, Some(true)),
            AssistantStatus::ShellReadyAbsent
        );
    }

    #[test]
    fn derive_installing_is_installing() {
        assert_eq!(
            derive_status(&BootstrapState::Installing, &TenantState::Absent, false, None),
            AssistantStatus::Installing
        );
    }

    #[test]
    fn derive_bootstrapping_is_bootstrapping() {
        assert_eq!(
            derive_status(&BootstrapState::Bootstrapping, &TenantState::Absent, false, None),
            AssistantStatus::Bootstrapping
        );
    }

    #[test]
    fn derive_paused_overrides_running() {
        // TenantState::Paused should yield PausedByUser regardless of key state.
        assert_eq!(
            derive_status(&BootstrapState::ShellReady, &TenantState::Paused, true, Some(true)),
            AssistantStatus::PausedByUser
        );
    }

    // ---- build_alerts ----

    #[test]
    fn alerts_when_no_anthropic_key() {
        let alerts = build_alerts(
            &BootstrapState::ShellReady,
            &TenantState::Running,
            false,
            true,
            None,
            false,
        );
        assert!(alerts.iter().any(|a| a.id == "missing-anthropic-key"));
        assert!(!alerts.iter().any(|a| a.id == "invalid-anthropic-key"));
    }

    #[test]
    fn alerts_when_key_invalid_but_present() {
        let alerts = build_alerts(
            &BootstrapState::ShellReady,
            &TenantState::Running,
            true,
            true,
            Some(false),
            false,
        );
        assert!(alerts.iter().any(|a| a.id == "invalid-anthropic-key"));
        assert!(!alerts.iter().any(|a| a.id == "missing-anthropic-key"));
    }

    #[test]
    fn alerts_when_no_telegram_token() {
        let alerts = build_alerts(
            &BootstrapState::ShellReady,
            &TenantState::Running,
            true,
            false,
            Some(true),
            false,
        );
        assert!(alerts.iter().any(|a| a.id == "missing-telegram-token"));
    }

    #[test]
    fn alerts_when_shell_failed() {
        let alerts = build_alerts(
            &BootstrapState::ShellFailed,
            &TenantState::Absent,
            true,
            true,
            Some(true),
            false,
        );
        assert!(alerts.iter().any(|a| a.id == "perimeter-error"));
    }

    #[test]
    fn alerts_when_tenant_errored() {
        let alerts = build_alerts(
            &BootstrapState::ShellReady,
            &TenantState::Errored,
            true,
            true,
            Some(true),
            false,
        );
        assert!(alerts.iter().any(|a| a.id == "perimeter-error"));
    }

    #[test]
    fn no_alerts_during_installing() {
        let alerts = build_alerts(
            &BootstrapState::Installing,
            &TenantState::Absent,
            false,
            false,
            None,
            false,
        );
        assert!(alerts.is_empty());
    }

    #[test]
    fn no_alerts_during_bootstrapping() {
        let alerts = build_alerts(
            &BootstrapState::Bootstrapping,
            &TenantState::Absent,
            false,
            false,
            None,
            false,
        );
        assert!(alerts.is_empty());
    }

    #[test]
    fn no_alerts_when_all_good() {
        let alerts = build_alerts(
            &BootstrapState::ShellReady,
            &TenantState::Running,
            true,
            true,
            Some(true),
            false,
        );
        assert!(alerts.is_empty());
    }

    #[test]
    fn missing_key_alert_suppresses_during_wizard() {
        let alerts = build_alerts(
            &BootstrapState::ShellReady,
            &TenantState::Running,
            false,
            true,
            None,
            false,
        );
        let missing = alerts
            .iter()
            .find(|a| a.id == "missing-anthropic-key")
            .unwrap();
        assert!(missing.suppress_during_wizard);
    }

    #[test]
    fn perimeter_error_alert_does_not_suppress_during_wizard() {
        let alerts = build_alerts(
            &BootstrapState::ShellFailed,
            &TenantState::Absent,
            true,
            true,
            Some(true),
            false,
        );
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
