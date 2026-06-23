//! Workflow commands — thin Tauri shims over `opentrapp_core::workflow_ops`.
//!
//! The lookup + engine hand-off were lifted into `opentrapp-core` (ADR-0022 migration step 1) so
//! the same transport-neutral fns back both this shim and the future web route. Each shim clones
//! the component cache out of the `AppState` mutex (never holding the guard across an await) and
//! delegates. Command names + signatures unchanged (registration in `lib.rs` is intact).

use std::collections::HashMap;

use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::manifest::Workflow;
use crate::orchestrator::state::AppState;
use crate::orchestrator::workflow::WorkflowResult;

/// List all workflows for a component.
#[tauri::command]
pub async fn list_workflows(
    state: State<'_, AppState>,
    component_id: String,
) -> Result<Vec<Workflow>, OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::workflow_ops::list_workflows(&components, component_id)
}

/// Execute a workflow by ID.
#[tauri::command]
pub async fn execute_workflow(
    state: State<'_, AppState>,
    component_id: String,
    workflow_id: String,
    inputs: HashMap<String, String>,
) -> Result<WorkflowResult, OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::workflow_ops::execute_workflow(&components, component_id, workflow_id, &inputs)
        .await
}
