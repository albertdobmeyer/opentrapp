//! Streaming command output — the Tauri binding over `core::stream` (de-Tauri, ADR-0022).
//!
//! The process spawning, line reading, single-quote-escaped arg interpolation, and active-stream
//! bookkeeping all live in `opentrapp_core::stream` now (one core, shared with the loopback
//! viewer-server). `core::stream` emits `stream-line` / `stream-end` to the shared
//! `AppState.event_bus`; the forwarder spawned in `lib.rs` setup re-emits each event via
//! `AppHandle::emit`, so the webview's `listen()` hooks are unchanged. These commands are now thin
//! shims with no duplicated logic.

use std::collections::HashMap;

use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::state::AppState;

#[tauri::command]
pub async fn start_stream(
    state: State<'_, AppState>,
    component_id: String,
    command_id: String,
    args: HashMap<String, String>,
) -> Result<(), OrchestratorError> {
    // Clone the component cache out of the mutex — never hold the guard across the await in core.
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::stream::start_stream(
        &components,
        &state.active_streams,
        &state.event_bus,
        component_id,
        command_id,
        &args,
    )
    .await
}

#[tauri::command]
pub async fn stop_stream(
    state: State<'_, AppState>,
    component_id: String,
    command_id: String,
) -> Result<(), OrchestratorError> {
    opentrapp_core::stream::stop_stream(&state.active_streams, &component_id, &command_id)
}
