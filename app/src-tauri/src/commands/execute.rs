use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::runner::{self, CommandResult};
use crate::orchestrator::state::AppState;
use crate::orchestrator::{perimeter, podman};

/// How long an on-demand shield stays warm after its last command before the
/// idle-stop timer reclaims it. Long enough that a multi-step workflow (several
/// commands back-to-back) never cold-starts the container twice.
const IDLE_GRACE_SECS: u64 = 300;

/// Map a component id to its backing on-demand perimeter service, if any.
/// Generic by construction: returns `Some` only when the signed spec has a
/// `vault-<id>` service flagged `on_demand`, so it reacts to the spec, never to
/// a hardcoded component name (the generic-backend rule).
fn on_demand_service_for(component_id: &str) -> Option<String> {
    let svc = format!("vault-{component_id}");
    let spec = perimeter::load().ok()?;
    spec.services.get(&svc).filter(|s| s.on_demand).map(|_| svc)
}

/// (Re)arm the idle-stop timer for an on-demand service: cancel any pending stop
/// and schedule a fresh one `IDLE_GRACE_SECS` out. Bursts keep the container
/// warm; it is stopped once that long passes with no further command.
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
    let (manifest_cmd, component_dir) = {
        let components = state.components.lock().unwrap();
        let component = components
            .iter()
            .find(|c| c.manifest.identity.id == component_id)
            .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;

        let cmd = component
            .manifest
            .commands
            .iter()
            .find(|c| c.id == command_id)
            .ok_or_else(|| OrchestratorError::CommandNotFound {
                component: component_id.clone(),
                command: command_id.clone(),
            })?
            .clone();

        (cmd, component.component_dir.clone())
    };

    // On-demand shields (vault-skills/vault-social) are not booted with the
    // perimeter. If this command targets one, start its container first, then
    // keep it warm for a short grace afterwards so a burst of commands does not
    // thrash. A start failure falls through to the existing graceful-degradation
    // path (the command runs and reports "unavailable").
    let on_demand = on_demand_service_for(&component_id);
    if let Some(svc) = on_demand.clone() {
        let data_dir = state.runtime_data_dir.read().unwrap().clone();
        let _ = tokio::task::spawn_blocking(move || podman::service_up(&data_dir, &svc, false)).await;
    }

    let result = runner::run_command(
        &manifest_cmd,
        &PathBuf::from(&component_dir),
        &args,
        manifest_cmd.timeout_seconds,
    )
    .await;

    if let Some(svc) = on_demand {
        arm_idle_stop(state.inner(), svc);
    }

    result
}

#[tauri::command]
pub async fn load_options(
    state: State<'_, AppState>,
    component_id: String,
    command_string: String,
    timeout_seconds: u64,
) -> Result<Vec<String>, OrchestratorError> {
    let component_dir = {
        let components = state.components.lock().unwrap();
        let component = components
            .iter()
            .find(|c| c.manifest.identity.id == component_id)
            .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;
        component.component_dir.clone()
    };

    let result = runner::run_shell(
        &command_string,
        &PathBuf::from(&component_dir),
        timeout_seconds,
    )
    .await?;

    Ok(result
        .stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}
