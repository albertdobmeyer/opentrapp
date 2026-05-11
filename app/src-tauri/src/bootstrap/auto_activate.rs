//! Post-bootstrap auto-activation dispatcher.
//!
//! Called after `verify-shell` succeeds. Checks marker files and decides
//! whether to bring vault-agent up automatically. Three branches:
//!
//! 1. Fresh install or user reset: markers absent → stay `(ShellReady, Absent)`;
//!    the user will click "Launch your assistant" when ready.
//! 2. Existing install needing re-validation: activated marker present but
//!    credentials-ok is missing or stale → live-ping Anthropic. If valid,
//!    write marker and commit; if auth failure, clear credential marker and
//!    stay Absent (activation modal will re-surface).
//! 3. Validated existing install: both markers present and fresh → commit
//!    immediately (bring vault-agent up).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager as _};

use crate::lifecycle::{
    is_activated_persisted, is_paused_persisted, run_compose, write_credentials_ok_marker,
    clear_credentials_ok_marker, BootstrapProgress, BootstrapStep, PerimeterStateStore,
};
use crate::orchestrator::state::AppState;

const CREDENTIALS_OK_TTL_DAYS: u64 = 7;

pub async fn after_shell_ready(handle: AppHandle, root: PathBuf) {
    let activated = is_activated_persisted();
    let paused = is_paused_persisted();

    // Branch 1: not activated yet (first launch or user reset).
    // State computes to (ShellReady, Absent); user clicks "Launch your assistant".
    if !activated {
        eprintln!("[auto-activate] not activated — staying (ShellReady, Absent)");
        return;
    }

    // Don't bring the agent up if the user explicitly paused.
    if paused {
        eprintln!("[auto-activate] paused marker present — skipping agent start");
        return;
    }

    // Read .env to check key presence.
    let env_path = vault_env_path(&handle);
    let anthropic_key = read_anthropic_key(&env_path);
    if anthropic_key.is_none() {
        eprintln!("[auto-activate] no Anthropic key in .env — staying (ShellReady, Absent)");
        return;
    }
    let key = anthropic_key.unwrap();

    // Branch 2/3: check if credentials-ok marker is fresh enough.
    let needs_revalidation = credentials_ok_stale(&handle);

    if needs_revalidation {
        eprintln!("[auto-activate] re-validating Anthropic key");
        match probe_key(&key).await {
            Some(true) => {
                // Key valid — write fresh credentials-ok marker.
                let _ = write_credentials_ok_marker();
                if let Some(store) = handle.try_state::<PerimeterStateStore>() {
                    if let Ok(mut g) = store.credentials_ok_at.write() {
                        *g = Some(now_unix_ms());
                    }
                }
            }
            Some(false) => {
                // Key explicitly rejected.
                eprintln!("[auto-activate] key rejected — clearing credentials-ok marker");
                clear_credentials_ok_marker();
                if let Some(store) = handle.try_state::<PerimeterStateStore>() {
                    if let Ok(mut g) = store.credentials_ok_at.write() {
                        *g = None;
                    }
                }
                return;
            }
            None => {
                // Inconclusive (network, 5xx, timeout) — defer; don't bring agent up.
                eprintln!("[auto-activate] key probe inconclusive — deferring auto-activate");
                return;
            }
        }
    }

    // Commit: bring vault-agent up.
    commit_agent(&handle, &root).await;
}

async fn commit_agent(handle: &AppHandle, root: &Path) {
    eprintln!("[auto-activate] committing — bringing vault-agent up");

    // Report progress so the watchdog sees UpAgent in flight.
    if let Some(store) = handle.try_state::<PerimeterStateStore>() {
        if let Ok(mut g) = store.bootstrap_progress.write() {
            *g = Some(BootstrapProgress::Step {
                step: BootstrapStep::UpAgent,
                step_index: 8,
                total_steps: 8,
                percent: None,
                detail: None,
                started_at_unix_ms: now_unix_ms(),
            });
        }
    }

    // Re-create vault-proxy first so it picks up fresh .env keys.
    let root_clone = root.to_path_buf();
    let ok_proxy = tokio::task::spawn_blocking(move || {
        run_compose(&root_clone, &["up", "-d", "--force-recreate", "vault-proxy"], Duration::from_secs(30))
    })
    .await
    .unwrap_or(false);

    if !ok_proxy {
        eprintln!("[auto-activate] vault-proxy restart failed — agent not started");
        if let Some(store) = handle.try_state::<PerimeterStateStore>() {
            if let Ok(mut g) = store.bootstrap_progress.write() {
                *g = None;
            }
        }
        return;
    }

    let root_clone = root.to_path_buf();
    let ok_agent = tokio::task::spawn_blocking(move || {
        run_compose(&root_clone, &["up", "-d", "vault-agent"], Duration::from_secs(60))
    })
    .await
    .unwrap_or(false);

    // Clear the in-flight progress marker regardless of outcome.
    // Watchdog will observe container state and set Running or Errored.
    if let Some(store) = handle.try_state::<PerimeterStateStore>() {
        if let Ok(mut g) = store.bootstrap_progress.write() {
            *g = None;
        }
    }

    if !ok_agent {
        eprintln!("[auto-activate] vault-agent start failed");
    } else {
        eprintln!("[auto-activate] vault-agent up — state will compute to (ShellReady, Running)");
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn vault_env_path(handle: &AppHandle) -> PathBuf {
    handle
        .try_state::<AppState>()
        .and_then(|state| state.monorepo_root.read().ok().map(|r| r.clone()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("components")
        .join("openclaw-vault")
        .join(".env")
}

fn read_anthropic_key(env_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(env_path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == "ANTHROPIC_API_KEY" {
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                if !v.is_empty() && !v.contains("REPLACE") {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn credentials_ok_stale(handle: &AppHandle) -> bool {
    let ts = handle
        .try_state::<PerimeterStateStore>()
        .and_then(|s| s.credentials_ok_at.read().ok().map(|g| *g))
        .flatten();

    match ts {
        None => true,
        Some(unix_ms) => {
            let age_ms = now_unix_ms().saturating_sub(unix_ms);
            let age_days = age_ms / (1000 * 60 * 60 * 24);
            age_days > CREDENTIALS_OK_TTL_DAYS
        }
    }
}

async fn probe_key(key: &str) -> Option<bool> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let resp = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => Some(true),
        Ok(r) if r.status().as_u16() == 401 || r.status().as_u16() == 403 => Some(false),
        _ => None,
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
