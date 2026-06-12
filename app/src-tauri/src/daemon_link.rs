//! GUI → daemon link (Phase B / B4b, ADR-0019).
//!
//! When opted in (`OPENTRAPP_DAEMON_DEFER=1`), the GUI ensures a headless
//! `opentrapp-daemon` owns the perimeter — launching the bundled sidecar
//! **detached** (it must outlive the GUI, unlike a normal Tauri sidecar that
//! dies with the app) — and then acts as a viewer. Default OFF: the GUI
//! self-owns exactly as before. Any failure (no binary, spawn error, daemon
//! doesn't take the guard) falls back to self-owning — the safety net for a
//! change that cannot be verified in CI (no display, no perimeter).

use std::path::{Path, PathBuf};
use std::time::Duration;

use opentrapp_core::runguard;

/// Opt-in only — default OFF until the defer + memory win are verified on capable
/// hardware (this box / CI can run neither the packaged app nor the perimeter).
fn defer_enabled() -> bool {
    matches!(
        std::env::var("OPENTRAPP_DAEMON_DEFER").as_deref(),
        Ok("1") | Ok("true")
    )
}

/// The bundled daemon sits next to the app executable (tauri `externalBin`).
fn resolve_daemon_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = if cfg!(windows) {
        "opentrapp-daemon.exe"
    } else {
        "opentrapp-daemon"
    };
    let path = dir.join(name);
    path.exists().then_some(path)
}

/// Spawn the daemon detached so it outlives this GUI process. A new process group
/// means signals to the GUI's group don't reach it; stdio is null'd.
#[cfg(unix)]
fn spawn_detached(path: &Path) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    std::process::Command::new(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .map(|_| ())
}

#[cfg(not(unix))]
fn spawn_detached(path: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    std::process::Command::new(path)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}

/// Ensure a daemon owns the perimeter. Returns `true` iff (after this call) a
/// daemon holds the RunGuard, in which case the GUI should act as a viewer.
/// Default `false` (opt-in); every failure path returns `false` so the GUI
/// self-owns exactly as before.
pub fn ensure_daemon(data_dir: &Path) -> bool {
    if !defer_enabled() {
        return false;
    }
    if runguard::held_by_other(data_dir).is_some() {
        return true; // a daemon already owns it
    }
    let Some(path) = resolve_daemon_path() else {
        eprintln!("[daemon-link] defer requested but no bundled daemon found — self-owning");
        return false;
    };
    if let Err(e) = spawn_detached(&path) {
        eprintln!("[daemon-link] failed to launch daemon ({e}) — self-owning");
        return false;
    }
    // Give it a moment to take the RunGuard.
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        if runguard::held_by_other(data_dir).is_some() {
            eprintln!("[daemon-link] daemon launched and owns the perimeter");
            return true;
        }
    }
    eprintln!("[daemon-link] daemon did not take the RunGuard in time — self-owning");
    false
}
