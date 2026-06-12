//! The durable cross-process state contract between the headless daemon and the
//! GUI viewer (Phase B, ADR-0019).
//!
//! The daemon writes these marker files under `~/.opentrapp`; the viewer reads
//! them. Markers are the **source of truth** — when the live control socket
//! lands (a later slice) it is only a fast path layered over this durable state,
//! never a replacement (ADR-0019: "markers are truth; the socket is the fast
//! path"). The on-disk format mirrors the markers the GUI writes today
//! (`lifecycle.rs`: content `"1"` for the boolean flags, a unix-ms string for
//! `credentials-ok`), so the daemon and the current app stay byte-compatible
//! through the migration.

use std::path::{Path, PathBuf};

/// Perimeter paused by the user. Mirrors `~/.opentrapp/paused` (content `"1"`).
pub const PAUSED: &str = "paused";
/// Perimeter auto-paused to dormant to save memory (ADR-0018). Content `"1"`.
pub const DORMANT: &str = "dormant";
/// First successful activation has occurred. Content `"1"`.
pub const ACTIVATED: &str = "activated";
/// Credentials verified-good marker. Content: a unix-ms timestamp string.
pub const CREDENTIALS_OK: &str = "credentials-ok";
/// The boundary self-test FAILED after a (re)start — the perimeter was held
/// closed fail-closed (road-to-recommendable §1B, ADR-0018, task #45). Content:
/// a short reason string. The viewer surfaces this as a security alert; a clean
/// (re)start clears it.
pub const BOUNDARY_FAILED: &str = "boundary-failed";

/// Resolve `~/.opentrapp` from `$HOME`, matching `lifecycle.rs` and
/// `orchestrator::podman::runtime_data_dir` in the GUI crate.
pub fn default_data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".opentrapp")
}

/// Absolute path of a marker within `data_dir`.
pub fn marker_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join(name)
}

/// Is the marker present?
pub fn is_set(data_dir: &Path, name: &str) -> bool {
    marker_path(data_dir, name).exists()
}

/// Write a marker with the given content, creating `data_dir` if needed.
pub fn set(data_dir: &Path, name: &str, content: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    std::fs::write(marker_path(data_dir, name), content)
}

/// Write a boolean-style marker (content `"1"`), matching the GUI's format.
pub fn set_flag(data_dir: &Path, name: &str) -> std::io::Result<()> {
    set(data_dir, name, "1")
}

/// Remove a marker (idempotent — absence is not an error).
pub fn clear(data_dir: &Path, name: &str) {
    let _ = std::fs::remove_file(marker_path(data_dir, name));
}

/// A point-in-time read of the durable perimeter state. Raw booleans only —
/// precedence between them (dormant vs paused vs activated) is a policy decision
/// that lives in the daemon's status aggregator, not in this contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateSnapshot {
    pub activated: bool,
    pub paused: bool,
    pub dormant: bool,
    pub credentials_ok: bool,
}

/// Read all markers for `data_dir` in one pass.
pub fn snapshot(data_dir: &Path) -> StateSnapshot {
    StateSnapshot {
        activated: is_set(data_dir, ACTIVATED),
        paused: is_set(data_dir, PAUSED),
        dormant: is_set(data_dir, DORMANT),
        credentials_ok: is_set(data_dir, CREDENTIALS_OK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique, self-cleaning temp dir (no external crates — keeps core
    /// dependency-free, which is the whole point of this crate).
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("opentrapp-core-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn set_is_set_clear_round_trip() {
        let d = temp_dir("rt");
        assert!(!is_set(&d, PAUSED));
        set_flag(&d, PAUSED).unwrap();
        assert!(is_set(&d, PAUSED));
        assert_eq!(
            std::fs::read_to_string(marker_path(&d, PAUSED)).unwrap(),
            "1"
        );
        clear(&d, PAUSED);
        assert!(!is_set(&d, PAUSED));
        clear(&d, PAUSED); // idempotent — no panic on absent
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn snapshot_reflects_markers() {
        let d = temp_dir("snap");
        assert_eq!(
            snapshot(&d),
            StateSnapshot {
                activated: false,
                paused: false,
                dormant: false,
                credentials_ok: false,
            }
        );
        set_flag(&d, ACTIVATED).unwrap();
        set_flag(&d, DORMANT).unwrap();
        set(&d, CREDENTIALS_OK, "1718000000000").unwrap();
        let s = snapshot(&d);
        assert!(s.activated && s.dormant && s.credentials_ok && !s.paused);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn default_data_dir_ends_with_opentrapp() {
        assert!(default_data_dir().ends_with(".opentrapp"));
    }
}
