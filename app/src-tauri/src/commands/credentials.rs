//! Anthropic API-key validation and activation-commit command.
//!
//! `validate_anthropic_key` — live-pings Anthropic directly from the host
//! (not via vault-proxy) as a pre-flight check. Post-activation, all real
//! agent traffic goes through the proxy.
//!
//! `commit_activation` — called by ActivationModal after both keys are
//! validated and written to .env. Restarts vault-proxy to pick up new keys,
//! brings vault-agent up, and writes the activated + credentials-ok markers.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager as _};
use serde::Serialize;

use crate::lifecycle::{
    write_activated_marker, write_credentials_ok_marker, PerimeterStateStore,
};
use crate::orchestrator::podman;
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
// The wizard previously wrote the keys via `write_config("agent", ".env")` —
// the generic component-config editor, which resolves into the *component*
// directory. On a packaged AppImage first-run that directory is the read-only
// bundle (the writable staged copy is only created later, inside the
// credentials-gated bootstrap), so the write failed and the wizard dead-ended.
// The runtime `.env` actually lives in the data dir (`~/.opentrapp/.env`) —
// where the bootstrap (`bootstrap::run_bootstrap` step_write_env) and the
// perimeter read it. These commands write/read it there directly, so first-run
// works on packaged builds, not just dev source trees.

fn runtime_env_path(handle: &AppHandle) -> std::path::PathBuf {
    let root = handle
        .try_state::<AppState>()
        .and_then(|s| s.runtime_data_dir.read().ok().map(|r| r.clone()))
        .unwrap_or_else(podman::runtime_data_dir);
    root.join(".env")
}

/// Replace an existing `KEY=...` line in a `.env` body, or append it. Preserves
/// every other line (other vars, comments). Pure — unit-tested below. Mirrors
/// the frontend `upsertEnvVar`.
fn upsert_env_var(content: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}=");
    let mut replaced = false;
    let mut out: Vec<String> = content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&prefix) {
                replaced = true;
                format!("{key}={value}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !replaced {
        out.push(format!("{key}={value}"));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// Upsert the (non-empty) keys into the `.env` at `env_path`, preserving other
/// content; creates the parent dir and tightens the file to 0600. Testable with
/// a temp path.
fn write_credentials_at(env_path: &Path, anthropic_key: &str, telegram_token: &str) -> std::io::Result<()> {
    let mut content =
        std::fs::read_to_string(env_path).unwrap_or_else(|_| "# OpenTrApp configuration\n".to_string());
    if !anthropic_key.is_empty() {
        content = upsert_env_var(&content, "ANTHROPIC_API_KEY", anthropic_key);
    }
    if !telegram_token.is_empty() {
        content = upsert_env_var(&content, "TELEGRAM_BOT_TOKEN", telegram_token);
    }
    if let Some(parent) = env_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(env_path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(env_path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Write the agent credentials to the runtime `.env` (`~/.opentrapp/.env`).
/// Only non-empty keys are upserted; existing vars are preserved. Replaces the
/// wizard's old `write_config("agent",".env")` path that failed on packaged
/// first-run.
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

    #[test]
    fn upsert_replaces_existing_preserves_others_and_appends_new() {
        let c = "# header\nANTHROPIC_API_KEY=old\nOPENAI_API_KEY=keep\n";
        let c = upsert_env_var(c, "ANTHROPIC_API_KEY", "new");
        assert!(c.contains("ANTHROPIC_API_KEY=new"));
        assert!(!c.contains("old"));
        assert!(c.contains("OPENAI_API_KEY=keep"));
        assert!(c.contains("# header"));
        // appends a key that isn't present
        let c = upsert_env_var(&c, "TELEGRAM_BOT_TOKEN", "123:abc");
        assert!(c.contains("TELEGRAM_BOT_TOKEN=123:abc"));
        assert!(c.ends_with('\n'));
    }

    #[test]
    fn write_credentials_creates_upserts_and_preserves() {
        let dir = std::env::temp_dir().join(format!("oc-cred-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let env_path = dir.join(".env");

        // First write into a non-existent path: creates dir + file with both keys.
        write_credentials_at(&env_path, "sk-ant-aaa", "999:tok").unwrap();
        let content = std::fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-aaa"));
        assert!(content.contains("TELEGRAM_BOT_TOKEN=999:tok"));

        // Second write with an empty telegram token: replaces anthropic, keeps telegram.
        write_credentials_at(&env_path, "sk-ant-bbb", "").unwrap();
        let content = std::fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-bbb"));
        assert!(!content.contains("sk-ant-aaa"));
        assert!(content.contains("TELEGRAM_BOT_TOKEN=999:tok"));

        // 0600 on unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&env_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
