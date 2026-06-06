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

/// The 5 compose *services* that constitute the perimeter. Container names
/// are project-prefixed at runtime (e.g. `opentrapp_vault-proxy_1`); we
/// filter by `com.docker.compose.service` label so the same code works
/// regardless of project name.
const PERIMETER_CONTAINERS: [&str; 5] =
    ["vault-agent", "vault-proxy", "vault-egress", "vault-skills", "vault-social"];

// ─── State types (frontend-visible) ───────────────────────────────────

/// Bootstrap axis. Encodes whether the 3-container security shell
/// (proxy + forge + pioneer) is set up and healthy, independent of whether
/// the tenant (vault-agent) is running.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapState {
    /// First-launch: writing .env, no containers yet.
    Installing,
    /// Podman install / image build+pull / shell-up in progress.
    /// Set by the bootstrap subsystem (PR-3); computed from container
    /// presence alone until then.
    Bootstrapping,
    /// Proxy + forge + pioneer all up; shell is healthy.
    ShellReady,
    /// Bootstrap halted or shell containers missing; recovery card surfaces.
    ShellFailed,
}

/// Tenant axis. Encodes the state of vault-agent (the OpenClaw runtime),
/// which only runs after the user has activated their assistant.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TenantState {
    /// No agent container; not yet activated, or post-stop.
    Absent,
    /// Wizard committed, bringing agent up. Set by bootstrap subsystem (PR-3).
    Activating,
    /// Agent container up; bot operational.
    Running,
    /// User-initiated stop; persisted via `~/.opentrapp/paused` marker.
    Paused,
    /// Agent expected up (activated marker present, not paused) but absent.
    Errored,
}

/// Named steps in the 7-step bootstrap pipeline.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BootstrapStep {
    DetectRuntime,
    InstallRuntime,
    WriteEnv,
    BuildImages,
    PullImages,
    UpShell,
    VerifyShell,
    /// Bring vault-agent up as part of auto-activation after shell is ready.
    UpAgent,
}

impl BootstrapStep {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DetectRuntime => "detect-runtime",
            Self::InstallRuntime => "install-runtime",
            Self::WriteEnv => "write-env",
            Self::BuildImages => "build-images",
            Self::PullImages => "pull-images",
            Self::UpShell => "up-shell",
            Self::VerifyShell => "verify-shell",
            Self::UpAgent => "up-agent",
        }
    }
}

/// In-flight bootstrap pipeline step or failure cause. Set by the bootstrap
/// subsystem; nil until bootstrap starts. Stored in `PerimeterStateStore` so
/// the watchdog can incorporate it into state computation without re-polling.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum BootstrapProgress {
    /// Step is running; includes optional percent (0-100) and detail string.
    Step {
        step: BootstrapStep,
        step_index: u8,
        total_steps: u8,
        percent: Option<u8>,
        detail: Option<String>,
        started_at_unix_ms: u64,
    },
    /// Pipeline halted; cause code for the RecoveryCard taxonomy.
    Failed { cause: String, message: String, last_error: Option<String> },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ContainerStatus {
    pub name: String,
    pub running: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PerimeterStatus {
    pub bootstrap: BootstrapState,
    pub tenant: TenantState,
    pub containers: Vec<ContainerStatus>,
    /// Unix-millis timestamp of the most recent watchdog poll. Lets the
    /// frontend show a "last checked" hint and detect a stalled watchdog.
    pub last_checked_unix_ms: u64,
}

impl PerimeterStatus {
    fn empty() -> Self {
        Self {
            bootstrap: BootstrapState::Installing,
            tenant: TenantState::Absent,
            containers: Vec::new(),
            last_checked_unix_ms: 0,
        }
    }
}

/// Tauri-managed shared state. Read by the `get_perimeter_state` command
/// and updated by the watchdog. Marker-file mirrors (`paused`, `activated`)
/// are kept in sync here so command handlers can read/write without file I/O
/// on every access.
pub struct PerimeterStateStore {
    pub status: Mutex<PerimeterStatus>,
    /// Mirrors `~/.opentrapp/paused`. User-initiated stop.
    pub paused: RwLock<bool>,
    /// Mirrors `~/.opentrapp/activated`. Set after first successful activation.
    pub activated: RwLock<bool>,
    /// Unix-ms of last successful credential validation; `None` = unverified.
    pub credentials_ok_at: RwLock<Option<u64>>,
    /// In-flight bootstrap pipeline step. Set by bootstrap subsystem (PR-3).
    pub bootstrap_progress: RwLock<Option<BootstrapProgress>>,
    /// True when migration detected an existing v0.3 install with an invalid
    /// Anthropic key. Cleared when new credentials are committed successfully.
    pub migration_credential_warning: RwLock<bool>,
}

impl PerimeterStateStore {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(PerimeterStatus::empty()),
            paused: RwLock::new(is_paused_persisted()),
            activated: RwLock::new(is_activated_persisted()),
            credentials_ok_at: RwLock::new(read_credentials_ok_ts()),
            bootstrap_progress: RwLock::new(None),
            migration_credential_warning: RwLock::new(false),
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

    pub fn is_activated(&self) -> bool {
        self.activated.read().map(|g| *g).unwrap_or(false)
    }

    pub fn set_activated(&self, value: bool) {
        if let Ok(mut g) = self.activated.write() {
            *g = value;
        }
    }
}

// ─── Marker file helpers ───────────────────────────────────────────────

fn runguard_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".opentrapp")
}

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

// ── Dormant marker (Phase 3 idle auto-pause) ──────────────────────────
// Distinct from `paused` (user-initiated): `dormant` means the watchdog
// auto-paused the perimeter to save memory after an idle period, and a
// host-side waker resumes it on the next Telegram message. Wired by the
// watchdog + waker in a later Phase 3 slice.

#[allow(dead_code)]
fn dormant_marker_path() -> PathBuf {
    runguard_dir().join("dormant")
}

#[allow(dead_code)]
pub fn is_dormant_persisted() -> bool {
    dormant_marker_path().exists()
}

#[allow(dead_code)]
pub fn write_dormant_marker() -> std::io::Result<()> {
    let path = dormant_marker_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, "1")
}

#[allow(dead_code)]
pub fn clear_dormant_marker() {
    let _ = std::fs::remove_file(dormant_marker_path());
}

fn activated_marker_path() -> PathBuf {
    runguard_dir().join("activated")
}

pub fn is_activated_persisted() -> bool {
    activated_marker_path().exists()
}

pub fn write_activated_marker() -> std::io::Result<()> {
    let path = activated_marker_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, "1")
}

pub fn clear_activated_marker() {
    let _ = std::fs::remove_file(activated_marker_path());
}

fn credentials_ok_marker_path() -> PathBuf {
    runguard_dir().join("credentials-ok")
}

fn read_credentials_ok_ts() -> Option<u64> {
    std::fs::read_to_string(credentials_ok_marker_path())
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
}

pub fn write_credentials_ok_marker() -> std::io::Result<()> {
    let path = credentials_ok_marker_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, now_unix_ms().to_string())
}

pub fn clear_credentials_ok_marker() {
    let _ = std::fs::remove_file(credentials_ok_marker_path());
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

// ─── Perimeter teardown ───────────────────────────────────────────────

/// Tear the perimeter down on graceful exit. Synchronous — we want the
/// containers actually stopped before the process terminates so we don't
/// leak the running perimeter. 30s budget enforced by `timeout(1)`.
pub fn bring_perimeter_down_sync(root: &Path) {
    let _ = crate::orchestrator::podman::perimeter_down(root);
}

// ─── RunGuard (orphan reap on next launch after SIGKILL) ──────────────

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
                let _ = crate::orchestrator::podman::perimeter_down(root);
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

// ─── State compute ────────────────────────────────────────────────────

/// Store-state snapshot passed into the blocking compute function so it
/// can derive the full `(bootstrap, tenant)` pair without touching the
/// store's locks from a blocking thread.
struct ShellSnapshot {
    paused: bool,
    activated: bool,
    bootstrap_progress: Option<BootstrapProgress>,
}

fn compute_perimeter_status(snap: ShellSnapshot) -> PerimeterStatus {
    let containers: Vec<ContainerStatus> = PERIMETER_CONTAINERS
        .iter()
        .map(|&name| ContainerStatus {
            name: name.to_string(),
            running: is_service_running(name),
        })
        .collect();

    let shell_up = containers
        .iter()
        .filter(|c| c.name != "vault-agent")
        .all(|c| c.running);
    let agent_up = containers
        .iter()
        .find(|c| c.name == "vault-agent")
        .map_or(false, |c| c.running);

    let bootstrap = compute_bootstrap_state(shell_up, &snap.bootstrap_progress, snap.activated);
    let tenant = compute_tenant_state(agent_up, snap.paused, snap.activated, &snap.bootstrap_progress, &bootstrap);

    PerimeterStatus {
        bootstrap,
        tenant,
        containers,
        last_checked_unix_ms: now_unix_ms(),
    }
}

fn compute_bootstrap_state(
    shell_up: bool,
    progress: &Option<BootstrapProgress>,
    activated: bool,
) -> BootstrapState {
    match progress {
        // Self-heal: a stale Failed marker is overridden by a healthy live
        // probe. Containers may have recovered (compose restart-policy, manual
        // intervention, transient runtime hiccup) after bootstrap failed. The
        // watchdog clears the marker in the same tick.
        Some(BootstrapProgress::Failed { .. }) if shell_up => BootstrapState::ShellReady,
        Some(BootstrapProgress::Failed { .. }) => BootstrapState::ShellFailed,
        Some(BootstrapProgress::Step { .. }) => BootstrapState::Bootstrapping,
        None => {
            if shell_up {
                BootstrapState::ShellReady
            } else if activated {
                // Shell was previously working (activated marker present) but
                // now no shell containers are running → something failed.
                BootstrapState::ShellFailed
            } else {
                // No bootstrap in-flight, no shell containers, never activated →
                // first launch, still in initial setup.
                BootstrapState::Installing
            }
        }
    }
}

fn compute_tenant_state(
    agent_up: bool,
    paused: bool,
    activated: bool,
    progress: &Option<BootstrapProgress>,
    bootstrap: &BootstrapState,
) -> TenantState {
    // Tenant can only be non-Absent on a healthy shell.
    if !matches!(bootstrap, BootstrapState::ShellReady) {
        return TenantState::Absent;
    }

    if paused {
        return TenantState::Paused;
    }

    // Activating: bootstrap subsystem reports agent bring-up in flight.
    if let Some(BootstrapProgress::Step { step, .. }) = progress {
        if matches!(step, BootstrapStep::UpAgent) {
            return TenantState::Activating;
        }
    }

    if agent_up {
        return TenantState::Running;
    }

    if activated {
        // Was activated, not paused, shell ready, but agent isn't running.
        return TenantState::Errored;
    }

    TenantState::Absent
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
/// the tray + the hero state — they take action, not us.
pub fn spawn_watchdog(handle: AppHandle, interval: Duration) {
    tauri::async_runtime::spawn(async move {
        // First tick fires immediately (tokio::time::interval fires the first
        // tick at t=0, then every `interval` after).
        let mut ticker = tokio::time::interval(interval);
        let mut last_state: Option<(BootstrapState, TenantState)> = None;

        loop {
            ticker.tick().await;

            // Snapshot store-side fields before handing off to blocking thread.
            let snap = if let Some(store) = handle.try_state::<PerimeterStateStore>() {
                let progress = store
                    .bootstrap_progress
                    .read()
                    .ok()
                    .and_then(|g| g.clone());
                ShellSnapshot {
                    paused: store.is_paused(),
                    activated: store.is_activated(),
                    bootstrap_progress: progress,
                }
            } else {
                ShellSnapshot {
                    paused: is_paused_persisted(),
                    activated: is_activated_persisted(),
                    bootstrap_progress: None,
                }
            };

            // Run the synchronous status probe on a blocking thread so
            // we don't stall the tokio reactor on slow podman calls.
            let status =
                tokio::task::spawn_blocking(move || compute_perimeter_status(snap))
                    .await
                    .ok();
            let Some(status) = status else { continue };

            // Update the store + decide whether to emit + update tray.
            let pair = (status.bootstrap.clone(), status.tenant.clone());
            let state_changed = last_state.as_ref() != Some(&pair);
            last_state = Some(pair);

            if let Some(store) = handle.try_state::<PerimeterStateStore>() {
                if let Ok(mut guard) = store.status.lock() {
                    *guard = status.clone();
                }
                // Self-heal cleanup: if the live probe shows the shell back
                // up while a stale Failed progress marker is still in the
                // store, drop the marker so subsequent reads don't keep
                // surfacing failure details to the UI.
                if matches!(status.bootstrap, BootstrapState::ShellReady) {
                    if let Ok(mut g) = store.bootstrap_progress.write() {
                        if matches!(*g, Some(BootstrapProgress::Failed { .. })) {
                            *g = None;
                        }
                    }
                }
            }

            if state_changed {
                let _ = handle.emit("perimeter-state-changed", &status);
            }
            update_tray_for_state(&handle, &status.bootstrap, &status.tenant);
        }
    });
}

// Tray icons embedded at compile time — amber/green/red circle PNGs.
static TRAY_AMBER: &[u8] = include_bytes!("../icons/tray-amber.png");
static TRAY_GREEN: &[u8] = include_bytes!("../icons/tray-green.png");
static TRAY_RED:   &[u8] = include_bytes!("../icons/tray-red.png");

fn tray_icon_bytes(bootstrap: &BootstrapState, tenant: &TenantState) -> &'static [u8] {
    match (bootstrap, tenant) {
        (BootstrapState::ShellReady, TenantState::Running) => TRAY_GREEN,
        (BootstrapState::ShellFailed, _) | (BootstrapState::ShellReady, TenantState::Errored) => TRAY_RED,
        _ => TRAY_AMBER,
    }
}

fn tray_label_for(bootstrap: &BootstrapState, tenant: &TenantState) -> &'static str {
    match (bootstrap, tenant) {
        (BootstrapState::Installing, _) => "Assistant — setting up…",
        (BootstrapState::Bootstrapping, _) => "Assistant — setting up…",
        (BootstrapState::ShellFailed, _) => "Assistant — setup needs attention",
        (BootstrapState::ShellReady, TenantState::Absent) => "Assistant — ready to launch",
        (BootstrapState::ShellReady, TenantState::Activating) => "Assistant — starting…",
        (BootstrapState::ShellReady, TenantState::Running) => "Assistant — running safely",
        (BootstrapState::ShellReady, TenantState::Paused) => "Assistant — stopped",
        (BootstrapState::ShellReady, TenantState::Errored) => "Assistant — needs attention",
    }
}

fn update_tray_for_state(handle: &AppHandle, bootstrap: &BootstrapState, tenant: &TenantState) {
    let label = tray_label_for(bootstrap, tenant);
    let icon_bytes = tray_icon_bytes(bootstrap, tenant);
    if let Some(tray) = handle.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(label));
        if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(image));
        }
    }
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
            bootstrap: BootstrapState::ShellReady,
            tenant: TenantState::Running,
            containers: vec![ContainerStatus {
                name: "vault-agent".into(),
                running: true,
            }],
            last_checked_unix_ms: 1_000_000,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"shell_ready\""));
        assert!(json.contains("\"running\""));
        assert!(json.contains("\"vault-agent\""));
    }

    #[test]
    fn tray_label_covers_all_state_pairs() {
        let pairs = [
            (BootstrapState::Installing, TenantState::Absent),
            (BootstrapState::Bootstrapping, TenantState::Absent),
            (BootstrapState::ShellFailed, TenantState::Absent),
            (BootstrapState::ShellReady, TenantState::Absent),
            (BootstrapState::ShellReady, TenantState::Activating),
            (BootstrapState::ShellReady, TenantState::Running),
            (BootstrapState::ShellReady, TenantState::Paused),
            (BootstrapState::ShellReady, TenantState::Errored),
        ];
        for (bootstrap, tenant) in pairs {
            let label = tray_label_for(&bootstrap, &tenant);
            assert!(
                label.starts_with("Assistant"),
                "bad label for {bootstrap:?}/{tenant:?}"
            );
            assert!(!label.is_empty());
        }
    }

    #[test]
    fn compute_bootstrap_no_containers_not_activated_is_installing() {
        assert_eq!(
            compute_bootstrap_state(false, &None, false),
            BootstrapState::Installing
        );
    }

    #[test]
    fn compute_bootstrap_no_containers_but_activated_is_shell_failed() {
        assert_eq!(
            compute_bootstrap_state(false, &None, true),
            BootstrapState::ShellFailed
        );
    }

    #[test]
    fn compute_bootstrap_shell_up_is_shell_ready() {
        assert_eq!(
            compute_bootstrap_state(true, &None, false),
            BootstrapState::ShellReady
        );
    }

    #[test]
    fn compute_bootstrap_failed_progress_self_heals_when_shell_up() {
        let progress = Some(BootstrapProgress::Failed {
            cause: "image-build-failed".into(),
            message: "Build failed".into(),
            last_error: None,
        });
        // A live probe showing the shell back up overrides a stale Failed
        // marker — the watchdog will clear the marker on the same tick.
        assert_eq!(
            compute_bootstrap_state(true, &progress, true),
            BootstrapState::ShellReady
        );
    }

    #[test]
    fn compute_bootstrap_failed_progress_holds_when_shell_down() {
        let progress = Some(BootstrapProgress::Failed {
            cause: "image-build-failed".into(),
            message: "Build failed".into(),
            last_error: None,
        });
        assert_eq!(
            compute_bootstrap_state(false, &progress, true),
            BootstrapState::ShellFailed
        );
    }

    #[test]
    fn compute_bootstrap_step_in_flight_is_bootstrapping() {
        let progress = Some(BootstrapProgress::Step {
            step: BootstrapStep::BuildImages,
            step_index: 4,
            total_steps: 7,
            percent: None,
            detail: None,
            started_at_unix_ms: 0,
        });
        assert_eq!(
            compute_bootstrap_state(false, &progress, false),
            BootstrapState::Bootstrapping
        );
    }

    #[test]
    fn compute_tenant_paused_overrides_running_agent() {
        let tenant =
            compute_tenant_state(true, true, true, &None, &BootstrapState::ShellReady);
        assert_eq!(tenant, TenantState::Paused);
    }

    #[test]
    fn compute_tenant_not_shell_ready_is_absent() {
        let tenant =
            compute_tenant_state(true, false, true, &None, &BootstrapState::Installing);
        assert_eq!(tenant, TenantState::Absent);
    }

    #[test]
    fn compute_tenant_activated_no_agent_is_errored() {
        let tenant =
            compute_tenant_state(false, false, true, &None, &BootstrapState::ShellReady);
        assert_eq!(tenant, TenantState::Errored);
    }

    #[test]
    fn compute_tenant_not_activated_no_agent_is_absent() {
        let tenant =
            compute_tenant_state(false, false, false, &None, &BootstrapState::ShellReady);
        assert_eq!(tenant, TenantState::Absent);
    }

    #[test]
    fn compute_tenant_agent_running_not_paused_is_running() {
        let tenant =
            compute_tenant_state(true, false, true, &None, &BootstrapState::ShellReady);
        assert_eq!(tenant, TenantState::Running);
    }
}
