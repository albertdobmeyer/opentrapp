//! Config commands — thin Tauri shims over `opentrapp_core::config_ops`.
//!
//! The read/write + the CLAUDE.md §9 path-traversal containment were lifted into
//! `opentrapp-core` (ADR-0022 migration step 1) so the same transport-neutral, traversal-guarded
//! fns back both this shim and the future loopback web route. Each shim clones the component cache
//! out of the `AppState` mutex (never holding the guard across the call) and delegates. Command
//! names + signatures + `lib.rs` registration unchanged.

use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::state::AppState;

#[tauri::command]
pub async fn read_config(
    state: State<'_, AppState>,
    component_id: String,
    config_path: String,
) -> Result<String, OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::config_ops::read_config(&components, component_id, config_path)
}

#[tauri::command]
pub async fn write_config(
    state: State<'_, AppState>,
    component_id: String,
    config_path: String,
    content: String,
) -> Result<(), OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::config_ops::write_config(&components, component_id, config_path, content)
}
