//! First-run prerequisite checks + setup helpers (lifted from the Tauri command layer — ADR-0022
//! migration step 1). The report assembly, the container-runtime probe, the submodule scan, and
//! the template-copy all become transport-neutral fns so the same logic backs both the Tauri shim
//! and the future loopback web route.
//!
//! `create_config_from_template` carries a path-traversal guard (CLAUDE.md §9, like `config_ops`):
//! both the template source and the config destination must canonicalize to within the component
//! directory. That guard is factored into `copy_template_within(dir, ..)` so it is unit-tested
//! directly. (The same containment shape lives in `config_ops::{read,write}_within`; a follow-up
//! could extract one shared path-guard primitive — noted, not done here, to keep this a faithful
//! behavior-preserving lift.)

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::orchestrator::discovery::{discover_components, DiscoveredComponent};
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::runner::run_shell;

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

/// Assemble the first-run prerequisite report for the perimeter rooted at `root`: which container
/// runtime is available, which submodule directories are present, and per-component missing
/// configs / check-command results.
pub async fn check_prerequisites(root: &Path) -> Result<PrerequisiteReport, OrchestratorError> {
    let container_runtime = check_container_runtime(root).await;

    // Submodule directories (legacy components/ layout).
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

    // Per-component prerequisites.
    let discovered = discover_components(root).unwrap_or_default();
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

    Ok(PrerequisiteReport { container_runtime, submodules, components: component_prereqs })
}

/// Detect an available container runtime (podman preferred, then docker) by probing `--version`.
async fn check_container_runtime(root: &Path) -> ContainerRuntimeInfo {
    for runtime in &["podman", "docker"] {
        let cmd = format!("{} --version", runtime);
        if let Ok(result) = run_shell(&cmd, &root.to_path_buf(), 5).await {
            if result.exit_code == 0 {
                return ContainerRuntimeInfo {
                    found: true,
                    name: Some(runtime.to_string()),
                    version: Some(result.stdout.trim().to_string()),
                };
            }
        }
    }

    ContainerRuntimeInfo { found: false, name: None, version: None }
}

/// Run `git submodule update --init --recursive` in `root` (legacy submodule layout helper).
pub async fn init_submodules(root: &Path) -> Result<String, OrchestratorError> {
    let result = run_shell("git submodule update --init --recursive", &root.to_path_buf(), 120).await?;

    if result.exit_code != 0 {
        return Err(OrchestratorError::ExecutionError(format!(
            "git submodule update failed: {}",
            result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Copy a component's bundled template file to a config destination. `ComponentNotFound` if the id
/// is unknown; the traversal-guarded copy itself is `copy_template_within`.
pub fn create_config_from_template(
    components: &[DiscoveredComponent],
    component_id: String,
    config_path: String,
    template_path: String,
) -> Result<(), OrchestratorError> {
    let component_dir = components
        .iter()
        .find(|c| c.manifest.identity.id == component_id)
        .map(|c| c.component_dir.clone())
        .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;

    copy_template_within(Path::new(&component_dir), &config_path, &template_path)
}

/// Copy `template_path` → `config_path`, both resolved under `component_dir`, refusing any path
/// (source or destination) that escapes it. The template must exist (`ConfigNotFound` otherwise).
pub fn copy_template_within(
    component_dir: &Path,
    config_path: &str,
    template_path: &str,
) -> Result<(), OrchestratorError> {
    let template_full = component_dir.join(template_path);
    let config_full = component_dir.join(config_path);

    // Security: path traversal check (CLAUDE.md §9).
    let canonical_dir = component_dir
        .canonicalize()
        .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))?;

    if template_full.exists() {
        let canonical_template = template_full
            .canonicalize()
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

    // Verify the config destination is within the component dir.
    if let Some(parent) = config_full.parent() {
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique, self-cleaning temp base per test — avoids a `tempfile` dev-dep (keeps core lean).
    struct TempBase(PathBuf);
    impl TempBase {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("opentrapp-prereq-{}-{}", std::process::id(), tag));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempBase(dir)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempBase {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn check_prerequisites_empty_root_has_no_submodules_or_components() {
        let base = TempBase::new("empty-root");
        let report = check_prerequisites(base.path()).await.unwrap();
        assert!(report.submodules.is_empty(), "empty root has no submodule dirs");
        assert!(report.components.is_empty(), "empty root discovers no components");
    }

    #[test]
    fn create_config_unknown_component_is_not_found() {
        let err = create_config_from_template(&[], "nope".into(), "a".into(), "b".into())
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }

    #[test]
    fn copy_template_within_happy_path_copies_file() {
        let base = TempBase::new("copy-happy");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        std::fs::write(comp.join("env.example"), "KEY=value").unwrap();

        copy_template_within(&comp, ".env", "env.example").unwrap();
        assert_eq!(std::fs::read_to_string(comp.join(".env")).unwrap(), "KEY=value");
    }

    #[test]
    fn copy_template_within_missing_template_is_not_found() {
        let base = TempBase::new("copy-missing");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        let err = copy_template_within(&comp, ".env", "nope.example").unwrap_err();
        assert!(matches!(err, OrchestratorError::ConfigNotFound(m) if m.contains("Template not found")));
    }

    #[test]
    fn copy_template_within_rejects_traversal_in_template_source() {
        let base = TempBase::new("copy-tmpl-traversal");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        std::fs::write(base.path().join("secret.example"), "SECRET").unwrap();

        let err = copy_template_within(&comp, ".env", "../secret.example").unwrap_err();
        assert!(matches!(err, OrchestratorError::ConfigWriteError(m) if m.contains("traversal")));
        assert!(!comp.join(".env").exists(), "a rejected copy must not create the destination");
    }

    #[test]
    fn copy_template_within_rejects_traversal_in_config_destination() {
        let base = TempBase::new("copy-dest-traversal");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        std::fs::write(comp.join("env.example"), "KEY=value").unwrap();
        let escaped = base.path().join("escaped.env");

        let err = copy_template_within(&comp, "../escaped.env", "env.example").unwrap_err();
        assert!(matches!(err, OrchestratorError::ConfigWriteError(m) if m.contains("traversal")));
        assert!(!escaped.exists(), "a rejected traversal copy must not create the escaping file");
    }
}
