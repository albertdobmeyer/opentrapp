use std::collections::HashMap;
use std::path::PathBuf;

use regex::Regex;

use super::error::OrchestratorError;
use super::manifest::{SuccessCondition, Workflow, WorkflowStep};
use super::runner::{self, CommandResult};

/// Result of a single workflow step execution
#[derive(Debug, Clone, serde::Serialize)]
pub struct StepResult {
    pub step_id: String,
    pub command_id: String,
    pub status: StepStatus,
    pub result: Option<CommandResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

/// Result of a complete workflow execution
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowResult {
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub steps: Vec<StepResult>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Running,
    Completed,
    Failed,
    Aborted,
}

/// Interpolate workflow-level template variables in step args.
///
/// Supports:
///   {{input.var_name}} — replaced with user-provided input values
///   {{steps.step_id.stdout}} — replaced with a previous step's stdout
fn interpolate_workflow_args(
    step_args: &HashMap<String, String>,
    inputs: &HashMap<String, String>,
    step_outputs: &HashMap<String, CommandResult>,
) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (key, template) in step_args {
        let mut value = template.clone();

        // Replace {{input.x}} with user input values
        for (input_key, input_val) in inputs {
            let pattern = format!("{{{{input.{}}}}}", input_key);
            value = value.replace(&pattern, input_val);
        }

        // Replace {{steps.step_id.stdout}} with previous step output
        for (step_id, step_result) in step_outputs {
            let pattern = format!("{{{{steps.{}.stdout}}}}", step_id);
            value = value.replace(&pattern, step_result.stdout.trim());

            let pattern = format!("{{{{steps.{}.output}}}}", step_id);
            value = value.replace(&pattern, step_result.stdout.trim());
        }

        result.insert(key.clone(), value);
    }
    result
}

/// Check if a step result meets the success condition
fn check_success(result: &CommandResult, condition: &Option<SuccessCondition>) -> bool {
    match condition {
        None => result.exit_code == 0,
        Some(cond) => {
            if let Some(code) = cond.exit_code {
                if result.exit_code != code {
                    return false;
                }
            }
            if let Some(ref contains) = cond.stdout_contains {
                if !result.stdout.contains(contains) {
                    return false;
                }
            }
            if let Some(ref regex_str) = cond.stdout_regex {
                match Regex::new(regex_str) {
                    Ok(re) => {
                        if !re.is_match(&result.stdout) {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
            }
            true
        }
    }
}

/// Determine step execution order based on depends_on.
/// Returns step indices in execution order.
fn resolve_step_order(steps: &[WorkflowStep]) -> Vec<usize> {
    // Simple topological sort: steps without depends_on go first,
    // then steps that depend on already-resolved steps.
    let mut order: Vec<usize> = Vec::new();
    let mut resolved: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Multiple passes to resolve dependencies
    let max_iterations = steps.len() + 1;
    for _ in 0..max_iterations {
        if order.len() == steps.len() {
            break;
        }
        for (i, step) in steps.iter().enumerate() {
            if order.contains(&i) {
                continue;
            }
            match &step.depends_on {
                None => {
                    // No dependency — check if previous step (by index) is resolved
                    // or if this is the first step
                    if i == 0 || resolved.contains(&steps[i - 1].id) {
                        order.push(i);
                        resolved.insert(step.id.clone());
                    }
                }
                Some(dep) => {
                    if resolved.contains(dep) {
                        order.push(i);
                        resolved.insert(step.id.clone());
                    }
                }
            }
        }
    }

    // If any steps couldn't be resolved, append them at the end
    for i in 0..steps.len() {
        if !order.contains(&i) {
            order.push(i);
        }
    }

    order
}

/// Execute a workflow by running its steps sequentially.
///
/// Each step looks up the referenced command in the component's manifest,
/// interpolates args with user inputs and previous step outputs, runs it,
/// and checks the success condition.
pub async fn execute_workflow(
    workflow: &Workflow,
    commands: &[super::manifest::Command],
    component_dir: &PathBuf,
    inputs: &HashMap<String, String>,
) -> Result<WorkflowResult, OrchestratorError> {
    let start = std::time::Instant::now();
    let mut step_results: Vec<StepResult> = Vec::new();
    let mut step_outputs: HashMap<String, CommandResult> = HashMap::new();
    let mut workflow_failed = false;

    let step_order = resolve_step_order(&workflow.steps);

    for &step_idx in &step_order {
        let step = &workflow.steps[step_idx];

        if workflow_failed {
            step_results.push(StepResult {
                step_id: step.id.clone(),
                command_id: step.command.clone(),
                status: StepStatus::Skipped,
                result: None,
                error: None,
            });
            continue;
        }

        // Find the referenced command in the manifest
        let manifest_cmd = commands
            .iter()
            .find(|c| c.id == step.command)
            .ok_or_else(|| OrchestratorError::WorkflowStepFailed {
                workflow: workflow.id.clone(),
                step: step.id.clone(),
                message: format!("command '{}' not found in manifest", step.command),
            })?;

        // Interpolate workflow args with inputs and previous outputs
        let interpolated_args = interpolate_workflow_args(&step.args, inputs, &step_outputs);

        // Run the command
        let cmd_result = runner::run_command(
            manifest_cmd,
            component_dir,
            &interpolated_args,
            manifest_cmd.timeout_seconds,
        )
        .await;

        match cmd_result {
            Ok(result) => {
                let success = check_success(&result, &step.success_condition);
                let status = if success {
                    StepStatus::Passed
                } else {
                    StepStatus::Failed
                };

                step_outputs.insert(step.id.clone(), result.clone());

                step_results.push(StepResult {
                    step_id: step.id.clone(),
                    command_id: step.command.clone(),
                    status: status.clone(),
                    result: Some(result),
                    error: None,
                });

                if status == StepStatus::Failed && step.abort_on_failure {
                    workflow_failed = true;
                }
            }
            Err(e) => {
                step_results.push(StepResult {
                    step_id: step.id.clone(),
                    command_id: step.command.clone(),
                    status: StepStatus::Failed,
                    result: None,
                    error: Some(e.to_string()),
                });

                if step.abort_on_failure {
                    workflow_failed = true;
                }
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = if workflow_failed {
        WorkflowStatus::Failed
    } else {
        WorkflowStatus::Completed
    };

    Ok(WorkflowResult {
        workflow_id: workflow.id.clone(),
        status,
        steps: step_results,
        duration_ms,
    })
}
