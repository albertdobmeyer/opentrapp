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

/// Build the `podman exec` argv for an interpolated command. Pure + unit-tested:
/// the interpolated string (already single-quote escaped by `interpolate_args`)
/// is ONE argv element after `bash -c`, so the host `podman` invocation never
/// word-splits or re-interprets it.
fn podman_exec_args(container: &str, interpolated: &str) -> Vec<String> {
    vec![
        "exec".into(),
        container.into(),
        "bash".into(),
        "-c".into(),
        interpolated.into(),
    ]
}

/// Run a manifest command INSIDE its backing container via `podman exec`.
///
/// Used for the on-demand workload shields (vault-skills/vault-social): their
/// commands (`make scan`, …) process UNTRUSTED downloaded content and MUST run in
/// the container, not on the host (CLAUDE.md §9 — "no untrusted content on the
/// host; all skill scanning happens inside containers"). The command runs in the
/// image's WORKDIR (`/app`, where the Makefile + tools live). `interpolate_args`
/// applies the same single-quote escaping as the host path; podman passes the
/// interpolated string as one argv to the container's bash, so the HOST shell
/// never re-interprets it (no host-side injection).
pub async fn run_command_in_container(
    container: &str,
    manifest_cmd: &ManifestCommand,
    args: &HashMap<String, String>,
    timeout_secs: u64,
) -> Result<CommandResult, OrchestratorError> {
    let interpolated = interpolate_args(&manifest_cmd.command, args);
    let start = Instant::now();

    let mut cmd = TokioCommand::new("podman");
    cmd.args(podman_exec_args(container, &interpolated));
    // Strip the AppImage's bundled-lib env so system podman/conmon don't load the
    // wrong glib (same reason as `orchestrator::podman`'s process helpers).
    for var in crate::orchestrator::podman::APPIMAGE_LIB_ENV {
        cmd.env_remove(var);
    }

    let output = if timeout_secs > 0 {
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output())
            .await
            .map_err(|_| OrchestratorError::Timeout(timeout_secs))?
            .map_err(OrchestratorError::IoError)?
    } else {
        cmd.output().await.map_err(OrchestratorError::IoError)?
    };

    Ok(CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: start.elapsed().as_millis() as u64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn podman_exec_keeps_escaped_arg_as_single_argv() {
        // An injection attempt in an arg value must stay a single shell-escaped
        // token inside ONE argv element — never split into separate podman args
        // and never re-interpreted by the host shell (CLAUDE.md §9).
        let mut args = HashMap::new();
        args.insert("skill".to_string(), "evil'; rm -rf / #".to_string());
        let interpolated = interpolate_args("make scan SKILL=${skill}", &args);
        let argv = podman_exec_args("vault-skills", &interpolated);
        assert_eq!(argv[0], "exec");
        assert_eq!(argv[1], "vault-skills");
        assert_eq!(argv[2], "bash");
        assert_eq!(argv[3], "-c");
        assert_eq!(argv.len(), 5, "the whole command is one argv element");
        assert!(argv[4].starts_with("make scan SKILL="));
        // The single quote in the value is escaped ('\'' ), neutralizing the
        // injection: `rm -rf /` can never break out of the quoted literal.
        assert!(argv[4].contains("'\\''"));
        assert!(!argv.iter().any(|a| a == "rm"));
    }
}
