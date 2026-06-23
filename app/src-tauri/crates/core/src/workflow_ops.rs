//! Workflow listing + execution (lifted from the Tauri command layer — ADR-0022 migration step 1).
//!
//! Thin command-layer over the orchestrator workflow ENGINE (`orchestrator::workflow`): resolve
//! the workflow + its component's commands + dir from the `components` slice, then hand off to the
//! engine. Like `health`/`status`, the components slice is a parameter (the caller passes its
//! cache) so the same transport-neutral fns serve both the Tauri shim and the future web route.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::orchestrator::discovery::DiscoveredComponent;
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::manifest::Workflow;
use crate::orchestrator::workflow::{self, WorkflowResult};

/// List all workflows declared by `component_id`. `ComponentNotFound` if the id is not in
/// `components`.
pub fn list_workflows(
    components: &[DiscoveredComponent],
    component_id: String,
) -> Result<Vec<Workflow>, OrchestratorError> {
    let component = components
        .iter()
        .find(|c| c.manifest.identity.id == component_id)
        .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;

    Ok(component.manifest.workflows.clone())
}

/// Execute `workflow_id` for `component_id`: resolve the workflow + the component's command set +
/// dir from `components`, then run it via the orchestrator workflow engine. `ComponentNotFound` /
/// `WorkflowNotFound` if either id does not resolve.
pub async fn execute_workflow(
    components: &[DiscoveredComponent],
    component_id: String,
    workflow_id: String,
    inputs: &HashMap<String, String>,
) -> Result<WorkflowResult, OrchestratorError> {
    let (wf, commands, component_dir) = {
        let component = components
            .iter()
            .find(|c| c.manifest.identity.id == component_id)
            .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;

        let wf = component
            .manifest
            .workflows
            .iter()
            .find(|w| w.id == workflow_id)
            .ok_or_else(|| OrchestratorError::WorkflowNotFound {
                component: component_id.clone(),
                workflow: workflow_id.clone(),
            })?
            .clone();

        (wf, component.manifest.commands.clone(), component.component_dir.clone())
    };

    workflow::execute_workflow(&wf, &commands, &PathBuf::from(&component_dir), inputs).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_workflows_unknown_component_is_not_found() {
        let err = list_workflows(&[], "nope".to_string()).unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }

    #[tokio::test]
    async fn execute_workflow_unknown_component_is_not_found() {
        let inputs = HashMap::new();
        let err = execute_workflow(&[], "nope".to_string(), "wf".to_string(), &inputs)
            .await
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }
}
