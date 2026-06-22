//! Telegram API calls (lifted from the Tauri command layer — ADR-0022 migration step 1 — so
//! both the Tauri shim AND the loopback web GUI route call the same transport-neutral fns).
//!
//! Bot identity, activation handoff, and the poll-advance sequence that cleanly hands the
//! getUpdates offset to vault-agent. **All Telegram API calls are made from Rust** so the bot
//! token never leaks into the webview / browser and never hits the `connect-src` CSP.

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct TelegramResponse {
    ok: bool,
    result: Option<BotUser>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct BotUser {
    username: Option<String>,
}

/// Resolved Telegram bot identity surfaced to the frontend. Both fields are always populated
/// together (derived from the same `getMe` call): the Ready screen can deep-link into the bot
/// chat (`url`) or surface the `@username` for manual search.
#[derive(Serialize)]
pub struct TelegramBot {
    pub url: String,
    pub username: String,
}

/// Resolve a bot token into `{url, username}` via Telegram's `getMe`. `Err` on any failure
/// (the frontend falls back to a generic link; no error is shown). Never returns the token.
pub async fn derive_telegram_bot_url(token: String) -> Result<TelegramBot, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Empty token".to_string());
    }

    let url = format!("https://api.telegram.org/bot{}/getMe", token);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let resp = client.get(&url).send().await.map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Telegram API returned {}", resp.status()));
    }

    let body: TelegramResponse =
        resp.json().await.map_err(|e| format!("Malformed response: {}", e))?;

    if !body.ok {
        return Err(body.description.unwrap_or_else(|| "Telegram rejected the token".to_string()));
    }

    let username = body
        .result
        .and_then(|u| u.username)
        .ok_or_else(|| "Response missing username".to_string())?;

    if username.is_empty() {
        return Err("Empty username".to_string());
    }

    Ok(TelegramBot { url: format!("https://t.me/{}?text=Hi", username), username })
}

// ─── Activation handoff helpers ───────────────────────────────────────────

/// A single Telegram update carrying a /start message. `chat_id` sends the test message;
/// `update_id` advances the server-side offset so vault-agent won't re-process /start.
#[derive(Serialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub chat_id: i64,
}

#[derive(Deserialize)]
struct TelegramUpdatesResponse {
    ok: bool,
    result: Option<Vec<RawUpdate>>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct RawUpdate {
    update_id: i64,
    message: Option<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    chat: RawChat,
    text: Option<String>,
}

#[derive(Deserialize)]
struct RawChat {
    id: i64,
}

/// Clear any leftover webhook so the subsequent getUpdates long-poll works. Idempotent.
pub async fn telegram_delete_webhook(token: String) -> Result<(), String> {
    let token = token.trim().to_string();
    let url =
        format!("https://api.telegram.org/bot{token}/deleteWebhook?drop_pending_updates=false");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let _ = client.post(&url).send().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Long-poll Telegram for the first /start at or after `offset`. `Some(update)` when found,
/// `None` on timeout. The reqwest deadline is 10s past Telegram's server-side `timeout`.
pub async fn telegram_poll_for_start(
    token: String,
    offset: i64,
    timeout_secs: u32,
) -> Result<Option<TelegramUpdate>, String> {
    let token = token.trim().to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs as u64 + 10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("https://api.telegram.org/bot{token}/getUpdates"))
        .query(&[("offset", offset.to_string()), ("timeout", timeout_secs.to_string())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 409 {
        return Err("conflict".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("Telegram API returned {}", resp.status()));
    }

    let body: TelegramUpdatesResponse = resp.json().await.map_err(|e| e.to_string())?;
    if !body.ok {
        return Err(body.description.unwrap_or_else(|| "Telegram API error".to_string()));
    }

    for update in body.result.unwrap_or_default() {
        if let Some(msg) = update.message {
            let text = msg.text.unwrap_or_default();
            if text.trim_start().starts_with("/start") {
                return Ok(Some(TelegramUpdate { update_id: update.update_id, chat_id: msg.chat.id }));
            }
        }
    }

    Ok(None)
}

/// Send the activation test message to `chat_id`. `Err("conflict")` on HTTP 409.
pub async fn telegram_send_message(token: String, chat_id: i64, text: String) -> Result<(), String> {
    let token = token.trim().to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({ "chat_id": chat_id, "text": text });
    let resp = client
        .post(format!("https://api.telegram.org/bot{token}/sendMessage"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 409 {
        return Err("conflict".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("Telegram API returned {}", resp.status()));
    }
    Ok(())
}

/// Advance the server-side getUpdates offset past `update_id` so vault-agent won't re-process
/// /start on its first poll. `timeout=0` = instant return. Return value is ignored.
pub async fn telegram_advance_offset(token: String, update_id: i64) -> Result<(), String> {
    let token = token.trim().to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let _ = client
        .get(format!("https://api.telegram.org/bot{token}/getUpdates"))
        .query(&[("offset", (update_id + 1).to_string()), ("timeout", "0".to_string())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_token_errors() {
        assert!(derive_telegram_bot_url(String::new()).await.is_err());
    }

    #[tokio::test]
    async fn whitespace_token_errors() {
        assert!(derive_telegram_bot_url("   ".to_string()).await.is_err());
    }

    #[test]
    fn response_parses_happy_path() {
        let json = r#"{"ok":true,"result":{"id":1,"is_bot":true,"first_name":"Bot","username":"MyAssistantBot"}}"#;
        let parsed: TelegramResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.result.and_then(|u| u.username).as_deref(), Some("MyAssistantBot"));
    }

    #[test]
    fn response_parses_error_path() {
        let json = r#"{"ok":false,"description":"Unauthorized"}"#;
        let parsed: TelegramResponse = serde_json::from_str(json).unwrap();
        assert!(!parsed.ok);
        assert_eq!(parsed.description.as_deref(), Some("Unauthorized"));
    }
}
