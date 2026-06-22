//! Health-probe execution (lifted from the Tauri command layer — ADR-0022 migration step 1).
//!
//! The first `AppState`-coupled lift: the GUI handler used `AppState` only to resolve a
//! component's directory from the discovered-components cache, then ran the probe via
//! `orchestrator::runner` (already core). The transport-neutral fn takes the components slice
//! as a parameter (the caller — Tauri shim or web GUI route — passes its own cache), so the
//! lookup + run live in core once, not duplicated per transport (CLAUDE.md §5).

use std::path::PathBuf;

use crate::orchestrator::discovery::DiscoveredComponent;
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::runner;

#[derive(Debug, serde::Serialize)]
pub struct HealthResult {
    pub probe_id: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run a health probe for `component_id` against its component directory (resolved from
/// `components`). `ComponentNotFound` if the id is not in the slice.
pub async fn run_health_probe(
    components: &[DiscoveredComponent],
    component_id: String,
    probe_command: String,
    timeout_seconds: u64,
) -> Result<HealthResult, OrchestratorError> {
    let component_dir = components
        .iter()
        .find(|c| c.manifest.identity.id == component_id)
        .map(|c| c.component_dir.clone())
        .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;

    let result =
        runner::run_shell(&probe_command, &PathBuf::from(&component_dir), timeout_seconds).await?;

    Ok(HealthResult {
        probe_id: component_id,
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_component_is_not_found() {
        // Resolves before any shell run, so the empty-slice case is deterministic + offline.
        let err = run_health_probe(&[], "nope".to_string(), "echo hi".to_string(), 5)
            .await
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }
}
