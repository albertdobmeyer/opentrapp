//! Health-probe command — a thin Tauri shim over `opentrapp_core::health`.
//!
//! The probe execution + the component-directory lookup were lifted into `opentrapp-core`
//! (ADR-0022 migration step 1). This shim clones the discovered-components cache out of the
//! `AppState` mutex (the guard isn't held across the await) and delegates; the web GUI route
//! will pass its own cache to the same fn. Command name + signature unchanged (registration
//! intact).

use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::state::AppState;

pub use opentrapp_core::health::HealthResult;

#[tauri::command]
pub async fn run_health_probe(
    state: State<'_, AppState>,
    component_id: String,
    probe_command: String,
    timeout_seconds: u64,
) -> Result<HealthResult, OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::health::run_health_probe(&components, component_id, probe_command, timeout_seconds)
        .await
}
