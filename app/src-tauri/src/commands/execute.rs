//! Command-execution commands — thin Tauri shims over `opentrapp_core::execute`.
//!
//! The lookup + on-demand-shield start + the run were lifted into `opentrapp-core` (ADR-0022
//! migration step 1). The GUI-side idle-stop task bookkeeping (the `AppState` `AbortHandle` map)
//! stays HERE — the lifted `run_command` returns which on-demand service it started, so this
//! shim arms the idle-stop. Behavior preserved exactly: a lookup error returns early (no
//! idle-stop); the idle-stop is armed after the run regardless of the command's success/failure.
//! Command names + signatures unchanged (registration in `lib.rs` is intact).

use std::collections::HashMap;

use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::podman;
use crate::orchestrator::runner::CommandResult;
use crate::orchestrator::state::AppState;

/// How long an on-demand shield stays warm after its last command before the idle-stop timer
/// reclaims it. Long enough that a multi-step workflow (several commands back-to-back) never
/// cold-starts the container twice.
const IDLE_GRACE_SECS: u64 = 300;

/// (Re)arm the idle-stop timer for an on-demand service: cancel any pending stop and schedule a
/// fresh one `IDLE_GRACE_SECS` out. Bursts keep the container warm; it is stopped once that long
/// passes with no further command. GUI-side spawned-task bookkeeping — stays in the shim.
fn arm_idle_stop(state: &AppState, svc: String) {
    let mut map = state.idle_stops.lock().unwrap();
    if let Some(prev) = map.remove(&svc) {
        prev.abort();
    }
    let svc_task = svc.clone();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(IDLE_GRACE_SECS)).await;
        let s = svc_task.clone();
        let _ = tokio::task::spawn_blocking(move || podman::service_down(&s)).await;
    })
    .abort_handle();
    map.insert(svc, handle);
}

#[tauri::command]
pub async fn run_command(
    state: State<'_, AppState>,
    component_id: String,
    command_id: String,
    args: HashMap<String, String>,
) -> Result<CommandResult, OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    let data_dir = state.runtime_data_dir.read().unwrap().clone();

    let outcome = opentrapp_core::execute::run_command(
        &components,
        &data_dir,
        component_id,
        command_id,
        &args,
    )
    .await?; // lookup error (component/command not found) returns early — no idle-stop armed

    // Arm the on-demand shield's idle-stop after the run, regardless of the command's result.
    if let Some(svc) = outcome.on_demand_service {
        arm_idle_stop(state.inner(), svc);
    }
    outcome.result
}

#[tauri::command]
pub async fn load_options(
    state: State<'_, AppState>,
    component_id: String,
    command_string: String,
    timeout_seconds: u64,
) -> Result<Vec<String>, OrchestratorError> {
    let components = { state.components.lock().unwrap().clone() };
    opentrapp_core::execute::load_options(&components, component_id, command_string, timeout_seconds)
        .await
}
