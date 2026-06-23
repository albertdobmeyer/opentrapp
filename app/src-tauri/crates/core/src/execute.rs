//! Command execution (lifted from the Tauri command layer — ADR-0022 migration step 1).
//!
//! `run_command` is the first lift that touches the perimeter lifecycle, so the split is drawn
//! with care:
//!   - CORE owns the transport-neutral work: resolve the manifest command + dir, start the
//!     on-demand shield container if the command targets one, run the command (inside the
//!     container for on-demand shields, on the host otherwise).
//!   - The CALLER keeps the GUI-side idle-stop task bookkeeping (a `HashMap<service,
//!     AbortHandle>` on `AppState`): `run_command` RETURNS which on-demand service it started
//!     (`RunOutcome.on_demand_service`) so the caller arms the idle-stop. This preserves the
//!     exact original behavior — a lookup error returns early (no idle-stop), and the idle-stop
//!     is armed after the run regardless of the command's success/failure.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::orchestrator::discovery::DiscoveredComponent;
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::runner::{self, CommandResult};
use crate::orchestrator::{perimeter, podman};

/// Map a component id to its backing on-demand perimeter service, if any. Generic by
/// construction: `Some` only when the signed spec has a `vault-<id>` service flagged
/// `on_demand` (reacts to the spec, never a hardcoded name — the generic-backend rule).
pub fn on_demand_service_for(component_id: &str) -> Option<String> {
    let svc = format!("vault-{component_id}");
    let spec = perimeter::load().ok()?;
    spec.services.get(&svc).filter(|s| s.on_demand).map(|_| svc)
}

/// The result of running a manifest command plus the on-demand service that was started (if
/// any), so the caller can arm that service's idle-stop. `result` is the command's own Result
/// (run success/failure); the lookup errors are the *outer* `Err`.
pub struct RunOutcome {
    pub result: Result<CommandResult, OrchestratorError>,
    pub on_demand_service: Option<String>,
}

/// Run a manifest command for `component_id`/`command_id`. Resolves the command + dir from
/// `components`, starts the backing on-demand shield if any (via `runtime_data_dir`), then runs
/// the command inside that container (on-demand shields) or on the host (orchestrator commands).
pub async fn run_command(
    components: &[DiscoveredComponent],
    runtime_data_dir: &Path,
    component_id: String,
    command_id: String,
    args: &HashMap<String, String>,
) -> Result<RunOutcome, OrchestratorError> {
    let (manifest_cmd, component_dir) = {
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

    // On-demand shields (vault-skills / vault-social) are not booted with the perimeter. If this
    // command targets one, start its container first (a start failure falls through to the
    // existing graceful-degradation path: the exec fails cleanly → "unavailable").
    let on_demand = on_demand_service_for(&component_id);
    if let Some(svc) = &on_demand {
        let dd = runtime_data_dir.to_path_buf();
        let svc = svc.clone();
        let _ = tokio::task::spawn_blocking(move || podman::service_up(&dd, &svc, false)).await;
    }

    // Containerized execution (CLAUDE.md §9): on-demand shields run their commands INSIDE the
    // now-started container via `podman exec` (untrusted content never touches the host); other
    // components fall back to host bash in the component dir.
    let result = match &on_demand {
        Some(svc) => {
            runner::run_command_in_container(svc, &manifest_cmd, args, manifest_cmd.timeout_seconds)
                .await
        }
        None => {
            runner::run_command(
                &manifest_cmd,
                &PathBuf::from(&component_dir),
                args,
                manifest_cmd.timeout_seconds,
            )
            .await
        }
    };

    Ok(RunOutcome { result, on_demand_service: on_demand })
}

/// Run a `options_from` command and return its non-empty, trimmed stdout lines (the manifest
/// dynamic-options path). Component dir resolved from `components`.
pub async fn load_options(
    components: &[DiscoveredComponent],
    component_id: String,
    command_string: String,
    timeout_seconds: u64,
) -> Result<Vec<String>, OrchestratorError> {
    let component_dir = components
        .iter()
        .find(|c| c.manifest.identity.id == component_id)
        .map(|c| c.component_dir.clone())
        .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;

    let result =
        runner::run_shell(&command_string, &PathBuf::from(&component_dir), timeout_seconds).await?;

    Ok(result
        .stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_command_unknown_component_is_not_found() {
        let args = HashMap::new();
        let r =
            run_command(&[], Path::new("/tmp"), "nope".to_string(), "cmd".to_string(), &args).await;
        assert!(matches!(r, Err(OrchestratorError::ComponentNotFound(_))));
    }

    #[tokio::test]
    async fn load_options_unknown_component_is_not_found() {
        let r = load_options(&[], "nope".to_string(), "echo hi".to_string(), 5).await;
        assert!(matches!(r, Err(OrchestratorError::ComponentNotFound(_))));
    }
}
