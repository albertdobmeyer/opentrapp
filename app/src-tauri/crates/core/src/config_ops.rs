//! Component config read/write with path-traversal containment (lifted from the Tauri command
//! layer — ADR-0022 migration step 1; the CLAUDE.md §9 path-traversal protection).
//!
//! The security-load-bearing part — refusing any `config_path` that escapes the component
//! directory (canonicalize + `starts_with`, TOCTOU-safe by operating on the canonical path) — is
//! factored into `read_within` / `write_within`, which take the already-resolved component dir so
//! the containment guard is unit-tested directly (no fabricated `DiscoveredComponent`). The
//! `read_config` / `write_config` slice-lookup is the thin outer layer. Behavior is byte-for-byte
//! the original GUI handler.

use std::path::Path;

use crate::orchestrator::discovery::DiscoveredComponent;
use crate::orchestrator::error::OrchestratorError;

/// Resolve a component's on-disk directory from the discovery cache.
fn component_dir_for(
    components: &[DiscoveredComponent],
    component_id: &str,
) -> Result<String, OrchestratorError> {
    components
        .iter()
        .find(|c| c.manifest.identity.id == component_id)
        .map(|c| c.component_dir.clone())
        .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.to_string()))
}

/// Read a component-relative config file. `ComponentNotFound` if the id is unknown.
pub fn read_config(
    components: &[DiscoveredComponent],
    component_id: String,
    config_path: String,
) -> Result<String, OrchestratorError> {
    let dir = component_dir_for(components, &component_id)?;
    read_within(Path::new(&dir), &config_path)
}

/// Write a component-relative config file. `ComponentNotFound` if the id is unknown.
pub fn write_config(
    components: &[DiscoveredComponent],
    component_id: String,
    config_path: String,
    content: String,
) -> Result<(), OrchestratorError> {
    let dir = component_dir_for(components, &component_id)?;
    write_within(Path::new(&dir), &config_path, &content)
}

/// Read `config_path` resolved under `component_dir`, refusing any path that escapes it. A missing
/// file yields an empty string (the GUI's "no config yet" case); the containment check therefore
/// only fires when the resolved target actually exists.
pub fn read_within(component_dir: &Path, config_path: &str) -> Result<String, OrchestratorError> {
    let full_path = component_dir.join(config_path);

    if !full_path.exists() {
        return Ok(String::new());
    }

    // Security: ensure the resolved path does not escape the component directory.
    let canonical_dir = component_dir
        .canonicalize()
        .map_err(|e| OrchestratorError::ConfigNotFound(e.to_string()))?;
    let canonical_file = full_path
        .canonicalize()
        .map_err(|e| OrchestratorError::ConfigNotFound(e.to_string()))?;

    if !canonical_file.starts_with(&canonical_dir) {
        return Err(OrchestratorError::ConfigNotFound("Path traversal detected".to_string()));
    }

    // Read via the canonical path to prevent TOCTOU symlink swaps.
    std::fs::read_to_string(&canonical_file)
        .map_err(|e| OrchestratorError::ConfigNotFound(e.to_string()))
}

/// Write `content` to `config_path` resolved under `component_dir`, refusing any path that escapes
/// it (existing file: canonical-file check; new file: canonical-parent check). Operates on the
/// canonical path for existing files to defeat TOCTOU symlink swaps.
pub fn write_within(
    component_dir: &Path,
    config_path: &str,
    content: &str,
) -> Result<(), OrchestratorError> {
    let full_path = component_dir.join(config_path);

    // Security: ensure the path does not escape the component directory.
    let canonical_dir = component_dir
        .canonicalize()
        .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))?;

    if full_path.exists() {
        // Existing file: canonicalize and verify containment, then write via the canonical path
        // (defeats TOCTOU symlink swaps).
        let canonical_file = full_path
            .canonicalize()
            .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))?;
        if !canonical_file.starts_with(&canonical_dir) {
            return Err(OrchestratorError::ConfigWriteError("Path traversal detected".to_string()));
        }
        std::fs::write(&canonical_file, content)
            .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))
    } else {
        // New file: verify its (existing) parent is within the component dir.
        if let Some(parent) = full_path.parent() {
            if parent.exists() {
                let canonical_parent = parent
                    .canonicalize()
                    .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))?;
                if !canonical_parent.starts_with(&canonical_dir) {
                    return Err(OrchestratorError::ConfigWriteError(
                        "Path traversal detected".to_string(),
                    ));
                }
            }
        }
        std::fs::write(&full_path, content)
            .map_err(|e| OrchestratorError::ConfigWriteError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Unique, self-cleaning temp base per test — avoids a `tempfile` dev-dep (keeps core lean).
    struct TempBase(PathBuf);
    impl TempBase {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("opentrapp-config-{}-{}", std::process::id(), tag));
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

    #[test]
    fn read_config_unknown_component_is_not_found() {
        let err = read_config(&[], "nope".into(), "x.yml".into()).unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }

    #[test]
    fn write_config_unknown_component_is_not_found() {
        let err = write_config(&[], "nope".into(), "x.yml".into(), "data".into()).unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }

    #[test]
    fn read_within_missing_file_returns_empty() {
        let base = TempBase::new("read-missing");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        assert_eq!(read_within(&comp, "nope.yml").unwrap(), "");
    }

    #[test]
    fn write_then_read_within_roundtrips() {
        let base = TempBase::new("roundtrip");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        write_within(&comp, "conf.yml", "hello: world").unwrap();
        assert_eq!(read_within(&comp, "conf.yml").unwrap(), "hello: world");
    }

    #[test]
    fn read_within_rejects_traversal_to_existing_outside_file() {
        let base = TempBase::new("read-traversal");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        std::fs::write(base.path().join("secret.txt"), "TOP SECRET").unwrap();
        let err = read_within(&comp, "../secret.txt").unwrap_err();
        assert!(matches!(err, OrchestratorError::ConfigNotFound(m) if m.contains("traversal")));
    }

    #[test]
    fn write_within_rejects_traversal_and_writes_nothing() {
        let base = TempBase::new("write-traversal");
        let comp = base.path().join("component");
        std::fs::create_dir_all(&comp).unwrap();
        let escaped = base.path().join("escaped.txt");
        let err = write_within(&comp, "../escaped.txt", "pwned").unwrap_err();
        assert!(matches!(err, OrchestratorError::ConfigWriteError(m) if m.contains("traversal")));
        assert!(!escaped.exists(), "a rejected traversal write must not create the escaping file");
    }
}
