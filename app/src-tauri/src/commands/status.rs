//! Status command — a thin Tauri shim over `opentrapp_core::status`.
//!
//! The probe-running + rule-matching evaluation was lifted into `opentrapp-core` (ADR-0022
//! migration step 1). This shim clones the components cache out of the `AppState` mutex,
//! delegates the evaluation to core, then writes the resolved state into the GUI-side
//! `component_states` cache (a UI optimization, kept in the caller). Command name + signature
//! unchanged (registration intact).

use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::state::AppState;

pub use opentrapp_core::status::ComponentStatus;

#[tauri::command]
pub async fn get_status(
    state: State<'_, AppState>,
    component_id: String,
) -> Result<ComponentStatus, OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    let status = opentrapp_core::status::evaluate_status(&components, component_id.clone()).await?;
    // GUI-side cache of the resolved state (the lifted core fn stays cache-neutral).
    state.component_states.lock().unwrap().insert(component_id, status.state_id.clone());
    Ok(status)
}
