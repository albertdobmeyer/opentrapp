//! Telegram commands — thin Tauri shims over `opentrapp_core::telegram`.
//!
//! The logic + types + tests were lifted into `opentrapp-core` (ADR-0022 migration step 1) so
//! the on-demand loopback web GUI route calls the exact same transport-neutral fns — no
//! duplicate. All Telegram API calls stay Rust-side, so the bot token never reaches the
//! webview / browser. The `#[tauri::command]` names + signatures are unchanged (registration
//! in `lib.rs` is intact).

pub use opentrapp_core::telegram::{TelegramBot, TelegramUpdate};

#[tauri::command]
pub async fn derive_telegram_bot_url(token: String) -> Result<TelegramBot, String> {
    opentrapp_core::telegram::derive_telegram_bot_url(token).await
}

#[tauri::command]
pub async fn telegram_delete_webhook(token: String) -> Result<(), String> {
    opentrapp_core::telegram::telegram_delete_webhook(token).await
}

#[tauri::command]
pub async fn telegram_poll_for_start(
    token: String,
    offset: i64,
    timeout_secs: u32,
) -> Result<Option<TelegramUpdate>, String> {
    opentrapp_core::telegram::telegram_poll_for_start(token, offset, timeout_secs).await
}

#[tauri::command]
pub async fn telegram_send_message(
    token: String,
    chat_id: i64,
    text: String,
) -> Result<(), String> {
    opentrapp_core::telegram::telegram_send_message(token, chat_id, text).await
}

#[tauri::command]
pub async fn telegram_advance_offset(token: String, update_id: i64) -> Result<(), String> {
    opentrapp_core::telegram::telegram_advance_offset(token, update_id).await
}
