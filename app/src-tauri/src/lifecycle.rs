//! Perimeter lifecycle ownership (P11).
//!
//! The app and the 4-container perimeter share a single lifetime. App start
//! brings the perimeter up; graceful exit (window quit, tray Quit, SIGTERM,
//! SIGINT) tears it down. RunGuard reaps orphan containers from a previous
//! SIGKILL'd session on the next launch — best-effort, since SIGKILL itself
//! cannot be caught.
//!
//! Auto-restart of individual dead containers is delegated to the
//! `restart: unless-stopped` policy in `compose.yml`. The watchdog in this
//! module REPORTS state — it does not take corrective action.

use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

// ─── Constants ────────────────────────────────────────────────────────

pub const REDACTED: &str = "<REDACTED>";

/// The 4 compose *services* that constitute the perimeter. Container names
/// are project-prefixed at runtime (e.g. `lobster-trapp_vault-proxy_1`); we
/// filter by `com.docker.compose.service` label so the same code works
/// regardless of project name.
const PERIMETER_CONTAINERS: [&str; 4] =
    ["vault-agent", "vault-proxy", "vault-forge", "vault-pioneer"];

// ─── State types (frontend-visible) ───────────────────────────────────

/// High-level perimeter state. Maps to the 6-state hero machine in
/// `docs/specs/2026-04-29-delightful-sloth-target-ux.md`. The `Paused` and
/// `ErrorKey` states are user-initiated / user-driven and aren't yet
/// inferable purely from container status; they'll be wired in once the
/// frontend Home rebuild lands (Pass 6).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PerimeterState {
    /// Wizard hasn't run; no containers exist.
    NotSetup,
    /// Compose-up is in progress; not all containers are visible yet.
    /// Reserved for Pass 6 hero-state-machine wiring; not currently
    /// emitted by the watchdog (which uses `Recovering` for any partial
    /// state, including bring-up).
    #[allow(dead_code)]
    Starting,
    /// All 4 containers running.
    RunningSafely,
    /// 1–3 of 4 containers running. Transient or under recovery via
    /// `restart: unless-stopped`.
    Recovering,
    /// All 4 containers stopped — could be paused (user-initiated) or
    /// fully crashed. Refined post-Pass-6 when we track user intent.
    Stopped,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ContainerStatus {
    pub name: String,
    pub running: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PerimeterStatus {
    pub state: PerimeterState,
    pub containers: Vec<ContainerStatus>,
    /// Unix-millis timestamp of the most recent watchdog poll. Lets the
    /// frontend show a "last checked" hint and detect a stalled watchdog.
    pub last_checked_unix_ms: u64,
}

impl PerimeterStatus {
    fn empty() -> Self {
        Self {
            state: PerimeterState::NotSetup,
            containers: Vec::new(),
            last_checked_unix_ms: 0,
        }
    }
}

/// Tauri-managed shared state. Read by the `get_perimeter_state` command
/// and updated by the watchdog. Includes a `paused` flag separate from the
/// container-derived state so a user-initiated stop is distinguishable from
/// a crash.
pub struct PerimeterStateStore {
    pub status: Mutex<PerimeterStatus>,
    pub paused: RwLock<bool>,
}

impl PerimeterStateStore {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(PerimeterStatus::empty()),
            paused: RwLock::new(is_paused_persisted()),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.read().map(|g| *g).unwrap_or(false)
    }

    pub fn set_paused(&self, value: bool) {
        if let Ok(mut g) = self.paused.write() {
            *g = value;
        }
    }
}

/// Persisted pause flag — file presence at `~/.lobster-trapp/paused` means
/// the user paused their assistant and expects it to stay paused across
/// app restarts. Best-effort: a missing file (the default) means "not
/// paused", so a corrupted home dir simply resumes normal behavior.
fn paused_marker_path() -> PathBuf {
    runguard_dir().join("paused")
}

pub fn is_paused_persisted() -> bool {
    paused_marker_path().exists()
}

pub fn write_paused_marker() -> std::io::Result<()> {
    let path = paused_marker_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, "1")
}

pub fn clear_paused_marker() {
    let _ = std::fs::remove_file(paused_marker_path());
}

// ─── Secret redaction (defensive) ─────────────────────────────────────

/// Redact known token-bearing environment variables from a stderr blob
/// before it's logged. `podman compose` echoes the full container-creation
/// command on failure, including `TELEGRAM_BOT_TOKEN=...` in cleartext —
/// which would leak into our log if surfaced verbatim. Mirrors the
/// vault-proxy redaction pattern from Finding #1 (project_decisions.md).
pub fn redact_secrets(s: &str) -> String {
    const SENSITIVE_VARS: &[&str] = &[
        "TELEGRAM_BOT_TOKEN",
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut out = s.to_string();
    for var in SENSITIVE_VARS {
        let needle = format!("{var}=");
        let mut search_from = 0;
        while let Some(rel) = out[search_from..].find(&needle) {
            let pos = search_from + rel;
            let after = pos + needle.len();
            let end = out[after..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|n| after + n)
                .unwrap_or(out.len());
            out.replace_range(after..end, REDACTED);
            search_from = after + REDACTED.len();
        }
    }
    out
}

// ─── Compose runner ───────────────────────────────────────────────────

/// Try `podman compose <args...>` first, then `docker compose <args...>`.
/// Returns true if either succeeded with a zero exit code. Intentionally
/// non-fatal — the app boots even if no container runtime is installed
/// yet (e.g. first launch before the wizard has run System Check).
///
/// Wraps the call with `timeout(1)` if that binary is on PATH; falls back
/// to a direct invocation if not. Stderr is redacted via `redact_secrets`
/// before logging.
pub fn run_compose(root: &Path, args: &[&str], timeout: Duration) -> bool {
    for runtime in &["podman", "docker"] {
        let secs = timeout.as_secs().max(1).to_string();

        let wrapped = StdCommand::new("timeout")
            .args(["--signal=TERM", "--kill-after=5s", &secs, runtime, "compose"])
            .args(args)
            .current_dir(root)
            .output();

        let output = match wrapped {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => StdCommand::new(runtime)
                .arg("compose")
                .args(args)
                .current_dir(root)
                .output(),
            other => other,
        };

        match output {
            Ok(out) if out.status.success() => {
                eprintln!("[lifecycle] {} compose {} → ok", runtime, args.join(" "));
                return true;
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!(
                    "[lifecycle] {} compose {} exited {}: {}",
                    runtime,
                    args.join(" "),
                    out.status,
                    redact_secrets(stderr.trim())
                );
            }
            Err(e) => {
                eprintln!(
                    "[lifecycle] failed to spawn {}: {} — trying next runtime",
                    runtime, e
                );
            }
        }
    }
    false
}

/// Bring the 4-container perimeter up. Idempotent — `compose up -d` is a
/// no-op when containers are already running. Spawned on a background
/// thread so the Tauri window appears immediately even if a first-time
/// pull is happening.
pub fn bring_perimeter_up_async(root: PathBuf) {
    std::thread::spawn(move || {
        run_compose(&root, &["up", "-d"], Duration::from_secs(90));
    });
}

/// Tear the perimeter down on graceful exit. Synchronous — we want the
/// containers actually stopped before the process terminates so we don't
/// leak the running perimeter. 30s budget enforced by `timeout(1)`.
pub fn bring_perimeter_down_sync(root: &Path) {
    run_compose(root, &["down"], Duration::from_secs(30));
}

// ─── RunGuard (orphan reap on next launch after SIGKILL) ──────────────

fn runguard_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".lobster-trapp")
}

fn runguard_path() -> PathBuf {
    runguard_dir().join("runguard.pid")
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    // /proc/<pid> exists iff process is running. Linux-specific but
    // reliable. (macOS would use libc::kill(pid, 0).)
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(unix))]
fn is_pid_alive(_pid: u32) -> bool {
    // Conservative on non-Unix: assume previous PID is alive so we don't
    // kill someone else's containers. Best-effort — Pass 4 wrap-up here.
    true
}

/// Read the previous session's PID file. If that PID is no longer running,
/// the previous session was killed unrecoverably (SIGKILL, OOM, hard
/// reboot) — reap any orphan containers it left behind before we start
/// fresh. Then write our PID for the next session to find.
pub fn establish_runguard(root: &Path) {
    let path = runguard_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(prev) = std::fs::read_to_string(&path) {
        if let Ok(prev_pid) = prev.trim().parse::<u32>() {
            if !is_pid_alive(prev_pid) {
                eprintln!(
                    "[lifecycle] previous session pid={prev_pid} is dead — \
                     reaping orphan containers before fresh start"
                );
                run_compose(root, &["down"], Duration::from_secs(30));
            } else {
                eprintln!(
                    "[lifecycle] previous session pid={prev_pid} is still alive — \
                     not reaping (another instance may be running)"
                );
            }
        }
    }
    let pid = std::process::id();
    if let Err(e) = std::fs::write(&path, pid.to_string()) {
        eprintln!("[lifecycle] failed to write runguard pid file: {e}");
    } else {
        eprintln!("[lifecycle] runguard established (pid={pid})");
    }
}

/// Clear the PID file on graceful exit. If we fail to clear, the next
/// session will see our (now-dead) PID and reap correctly anyway — clearing
/// is just an optimization that avoids a redundant `compose down` on the
/// happy path.
pub fn clear_runguard() {
    let _ = std::fs::remove_file(runguard_path());
}

// ─── Container status probe ──────────────────────────────────────────

fn is_service_running(service: &str) -> bool {
    for runtime in &["podman", "docker"] {
        let out = StdCommand::new(runtime)
            .args([
                "ps",
                "--filter",
                &format!("label=com.docker.compose.service={}", service),
                "--filter",
                "status=running",
                "--format",
                "{{.Names}}",
            ])
            .output();
        if let Ok(o) = out {
            if o.status.success() && !o.stdout.trim_ascii().is_empty() {
                return true;
            }
        }
    }
    false
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn compute_perimeter_status() -> PerimeterStatus {
    let mut containers = Vec::with_capacity(PERIMETER_CONTAINERS.len());
    let mut running_count = 0usize;
    for name in PERIMETER_CONTAINERS {
        let running = is_service_running(name);
        if running {
            running_count += 1;
        }
        containers.push(ContainerStatus {
            name: name.to_string(),
            running,
        });
    }
    let state = match running_count {
        0 => PerimeterState::Stopped,
        n if n == PERIMETER_CONTAINERS.len() => PerimeterState::RunningSafely,
        _ => PerimeterState::Recovering,
    };
    PerimeterStatus {
        state,
        containers,
        last_checked_unix_ms: now_unix_ms(),
    }
}

// ─── Watchdog ─────────────────────────────────────────────────────────

/// Spawn the perimeter-state watchdog as a tokio task. Polls every
/// `interval`, updates the Tauri-managed `PerimeterStateStore`, emits a
/// `perimeter-state-changed` event when the state transitions, and updates
/// the tray-icon tooltip + status menu item.
///
/// The watchdog is a REPORTER, not a controller. Auto-restart of dead
/// containers is owned by the `restart: unless-stopped` policy in
/// `compose.yml`. If a sustained outage is detected, the user sees it via
/// the tray + the (Pass-6) hero state — they take action, not us.
pub fn spawn_watchdog(handle: AppHandle, interval: Duration) {
    tauri::async_runtime::spawn(async move {
        // First tick fires immediately (tokio::time::interval fires the first
        // tick at t=0, then every `interval` after).
        let mut ticker = tokio::time::interval(interval);
        let mut last_state: Option<PerimeterState> = None;

        loop {
            ticker.tick().await;

            // Run the synchronous status probe on a blocking thread so
            // we don't stall the tokio reactor on slow podman calls.
            let status =
                tokio::task::spawn_blocking(compute_perimeter_status).await.ok();
            let Some(status) = status else { continue };

            // Update the store + decide whether to emit + update tray.
            let state_changed = last_state.as_ref() != Some(&status.state);
            last_state = Some(status.state.clone());

            if let Some(store) = handle.try_state::<PerimeterStateStore>() {
                if let Ok(mut guard) = store.status.lock() {
                    *guard = status.clone();
                }
            }

            if state_changed {
                let _ = handle.emit("perimeter-state-changed", &status);
            }
            update_tray_for_state(&handle, &status.state);
        }
    });
}

fn tray_label_for(state: &PerimeterState) -> &'static str {
    match state {
        PerimeterState::NotSetup => "Assistant — not set up",
        PerimeterState::Starting => "Assistant — starting…",
        PerimeterState::RunningSafely => "Assistant — running safely",
        PerimeterState::Recovering => "Assistant — recovering",
        PerimeterState::Stopped => "Assistant — stopped",
    }
}

fn update_tray_for_state(handle: &AppHandle, state: &PerimeterState) {
    let label = tray_label_for(state);
    if let Some(tray) = handle.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(label));
    }
    // Note: updating the menu's "status" item text requires a stored
    // handle to that MenuItem; deferred until Pass 4 wrap-up if needed.
    // The tooltip update covers the primary user-visible signal.
}

// ─── Signal handlers (Unix) ──────────────────────────────────────────

/// Install SIGTERM + SIGINT handlers that trigger graceful exit
/// (`app.exit(0)`), which in turn fires `RunEvent::Exit` and runs
/// `bring_perimeter_down_sync`. SIGKILL is intentionally not caught —
/// it can't be — and is handled instead by RunGuard reaping orphans on
/// the next launch.
#[cfg(unix)]
pub fn install_signal_handlers(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[lifecycle] could not install SIGTERM handler: {e}");
                return;
            }
        };
        let mut sigint = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[lifecycle] could not install SIGINT handler: {e}");
                return;
            }
        };
        let signame = tokio::select! {
            _ = sigterm.recv() => "SIGTERM",
            _ = sigint.recv()  => "SIGINT",
        };
        eprintln!("[lifecycle] received {signame} — initiating graceful exit");
        handle.exit(0);
    });
}

#[cfg(not(unix))]
pub fn install_signal_handlers(_handle: AppHandle) {
    // Windows: rely on RunEvent::ExitRequested from window-close + tray Quit.
    // CTRL_BREAK_EVENT/CTRL_CLOSE_EVENT support deferred.
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_telegram_bot_token() {
        let input = "podman run -e TELEGRAM_BOT_TOKEN=12345:abcdef -e FOO=bar ...";
        let out = redact_secrets(input);
        assert!(!out.contains("12345:abcdef"));
        assert!(out.contains("TELEGRAM_BOT_TOKEN=<REDACTED>"));
        assert!(out.contains("FOO=bar"));
    }

    #[test]
    fn redacts_multiple_occurrences_without_looping() {
        let input = "ANTHROPIC_API_KEY=sk-ant-aaa OPENAI_API_KEY=sk-bbb";
        let out = redact_secrets(input);
        assert!(!out.contains("sk-ant-aaa"));
        assert!(!out.contains("sk-bbb"));
        assert!(out.matches(REDACTED).count() == 2);
    }

    #[test]
    fn passes_through_unrelated_text() {
        let input = "exit 137: SIGKILL received";
        assert_eq!(redact_secrets(input), input);
    }

    #[test]
    fn perimeter_status_serializes_to_snake_case() {
        let s = PerimeterStatus {
            state: PerimeterState::RunningSafely,
            containers: vec![ContainerStatus {
                name: "vault-agent".into(),
                running: true,
            }],
            last_checked_unix_ms: 1_000_000,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"running_safely\""));
        assert!(json.contains("\"vault-agent\""));
    }

    #[test]
    fn tray_label_covers_all_states() {
        for state in [
            PerimeterState::NotSetup,
            PerimeterState::Starting,
            PerimeterState::RunningSafely,
            PerimeterState::Recovering,
            PerimeterState::Stopped,
        ] {
            let label = tray_label_for(&state);
            assert!(label.starts_with("Assistant"));
            assert!(!label.is_empty());
        }
    }
}
