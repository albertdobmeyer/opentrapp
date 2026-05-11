//! Anthropic API-key validation and activation-commit command.
//!
//! `validate_anthropic_key` — live-pings Anthropic directly from the host
//! (not via vault-proxy) as a pre-flight check. Post-activation, all real
//! agent traffic goes through the proxy.
//!
//! `commit_activation` — called by ActivationModal after both keys are
//! validated and written to .env. Restarts vault-proxy to pick up new keys,
//! brings vault-agent up, and writes the activated + credentials-ok markers.

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager as _};
use serde::Serialize;

use crate::lifecycle::{
    run_compose, write_activated_marker, write_credentials_ok_marker, PerimeterStateStore,
};
use crate::orchestrator::state::AppState;

/// Structured outcome of a key validation attempt. Lets the frontend show
/// exact guidance per error class without parsing error strings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationOutcome {
    Ok,
    AuthFailure,
    Billing,
    Permission,
    Rate,
    ServerError,
    Unknown,
}

/// Live-pings Anthropic's `/v1/messages` with `max_tokens: 1` to verify
/// the key is accepted. Uses the cheapest current model for minimal cost.
///
/// Returns a network error string only on complete network failure —
/// all API-level responses (401, 402, …) come back as `Ok(outcome)` so
/// the frontend can give specific guidance.
#[tauri::command]
pub async fn validate_anthropic_key(key: String) -> Result<ValidationOutcome, String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Ok(ValidationOutcome::AuthFailure);
    }

    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "."}]
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| crate::lifecycle::redact_secrets(&e.to_string()))?;

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| crate::lifecycle::redact_secrets(&e.to_string()))?;

    Ok(match resp.status().as_u16() {
        200 => ValidationOutcome::Ok,
        401 => ValidationOutcome::AuthFailure,
        402 => ValidationOutcome::Billing,
        403 => ValidationOutcome::Permission,
        429 => ValidationOutcome::Rate,
        500..=599 => ValidationOutcome::ServerError,
        _ => ValidationOutcome::Unknown,
    })
}

/// Force-recreates vault-proxy (so it reads new .env keys), brings
/// vault-agent up, and writes the activated + credentials-ok markers.
///
/// The frontend must write both keys to `.env` via `write_config` BEFORE
/// calling this command. The commit is sequenced this way so the frontend
/// can show per-field validation errors before any persistent state changes.
#[tauri::command]
pub async fn commit_activation(handle: AppHandle) -> Result<(), String> {
    let root = handle
        .try_state::<AppState>()
        .and_then(|s| s.monorepo_root.read().ok().map(|r| r.clone()))
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    // Force-recreate vault-proxy so it picks up the fresh .env keys.
    let root_clone = root.clone();
    let ok_proxy = tokio::task::spawn_blocking(move || {
        run_compose(
            &root_clone,
            &["up", "-d", "--force-recreate", "vault-proxy"],
            Duration::from_secs(30),
        )
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
        run_compose(&root, &["up", "-d", "vault-agent"], Duration::from_secs(60))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_key_returns_auth_failure() {
        let result = validate_anthropic_key(String::new()).await;
        assert!(matches!(result, Ok(ValidationOutcome::AuthFailure)));
    }

    #[tokio::test]
    async fn whitespace_key_returns_auth_failure() {
        let result = validate_anthropic_key("   ".to_string()).await;
        assert!(matches!(result, Ok(ValidationOutcome::AuthFailure)));
    }
}
