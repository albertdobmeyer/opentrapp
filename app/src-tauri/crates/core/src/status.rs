//! Component status evaluation (lifted from the Tauri command layer — ADR-0022 migration step 1).
//!
//! Runs a component's declared status probes and matches their rules to a resolved state. Like
//! `health`, the components slice is a parameter (the caller passes its cache). The state-cache
//! write that the GUI handler did inline stays in the caller (a GUI-side optimization, not core
//! logic) — this fn just RETURNS the resolved state, behavior-equivalent.

use std::path::PathBuf;

use crate::orchestrator::discovery::DiscoveredComponent;
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::runner;

#[derive(Debug, serde::Serialize)]
pub struct ComponentStatus {
    pub component_id: String,
    pub state_id: String,
}

/// Evaluate `component_id`'s status: run each declared probe, match its rules in order, and
/// return the first matching state (or the declared default, or `"unknown"`). `ComponentNotFound`
/// if the id is not in `components`; `"unknown"` if the component declares no status config.
pub async fn evaluate_status(
    components: &[DiscoveredComponent],
    component_id: String,
) -> Result<ComponentStatus, OrchestratorError> {
    let (status_config, component_dir) = {
        let component = components
            .iter()
            .find(|c| c.manifest.identity.id == component_id)
            .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;
        (component.manifest.status.clone(), component.component_dir.clone())
    };

    let status = match status_config {
        Some(s) => s,
        None => return Ok(ComponentStatus { component_id, state_id: "unknown".to_string() }),
    };

    let dir = PathBuf::from(&component_dir);

    for probe in &status.probes {
        if let Ok(result) = runner::run_shell(&probe.command, &dir, probe.timeout_seconds).await {
            for rule in &probe.rules {
                let matches = match (&rule.exit_code, &rule.stdout_contains, &rule.stdout_regex) {
                    (Some(code), _, _) if result.exit_code == *code => true,
                    (_, Some(contains), _) if result.stdout.contains(contains.as_str()) => true,
                    (_, _, Some(pattern)) => regex_matches(&result.stdout, pattern),
                    _ => false,
                };
                if matches {
                    return Ok(ComponentStatus { component_id, state_id: rule.state.clone() });
                }
            }
        }
    }

    let default = status.default_state.unwrap_or_else(|| "unknown".to_string());
    Ok(ComponentStatus { component_id, state_id: default })
}

fn regex_matches(text: &str, pattern: &str) -> bool {
    match regex::Regex::new(pattern) {
        Ok(re) => re.is_match(text),
        Err(e) => {
            eprintln!("Invalid regex pattern '{}': {}", pattern, e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_component_is_not_found() {
        let err = evaluate_status(&[], "nope".to_string()).await.unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }
}
