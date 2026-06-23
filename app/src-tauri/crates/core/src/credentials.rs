//! Credential validation + the runtime `.env` writer (lifted from the Tauri command layer —
//! ADR-0022 migration step 1). The security-load-bearing parts move into the Tauri-free core so
//! the same transport-neutral fns back both the Tauri shim and the future loopback web route:
//!
//!   - `validate_anthropic_key` — host-side pre-flight ping of Anthropic; any network-error string
//!     is run through `redact_secrets` so a key can never leak into a surfaced error.
//!   - `write_credentials_at` — upserts the (non-empty) keys into the runtime `.env`, preserving
//!     every other line, then tightens the file to **0600** on unix (the #75 mandate: secrets stay
//!     off the process table AND off other users' read access).
//!
//! What deliberately STAYS in the GUI shim: `commit_activation` (AppHandle + `PerimeterStateStore`
//! + lifecycle markers + perimeter restart) and the `AppHandle`→`runtime_data_dir` path resolution.
//! The shim resolves the `.env` path, then hands the write to `write_credentials_at` here.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::util::secrets::redact_secrets;

/// Structured outcome of a key validation attempt. Lets the frontend show exact guidance per error
/// class without parsing error strings.
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

/// Live-pings Anthropic's `/v1/messages` with `max_tokens: 1` to verify the key is accepted. Uses
/// the cheapest current model for minimal cost. Returns a network error string only on complete
/// network failure — all API-level responses (401, 402, …) come back as `Ok(outcome)` so the
/// caller can give specific guidance. Any error string is redacted first.
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
        .map_err(|e| redact_secrets(&e.to_string()))?;

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| redact_secrets(&e.to_string()))?;

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

/// Replace an existing `KEY=...` line in a `.env` body, or append it. Preserves every other line
/// (other vars, comments). Pure. Mirrors the frontend `upsertEnvVar`.
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

/// Upsert the (non-empty) keys into the `.env` at `env_path`, preserving other content; creates the
/// parent dir and tightens the file to **0600**. Testable with a temp path.
pub fn write_credentials_at(
    env_path: &Path,
    anthropic_key: &str,
    telegram_token: &str,
) -> std::io::Result<()> {
    let mut content = std::fs::read_to_string(env_path)
        .unwrap_or_else(|_| "# OpenTrApp configuration\n".to_string());
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

    /// The writer must never clobber unrelated `.env` content when persisting keys (other API
    /// keys, comments, blank structure stay intact).
    #[test]
    fn write_credentials_preserves_unrelated_lines() {
        let dir = std::env::temp_dir().join(format!("oc-cred-preserve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let env_path = dir.join(".env");
        std::fs::write(&env_path, "# my notes\nOTHER_API_KEY=untouched\n").unwrap();

        write_credentials_at(&env_path, "sk-ant-zzz", "").unwrap();
        let content = std::fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("# my notes"), "comment preserved");
        assert!(content.contains("OTHER_API_KEY=untouched"), "unrelated key preserved");
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-zzz"), "new key written");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
