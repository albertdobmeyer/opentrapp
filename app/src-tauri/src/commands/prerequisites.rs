use std::path::PathBuf;
use serde::Serialize;
use tauri::State;

use crate::orchestrator::discovery::discover_components;
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::runner::run_shell;
use crate::orchestrator::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ContainerRuntimeInfo {
    pub found: bool,
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmoduleInfo {
    pub id: String,
    pub name: String,
    pub cloned: bool,
    pub has_manifest: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentPrereqInfo {
    pub component_id: String,
    pub component_name: String,
    pub needs_container_runtime: bool,
    pub missing_config_files: Vec<MissingConfigFile>,
    pub check_passed: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MissingConfigFile {
    pub path: String,
    pub template: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrerequisiteReport {
    pub container_runtime: ContainerRuntimeInfo,
    pub submodules: Vec<SubmoduleInfo>,
    pub components: Vec<ComponentPrereqInfo>,
}

#[tauri::command]
pub async fn check_prerequisites(
    state: State<'_, AppState>,
) -> Result<PrerequisiteReport, OrchestratorError> {
    let root = state.runtime_data_dir.read().unwrap().clone();

    // Check container runtime
    let container_runtime = check_container_runtime(&root).await;

    // Check submodules
    let components_dir = root.join("components");
    let mut submodules = Vec::new();

    if components_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&components_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    let has_manifest = path.join("component.yml").exists();
                    submodules.push(SubmoduleInfo {
                        id: dir_name.clone(),
                        name: dir_name,
                        cloned: true,
                        has_manifest,
                    });
                }
            }
        }
    }

    // Check per-component prerequisites
    let discovered = discover_components(&root).unwrap_or_default();
    let mut component_prereqs = Vec::new();

    for dc in &discovered {
        let manifest = &dc.manifest;
        let prereqs = match &manifest.prerequisites {
            Some(p) => p,
            None => continue,
        };

        let mut missing_configs = Vec::new();
        for cf in &prereqs.config_files {
            let config_path = PathBuf::from(&dc.component_dir).join(&cf.path);
            if !config_path.exists() {
                missing_configs.push(MissingConfigFile {
                    path: cf.path.clone(),
                    template: cf.template.clone(),
                    description: cf.description.clone(),
                });
            }
        }

        let check_passed = if let Some(cmd) = &prereqs.check_command {
            let result = run_shell(cmd, &PathBuf::from(&dc.component_dir), 10).await;
            Some(result.map(|r| r.exit_code == 0).unwrap_or(false))
        } else {
            None
        };

        component_prereqs.push(ComponentPrereqInfo {
            component_id: manifest.identity.id.clone(),
            component_name: manifest.identity.name.clone(),
            needs_container_runtime: prereqs.container_runtime,
            missing_config_files: missing_configs,
            check_passed,
        });
    }

    Ok(PrerequisiteReport {
        container_runtime,
        submodules,
        components: component_prereqs,
    })
}

async fn check_container_runtime(root: &PathBuf) -> ContainerRuntimeInfo {
    // Try podman first, then docker
    for runtime in &["podman", "docker"] {
        let cmd = format!("{} --version", runtime);
        if let Ok(result) = run_shell(&cmd, root, 5).await {
            if result.exit_code == 0 {
                return ContainerRuntimeInfo {
                    found: true,
                    name: Some(runtime.to_string()),
                    version: Some(result.stdout.trim().to_string()),
                };
            }
        }
    }

    ContainerRuntimeInfo {
        found: false,
        name: None,
        version: None,
    }
}

#[tauri::command]
pub async fn init_submodules(
    state: State<'_, AppState>,
) -> Result<String, OrchestratorError> {
    let root = state.runtime_data_dir.read().unwrap().clone();

    let result = run_shell(
        "git submodule update --init --recursive",
        &root,
        120,
    )
    .await?;

    if result.exit_code != 0 {
        return Err(OrchestratorError::ExecutionError(format!(
            "git submodule update failed: {}",
            result.stderr
        )));
    }

    Ok(result.stdout)
}

#[tauri::command]
pub async fn create_config_from_template(
    state: State<'_, AppState>,
    component_id: String,
    config_path: String,
    template_path: String,
) -> Result<(), OrchestratorError> {
    let component_dir = {
        let components = state.components.lock().unwrap();
        let component = components
            .iter()
            .find(|c| c.manifest.identity.id == component_id)
            .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;
        component.component_dir.clone()
    };

    let base = PathBuf::from(&component_dir);
    let template_full = base.join(&template_path);
    let config_full = base.join(&config_path);

    // Security: path traversal check
    let canonical_dir = base.canonicalize()
        .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))?;

    if template_full.exists() {
        let canonical_template = template_full.canonicalize()
            .map_err(|e| OrchestratorError::ConfigNotFound(e.to_string()))?;
        if !canonical_template.starts_with(&canonical_dir) {
            return Err(OrchestratorError::ConfigWriteError(
                "Path traversal detected in template path".to_string(),
            ));
        }
    } else {
        return Err(OrchestratorError::ConfigNotFound(format!(
            "Template not found: {}",
            template_path
        )));
    }

    // Verify config destination is within component dir
    if let Some(parent) = config_full.parent() {
        if parent.exists() {
            let canonical_parent = parent.canonicalize()
                .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))?;
            if !canonical_parent.starts_with(&canonical_dir) {
                return Err(OrchestratorError::ConfigWriteError(
                    "Path traversal detected in config path".to_string(),
                ));
            }
        }
    }

    std::fs::copy(&template_full, &config_full)
        .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))?;

    Ok(())
}
