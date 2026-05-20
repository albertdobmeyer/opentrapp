use std::path::PathBuf;
use tauri::{AppHandle, Manager as _, State};
use crate::orchestrator::discovery::{discover_components, discover_first, DiscoveredComponent};
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::podman;
use crate::orchestrator::state::AppState;

/// Directories to search for `*/component.yml`, in priority order:
/// the bundled manifests (inside the signed AppImage — the shipping case),
/// the runtime-staged copy, then the dev source tree's `components/`.
fn manifest_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut cands = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        cands.push(res.join("perimeter").join("manifests"));
    }
    cands.push(podman::resource_dir().join("manifests"));
    if let Ok(cwd) = std::env::current_dir() {
        cands.push(cwd.join("components"));
        cands.push(cwd.join("..").join("components"));
    }
    cands
}

#[tauri::command]
pub async fn list_components(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<DiscoveredComponent>, OrchestratorError> {
    let discovered = discover_first(&manifest_candidates(&app))?;

    // Update cached components
    let mut components = state.components.lock().unwrap();
    *components = discovered.clone();

    Ok(discovered)
}

#[tauri::command]
pub async fn get_component(
    app: AppHandle,
    state: State<'_, AppState>,
    component_id: String,
) -> Result<DiscoveredComponent, OrchestratorError> {
    // Try the cache first
    {
        let components = state.components.lock().unwrap();
        if let Some(found) = components
            .iter()
            .find(|c| c.manifest.identity.id == component_id)
        {
            return Ok(found.clone());
        }
    }

    // Cache miss (empty or component not found) — discover from the bundle.
    let discovered = discover_first(&manifest_candidates(&app))?;
    let result = discovered
        .iter()
        .find(|c| c.manifest.identity.id == component_id)
        .cloned();

    // Populate cache so subsequent calls are fast
    let mut components = state.components.lock().unwrap();
    *components = discovered;

    result.ok_or_else(|| OrchestratorError::ComponentNotFound(component_id))
}

#[tauri::command]
pub async fn set_monorepo_root(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<DiscoveredComponent>, OrchestratorError> {
    let new_root = PathBuf::from(&path);

    // Validate: must have a components/ directory
    if !new_root.join("components").exists() {
        return Err(OrchestratorError::NotFound(format!(
            "No components/ directory found at: {}",
            path
        )));
    }

    // Update the root
    {
        let mut root = state.runtime_data_dir.write().unwrap();
        *root = new_root.clone();
    }

    // Re-discover and cache
    let discovered = discover_components(&new_root)?;
    let mut components = state.components.lock().unwrap();
    *components = discovered.clone();

    Ok(discovered)
}
