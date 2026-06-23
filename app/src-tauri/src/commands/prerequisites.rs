//! First-run prerequisite commands — thin Tauri shims over `opentrapp_core::prerequisites`.
//!
//! The report assembly, the container-runtime probe, the submodule scan, and the traversal-guarded
//! template copy were lifted into `opentrapp-core` (ADR-0022 migration step 1), along with the
//! report types, so the same transport-neutral fns back both this shim and the future loopback web
//! route. Each shim reads the runtime data dir / clones the component cache out of `AppState` and
//! delegates. Command names + signatures + `lib.rs` registration unchanged.

use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::state::AppState;
use opentrapp_core::prerequisites::PrerequisiteReport;

#[tauri::command]
pub async fn check_prerequisites(
    state: State<'_, AppState>,
) -> Result<PrerequisiteReport, OrchestratorError> {
    let root = state.runtime_data_dir.read().unwrap().clone();
    opentrapp_core::prerequisites::check_prerequisites(&root).await
}

#[tauri::command]
pub async fn init_submodules(state: State<'_, AppState>) -> Result<String, OrchestratorError> {
    let root = state.runtime_data_dir.read().unwrap().clone();
    opentrapp_core::prerequisites::init_submodules(&root).await
}

#[tauri::command]
pub async fn create_config_from_template(
    state: State<'_, AppState>,
    component_id: String,
    config_path: String,
    template_path: String,
) -> Result<(), OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::prerequisites::create_config_from_template(
        &components,
        component_id,
        config_path,
        template_path,
    )
}
