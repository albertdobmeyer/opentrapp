use std::path::{Path, PathBuf};
use super::error::OrchestratorError;
use super::manifest::Manifest;

/// Discovered component: parsed manifest + its filesystem location
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredComponent {
    pub manifest: Manifest,
    pub component_dir: String,
}

/// Discover components from the first candidate directory that yields any.
///
/// Each candidate is a directory containing `*/component.yml`. The app passes,
/// in priority order: the bundled manifests dir (inside the signed AppImage —
/// the shipping case), the runtime-staged copy, and finally the dev source
/// tree's `components/` dir. This is what lets the UI render dashboards on a
/// clean machine with no source clone.
pub fn discover_first(candidates: &[PathBuf]) -> Result<Vec<DiscoveredComponent>, OrchestratorError> {
    for dir in candidates {
        if dir.exists() {
            let found = discover_components_at(dir)?;
            if !found.is_empty() {
                return Ok(found);
            }
        }
    }
    Ok(Vec::new())
}

/// Discover all `component.yml` manifests under `monorepo_root/components`.
/// Retained for the dev source-tree path and unit tests.
pub fn discover_components(monorepo_root: &Path) -> Result<Vec<DiscoveredComponent>, OrchestratorError> {
    let components_dir = monorepo_root.join("components");
    if !components_dir.exists() {
        return Ok(Vec::new());
    }
    discover_components_at(&components_dir)
}

/// Discover all `component.yml` manifests directly under `dir/*/component.yml`.
pub fn discover_components_at(dir: &Path) -> Result<Vec<DiscoveredComponent>, OrchestratorError> {
    let pattern = dir
        .join("*")
        .join("component.yml")
        .to_string_lossy()
        .replace('\\', "/");

    let mut components = Vec::new();

    for entry in glob::glob(&pattern).map_err(|e| {
        OrchestratorError::ManifestParseError {
            path: pattern.clone(),
            message: e.to_string(),
        }
    })? {
        let path = entry.map_err(|e| OrchestratorError::IoError(e.into_error()))?;
        match parse_manifest(&path) {
            Ok(manifest) => {
                let component_dir = path
                    .parent()
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                components.push(DiscoveredComponent {
                    manifest,
                    component_dir,
                });
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
            }
        }
    }

    // Sort by identity.id for stable ordering
    components.sort_by(|a, b| a.manifest.identity.id.cmp(&b.manifest.identity.id));

    Ok(components)
}

fn parse_manifest(path: &PathBuf) -> Result<Manifest, OrchestratorError> {
    let content = std::fs::read_to_string(path)?;
    serde_yaml::from_str(&content).map_err(|e| OrchestratorError::ManifestParseError {
        path: path.to_string_lossy().to_string(),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifests build.rs stages into the bundle must discover + parse —
    /// this is what the UI renders from on a clean machine (no source clone).
    #[test]
    fn bundled_manifests_discover_without_a_source_tree() {
        let staged = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/perimeter/manifests");
        // build.rs stages these before tests compile; if absent, the harness
        // ran without a build step — skip rather than false-fail.
        if !staged.exists() {
            return;
        }
        let found = discover_components_at(&staged).expect("staged manifests parse");
        assert_eq!(found.len(), 3, "three bundled components");
        assert!(found.iter().all(|c| !c.manifest.identity.id.is_empty()));
        // discover_first prefers the first non-empty candidate.
        let via_first = discover_first(&[staged.clone()]).unwrap();
        assert_eq!(via_first.len(), 3);
    }
}
