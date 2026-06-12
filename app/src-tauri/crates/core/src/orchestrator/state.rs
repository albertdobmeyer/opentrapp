use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, RwLock};

use super::discovery::DiscoveredComponent;

/// Application state managed by Tauri
pub struct AppState {
    /// The user's runtime data home (`~/.opentrapp/`) — where `.env`, marker
    /// files, and the verified `perimeter/` resources live. Replaces the old
    /// `monorepo_root`, which assumed the app ran from a source clone.
    pub runtime_data_dir: RwLock<PathBuf>,
    pub components: Mutex<Vec<DiscoveredComponent>>,
    pub component_states: Mutex<HashMap<String, String>>,
    pub active_streams: Mutex<HashMap<String, u32>>, // component:command -> child PID
    /// Pending idle-stop timers for on-demand shields (e.g. `vault-skills`),
    /// keyed by service name. A command (re)arms the timer; when it fires it
    /// stops the container. Re-arming aborts the previous handle so bursts of
    /// commands keep the container warm. See `commands/execute.rs`.
    pub idle_stops: Mutex<HashMap<String, tokio::task::AbortHandle>>,
    /// The host-side wake-on-message waker, present only while the perimeter is
    /// dormant (idle auto-pause). Holds its cancellation signal + task handle so
    /// an external resume can stop and await it before bringing the perimeter
    /// back up. See `crate::idle` and ADR-0018.
    pub waker: Mutex<Option<crate::idle::IdleWaker>>,
    /// Set to `true` by an EXPLICIT quit (tray Quit, SIGTERM/SIGINT) before
    /// `exit(0)`. The `RunEvent::ExitRequested` handler reads it: when `false`
    /// it `prevent_exit()`s so the daemon survives the dashboard window being
    /// closed/destroyed (the lean tray-only resting state). Only an explicit
    /// quit lets `RunEvent::Exit` fire and tear the perimeter down. See lib.rs.
    pub quitting: AtomicBool,
    /// `true` when a headless `opentrapp-daemon` owns the perimeter and this GUI
    /// is acting as a viewer (Phase B / B4b, ADR-0019): the GUI then skips
    /// establishing the RunGuard, bringing the perimeter up, idle auto-pause, and
    /// teardown-on-exit, and routes perimeter-mutating commands through the
    /// control channel. Default `false` (the GUI self-owns, exactly as before);
    /// only set true when the opt-in defer actually launches/finds a daemon.
    pub daemon_owned: AtomicBool,
}

impl AppState {
    pub fn new(runtime_data_dir: PathBuf) -> Self {
        Self {
            runtime_data_dir: RwLock::new(runtime_data_dir),
            components: Mutex::new(Vec::new()),
            component_states: Mutex::new(HashMap::new()),
            active_streams: Mutex::new(HashMap::new()),
            idle_stops: Mutex::new(HashMap::new()),
            waker: Mutex::new(None),
            quitting: AtomicBool::new(false),
            daemon_owned: AtomicBool::new(false),
        }
    }
}
