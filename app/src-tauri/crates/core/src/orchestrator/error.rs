use serde::Serialize;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum OrchestratorError {
    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Command not found: {command} in component {component}")]
    CommandNotFound { component: String, command: String },

    #[error("Workflow not found: {workflow} in component {component}")]
    WorkflowNotFound { component: String, workflow: String },

    #[error("Workflow step failed: {step} in workflow {workflow}: {message}")]
    WorkflowStepFailed { workflow: String, step: String, message: String },

    #[error("Manifest parse error in {path}: {message}")]
    ManifestParseError { path: String, message: String },

    #[error("Command execution failed: {0}")]
    ExecutionError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Config file not found: {0}")]
    ConfigNotFound(String),

    #[error("Config write error: {0}")]
    ConfigWriteError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Shell not found: {0}")]
    ShellNotFound(String),

    #[error("Command timed out after {0} seconds")]
    Timeout(u64),
}

impl Serialize for OrchestratorError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
