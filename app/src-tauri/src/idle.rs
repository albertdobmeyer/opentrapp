//! GUI-side waker glue (Phase B, ADR-0019).
//!
//! The waker MECHANISM lives in `opentrapp_core::idle` (tauri-free). This shim is
//! the only part that touches Tauri: `spawn_waker` resolves `AppState` from the
//! `AppHandle`, replaces any stored waker (cancelling the old one), and stores
//! the new core waker. `read_telegram_token` + `stop_waker` are re-exported
//! unchanged so existing `crate::idle::…` call sites keep working.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use opentrapp_core::idle;
use opentrapp_core::orchestrator::state::AppState;

pub use opentrapp_core::idle::{read_telegram_token, stop_waker};

/// Spawn/replace the dormant-perimeter waker, storing it in `AppState`. No-op
/// when no bot token is configured (core logs that case). Replaces and cancels
/// any previously-stored waker.
pub fn spawn_waker(app: AppHandle, data_dir: PathBuf) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if let Ok(mut guard) = state.waker.lock() {
        if let Some(old) = guard.take() {
            old.cancel();
        }
        *guard = idle::spawn(data_dir);
    }
}
