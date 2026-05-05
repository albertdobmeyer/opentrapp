use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tokio::process::Command as TokioCommand;

use super::error::OrchestratorError;
use super::manifest::Command as ManifestCommand;
use crate::util::shell::find_bash;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Test-accessible version of interpolate_args. Also exposed under the
/// `fuzzing` feature so the `fuzz_api::interpolate_args` wrapper in lib.rs
/// can drive the function from the cargo-fuzz harness.
#[cfg(any(test, feature = "fuzzing"))]
pub fn interpolate_args_for_test(command: &str, args: &HashMap<String, String>) -> String {
    interpolate_args(command, args)
}

/// Interpolate ${arg_id} placeholders in a command string.
/// Values are wrapped in single quotes with proper escaping to prevent injection.
fn interpolate_args(command: &str, args: &HashMap<String, String>) -> String {
    let mut result = command.to_string();
    for (key, value) in args {
        // Wrap in single quotes with proper escaping: foo'bar -> 'foo'\''bar'
        let safe_value = format!("'{}'", value.replace('\'', "'\\''"));
        result = result.replace(&format!("${{{}}}", key), &safe_value);
    }
    result
}

/// Run a command from a manifest in a component directory
pub async fn run_command(
    manifest_cmd: &ManifestCommand,
    component_dir: &Path,
    args: &HashMap<String, String>,
    timeout_secs: u64,
) -> Result<CommandResult, OrchestratorError> {
    let bash = find_bash()
        .ok_or_else(|| OrchestratorError::ShellNotFound(
            "bash not found. Install Git for Windows or add bash to PATH.".to_string()
        ))?;

    let interpolated = interpolate_args(&manifest_cmd.command, args);
    let start = Instant::now();

    let output = if timeout_secs > 0 {
        tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            TokioCommand::new(&bash)
                .arg("-c")
                .arg(&interpolated)
                .current_dir(component_dir)
                .output(),
        )
        .await
        .map_err(|_| OrchestratorError::Timeout(timeout_secs))?
        .map_err(OrchestratorError::IoError)?
    } else {
        TokioCommand::new(&bash)
            .arg("-c")
            .arg(&interpolated)
            .current_dir(component_dir)
            .output()
            .await
            .map_err(OrchestratorError::IoError)?
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms,
    })
}

/// Run a raw shell command (for probes and options_from)
pub async fn run_shell(
    command: &str,
    working_dir: &Path,
    timeout_secs: u64,
) -> Result<CommandResult, OrchestratorError> {
    let bash = find_bash()
        .ok_or_else(|| OrchestratorError::ShellNotFound(
            "bash not found".to_string()
        ))?;

    let start = Instant::now();

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        TokioCommand::new(&bash)
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .output(),
    )
    .await
    .map_err(|_| OrchestratorError::Timeout(timeout_secs))?
    .map_err(OrchestratorError::IoError)?;

    Ok(CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
