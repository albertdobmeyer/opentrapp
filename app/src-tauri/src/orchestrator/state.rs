use std::collections::HashMap;
use std::path::PathBuf;
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
}

impl AppState {
    pub fn new(runtime_data_dir: PathBuf) -> Self {
        Self {
            runtime_data_dir: RwLock::new(runtime_data_dir),
            components: Mutex::new(Vec::new()),
            component_states: Mutex::new(HashMap::new()),
            active_streams: Mutex::new(HashMap::new()),
            idle_stops: Mutex::new(HashMap::new()),
        }
    }
}
