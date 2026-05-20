use std::path::PathBuf;
use tauri::State;
use crate::orchestrator::discovery::{discover_components, DiscoveredComponent};
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::state::AppState;

#[tauri::command]
pub async fn list_components(
    state: State<'_, AppState>,
) -> Result<Vec<DiscoveredComponent>, OrchestratorError> {
    let root = state.runtime_data_dir.read().unwrap().clone();
    let discovered = discover_components(&root)?;

    // Update cached components
    let mut components = state.components.lock().unwrap();
    *components = discovered.clone();

    Ok(discovered)
}

#[tauri::command]
pub async fn get_component(
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

    // Cache miss (empty or component not found) — discover from filesystem
    let root = state.runtime_data_dir.read().unwrap().clone();
    let discovered = discover_components(&root)?;
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
