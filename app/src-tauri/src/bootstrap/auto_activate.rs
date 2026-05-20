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

use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::lifecycle::{
    is_activated_persisted, is_paused_persisted, write_activated_marker,
    write_credentials_ok_marker, clear_credentials_ok_marker,
    BootstrapProgress, BootstrapStep, PerimeterStateStore,
};
use crate::orchestrator::podman;
use crate::orchestrator::state::AppState;

const CREDENTIALS_OK_TTL_DAYS: u64 = 7;

pub async fn after_shell_ready(handle: AppHandle, root: PathBuf) {
    let activated = is_activated_persisted();
    let paused = is_paused_persisted();

    // Branch 1: markers absent — check for v0.3 migration before staying Absent.
    if !activated {
        let env_path = vault_env_path(&handle);
        let anthropic_key = read_anthropic_key(&env_path);
        let has_telegram = read_env_value(&env_path, "TELEGRAM_BOT_TOKEN").is_some();

        if let (Some(key), true) = (anthropic_key, has_telegram) {
            // v0.3 existing install: real keys present but no activated marker.
            eprintln!("[auto-activate] v0.3 install detected — running migration check");
            migrate_existing_install(handle, root, key).await;
            return;
        }

        // Fresh install: no real keys in .env.
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
    let env_path = vault_env_path(&handle);
    resolve_and_emit_bot_url(&handle, &env_path).await;
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
        podman::service_up(&root_clone, "vault-proxy", true).is_ok()
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
        podman::service_up(&root_clone, "vault-agent", false).is_ok()
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

// ─── Bot URL resolution ───────────────────────────────────────────────

/// Read the Telegram bot token from vault .env, call getMe, and emit
/// `telegram-bot-resolved` with `{url, username}` so the frontend can
/// populate its settings cache without the token ever entering the webview.
async fn resolve_and_emit_bot_url(handle: &AppHandle, env_path: &Path) {
    let Some(token) = read_env_value(env_path, "TELEGRAM_BOT_TOKEN") else {
        return;
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let resp = client
        .get(format!("https://api.telegram.org/bot{}/getMe", token))
        .send()
        .await;

    if let Ok(r) = resp {
        if let Ok(body) = r.json::<serde_json::Value>().await {
            if let Some(username) = body["result"]["username"].as_str() {
                let url = format!("https://t.me/{}?text=Hi", username);
                let _ = handle.emit(
                    "telegram-bot-resolved",
                    serde_json::json!({ "url": url, "username": username }),
                );
                eprintln!("[auto-activate] bot resolved: @{username}");
            }
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────

// ─── Migration ────────────────────────────────────────────────────────

/// Verify an existing v0.3 install's Anthropic key and, if valid, bring the
/// agent up silently. Called once per launch until markers are written.
async fn migrate_existing_install(handle: AppHandle, root: PathBuf, anthropic_key: String) {
    // Honor user's explicit pause before attempting migration.
    if is_paused_persisted() {
        eprintln!("[migration] paused marker present — honoring pause, skipping agent start");
        return;
    }

    match probe_key(&anthropic_key).await {
        Some(true) => {
            eprintln!("[migration] key valid — writing markers and bringing agent up");
            let _ = write_activated_marker();
            let _ = write_credentials_ok_marker();
            if let Some(store) = handle.try_state::<PerimeterStateStore>() {
                if let Ok(mut g) = store.activated.write() { *g = true; }
                if let Ok(mut g) = store.credentials_ok_at.write() {
                    *g = Some(now_unix_ms());
                }
                // Clear any prior credential warning.
                if let Ok(mut g) = store.migration_credential_warning.write() {
                    *g = false;
                }
            }
            commit_agent(&handle, &root).await;
            let env_path = vault_env_path(&handle);
            resolve_and_emit_bot_url(&handle, &env_path).await;
            let _ = handle.emit("migration-completed", ());
        }
        Some(false) => {
            // Key revoked since last use — user must re-enter.
            eprintln!("[migration] key rejected — signaling re-credential needed");
            if let Some(store) = handle.try_state::<PerimeterStateStore>() {
                if let Ok(mut g) = store.migration_credential_warning.write() {
                    *g = true;
                }
            }
            let _ = handle.emit("migration-needs-recredential", ());
        }
        None => {
            // Network error or inconclusive — don't block; retry on next launch.
            eprintln!("[migration] validation inconclusive — deferring to next launch");
            let _ = handle.emit("migration-deferred", ());
        }
    }
}

fn vault_env_path(handle: &AppHandle) -> PathBuf {
    handle
        .try_state::<AppState>()
        .and_then(|state| state.runtime_data_dir.read().ok().map(|r| r.clone()))
        .unwrap_or_else(podman::runtime_data_dir)
        .join(".env")
}

fn read_env_value(env_path: &Path, key_name: &str) -> Option<String> {
    let content = std::fs::read_to_string(env_path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key_name {
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                if !v.is_empty() && !v.contains("REPLACE") && v.len() >= 8 {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn read_anthropic_key(env_path: &Path) -> Option<String> {
    read_env_value(env_path, "ANTHROPIC_API_KEY")
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
