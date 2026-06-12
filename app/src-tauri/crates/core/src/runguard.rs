//! Single-instance guard + orphan-reap, for the headless daemon (Phase B,
//! ADR-0019).
//!
//! Mirrors the GUI's `lifecycle.rs` runguard, but takes an explicit `data_dir`
//! (rather than resolving `$HOME` itself) so the daemon owns it cleanly. Same
//! on-disk format (`<data_dir>/runguard.pid` holds the owner PID), so the daemon
//! and the current GUI are interoperable during the migration: whoever starts
//! reaps the other's orphan containers if its PID is dead.

use std::path::{Path, PathBuf};

/// The PID file the owning process writes.
pub fn runguard_path(data_dir: &Path) -> PathBuf {
    data_dir.join("runguard.pid")
}

/// Is `pid` a live process? Linux reads `/proc/<pid>`; other Unix/Windows is
/// conservative (assume alive) so we never reap a perimeter we don't own.
#[cfg(target_os = "linux")]
pub fn is_pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(target_os = "linux"))]
pub fn is_pid_alive(_pid: u32) -> bool {
    true
}

/// If a DIFFERENT live process holds the guard, return its PID — the daemon
/// refuses to start a second perimeter owner. `None` means we are free to take
/// it (no file, a dead owner, or the file is already ours).
pub fn held_by_other(data_dir: &Path) -> Option<u32> {
    let prev = std::fs::read_to_string(runguard_path(data_dir)).ok()?;
    let pid = prev.trim().parse::<u32>().ok()?;
    if pid != std::process::id() && is_pid_alive(pid) {
        Some(pid)
    } else {
        None
    }
}

/// Read the previous session's PID. If that PID is dead, the previous owner was
/// killed unrecoverably (SIGKILL / OOM / hard reboot) — reap any orphan
/// containers it left before starting fresh. Then write our PID.
pub fn establish(data_dir: &Path) {
    let path = runguard_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(prev) = std::fs::read_to_string(&path) {
        if let Ok(prev_pid) = prev.trim().parse::<u32>() {
            if !is_pid_alive(prev_pid) {
                eprintln!("[runguard] previous pid={prev_pid} is dead — reaping orphan containers");
                let _ = crate::orchestrator::podman::perimeter_down(data_dir);
            } else {
                eprintln!("[runguard] previous pid={prev_pid} is still alive — not reaping");
            }
        }
    }
    let pid = std::process::id();
    match std::fs::write(&path, pid.to_string()) {
        Ok(()) => eprintln!("[runguard] established (pid={pid})"),
        Err(e) => eprintln!("[runguard] failed to write pid file: {e}"),
    }
}

/// Clear the PID file on graceful exit. Best-effort: if it survives, the next
/// owner sees our (now-dead) PID and reaps correctly anyway.
pub fn clear(data_dir: &Path) {
    let _ = std::fs::remove_file(runguard_path(data_dir));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("opentrapp-runguard-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn path_is_under_data_dir() {
        let d = Path::new("/tmp/x");
        assert_eq!(runguard_path(d), Path::new("/tmp/x/runguard.pid"));
    }

    #[test]
    fn own_pid_is_alive() {
        assert!(is_pid_alive(std::process::id()));
    }

    #[test]
    fn held_by_other_excludes_self_and_missing() {
        let d = temp_dir("held");
        // No file → free.
        assert_eq!(held_by_other(&d), None);
        // Our own pid → not "other".
        std::fs::write(runguard_path(&d), std::process::id().to_string()).unwrap();
        assert_eq!(held_by_other(&d), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn clear_removes_pid_file() {
        let d = temp_dir("clear");
        std::fs::write(runguard_path(&d), "12345").unwrap();
        assert!(runguard_path(&d).exists());
        clear(&d);
        assert!(!runguard_path(&d).exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
