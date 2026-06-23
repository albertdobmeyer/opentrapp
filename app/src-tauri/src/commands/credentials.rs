//! Credential validation + activation-commit commands.
//!
//! The security-load-bearing parts were lifted into `opentrapp_core::credentials` (ADR-0022
//! migration step 1) so the same transport-neutral fns back both this shim and the future loopback
//! web route: `validate_anthropic_key` (host-side pre-flight ping, errors run through
//! `redact_secrets`) and the runtime `.env` **0600** writer (`write_credentials_at`).
//!
//! What deliberately STAYS here is the GUI orchestration that cannot be transport-neutral:
//! `commit_activation` restarts vault-proxy to pick up new keys, brings vault-agent up, writes the
//! activated + credentials-ok markers, and updates the in-memory `PerimeterStateStore`; plus the
//! `AppHandle`→`runtime_data_dir` path resolution. The shims resolve the `.env` path, then hand the
//! write to core. Command names + signatures + `lib.rs` registration unchanged.

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager as _};

use opentrapp_core::credentials::{write_credentials_at, ValidationOutcome};

use crate::lifecycle::{write_activated_marker, write_credentials_ok_marker, PerimeterStateStore};
use crate::orchestrator::podman;
use crate::orchestrator::state::AppState;

/// Live-pings Anthropic to verify the key is accepted (see `opentrapp_core::credentials`). All
/// API-level responses come back as `Ok(outcome)`; only a complete network failure is an `Err`
/// (already redacted in core).
#[tauri::command]
pub async fn validate_anthropic_key(key: String) -> Result<ValidationOutcome, String> {
    opentrapp_core::credentials::validate_anthropic_key(key).await
}

/// Force-recreates vault-proxy (so it reads new .env keys), brings
/// vault-agent up, and writes the activated + credentials-ok markers.
///
/// The frontend must write both keys to `.env` via `save_credentials` BEFORE
/// calling this command. The commit is sequenced this way so the frontend
/// can show per-field validation errors before any persistent state changes.
#[tauri::command]
pub async fn commit_activation(handle: AppHandle) -> Result<(), String> {
    let root = handle
        .try_state::<AppState>()
        .and_then(|s| s.runtime_data_dir.read().ok().map(|r| r.clone()))
        .unwrap_or_else(crate::orchestrator::podman::runtime_data_dir);

    // Force-recreate vault-proxy so it picks up the fresh .env keys.
    let root_clone = root.clone();
    let ok_proxy = tokio::task::spawn_blocking(move || {
        podman::service_up(&root_clone, "vault-proxy", true).is_ok()
    })
    .await
    .unwrap_or(false);

    if !ok_proxy {
        return Err(
            "Failed to restart the security proxy. Check that your container runtime is running."
                .to_string(),
        );
    }

    // Bring vault-agent up.
    let ok_agent = tokio::task::spawn_blocking(move || {
        podman::service_up(&root, "vault-agent", false).is_ok()
    })
    .await
    .unwrap_or(false);

    if !ok_agent {
        return Err("Failed to start your assistant. Check the logs for details.".to_string());
    }

    // Write marker files.
    write_activated_marker().map_err(|e| e.to_string())?;
    write_credentials_ok_marker().map_err(|e| e.to_string())?;

    // Update in-memory store so the watchdog's next tick reflects activation
    // without waiting for it to re-read the disk markers.
    if let Some(store) = handle.try_state::<PerimeterStateStore>() {
        if let Ok(mut g) = store.activated.write() {
            *g = true;
        }
        if let Ok(mut g) = store.credentials_ok_at.write() {
            *g = Some(now_unix_ms());
        }
        // Clear any migration credential warning — new keys just validated.
        if let Ok(mut g) = store.migration_credential_warning.write() {
            *g = false;
        }
    }

    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ─── Runtime credentials (.env in the data dir) ───────────────────────────
//
// The runtime `.env` lives in the data dir (`~/.opentrapp/.env`) — where the
// bootstrap (`bootstrap::run_bootstrap` step_write_env) and the perimeter read
// it. Resolving that path needs the AppHandle (the `AppState` runtime_data_dir),
// so it stays GUI-side; the write itself is the transport-neutral 0600 writer in
// `opentrapp_core::credentials::write_credentials_at`.

fn runtime_env_path(handle: &AppHandle) -> std::path::PathBuf {
    let root = handle
        .try_state::<AppState>()
        .and_then(|s| s.runtime_data_dir.read().ok().map(|r| r.clone()))
        .unwrap_or_else(podman::runtime_data_dir);
    root.join(".env")
}

/// Write the agent credentials to the runtime `.env` (`~/.opentrapp/.env`).
/// Only non-empty keys are upserted; existing vars are preserved; the file is
/// tightened to 0600 (in core). Replaces the wizard's old
/// `write_config("agent",".env")` path that failed on packaged first-run.
#[tauri::command]
pub fn save_credentials(
    handle: AppHandle,
    anthropic_key: String,
    telegram_token: String,
) -> Result<(), String> {
    let env_path = runtime_env_path(&handle);
    write_credentials_at(&env_path, anthropic_key.trim(), telegram_token.trim())
        .map_err(|e| format!("Couldn't write your keys to the configuration: {e}"))
}

/// Read the runtime `.env` body so the wizard can pre-populate masked existing
/// keys. Returns an empty string when the file doesn't exist yet (first-run).
#[tauri::command]
pub fn read_runtime_env(handle: AppHandle) -> Result<String, String> {
    let env_path = runtime_env_path(&handle);
    Ok(std::fs::read_to_string(&env_path).unwrap_or_default())
}
