//! Streaming command execution (lifted from the Tauri `commands/stream.rs`, ADR-0022 migration).
//!
//! Runs a manifest command and emits its output line-by-line as `stream-line` events plus a final
//! `stream-end`, into the transport-neutral [`EventBus`] instead of `AppHandle::emit`. The
//! loopback viewer-server forwards those envelopes to its `/api/events` WebSocket; the Tauri layer
//! can bridge the same bus to `app.emit`. This is the high-frequency path the WS design was
//! justified by (spec §1.2).
//!
//! `active_streams` (component:command → child PID) lets a new run of the same command supersede an
//! old one and lets `stop_stream` cancel it — mirroring the Tauri `AppState.active_streams`.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Mutex;

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command as TokioCommand;

use crate::events::EventBus;
use crate::orchestrator::discovery::DiscoveredComponent;
use crate::orchestrator::error::OrchestratorError;
use crate::orchestrator::manifest::Command as ManifestCommand;
use crate::util::shell::find_bash;

/// component:command → child PID. Shared (behind a `Mutex`) so a restart supersedes and `stop_stream`
/// can cancel. Same shape as the Tauri backend's `AppState.active_streams`.
pub type ActiveStreams = Mutex<HashMap<String, u32>>;

/// One line of streamed output (the `stream-line` event payload). Snake-case fields match the
/// shape the frontend's `StreamLine` type + the Tauri backend's event already expect.
#[derive(Serialize)]
struct StreamLine {
    component_id: String,
    command_id: String,
    line: String,
    stream: &'static str,
}

/// The terminal `stream-end` event payload, carrying the process exit code.
#[derive(Serialize)]
struct StreamEnd {
    component_id: String,
    command_id: String,
    exit_code: i32,
}

/// Serialize an event payload to JSON. The structs above always serialize (no non-string keys, no
/// floats), so this never fails; `Null` on the impossible error keeps it panic-free.
fn payload(value: impl Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

/// Start streaming a manifest command's output as `stream-line` / `stream-end` events on `bus`.
/// Returns once the process is spawned and the reader/waiter tasks are running (the events arrive
/// asynchronously). A prior run of the same `component:command` is killed first (no double stream).
pub async fn start_stream(
    components: &[DiscoveredComponent],
    active_streams: &ActiveStreams,
    bus: &EventBus,
    component_id: String,
    command_id: String,
    args: &HashMap<String, String>,
) -> Result<(), OrchestratorError> {
    let (manifest_cmd, component_dir) = {
        let component = components
            .iter()
            .find(|c| c.manifest.identity.id == component_id)
            .ok_or_else(|| OrchestratorError::ComponentNotFound(component_id.clone()))?;
        let cmd = component
            .manifest
            .commands
            .iter()
            .find(|c| c.id == command_id)
            .ok_or_else(|| OrchestratorError::CommandNotFound {
                component: component_id.clone(),
                command: command_id.clone(),
            })?
            .clone();
        (cmd, component.component_dir.clone())
    };

    let bash =
        find_bash().ok_or_else(|| OrchestratorError::ShellNotFound("bash not found".to_string()))?;

    // Supersede any existing stream for this component:command before starting a new one.
    let key = format!("{component_id}:{command_id}");
    if let Some(old_pid) = active_streams.lock().unwrap().remove(&key) {
        kill_pid(old_pid);
    }

    let interpolated = interpolate_stream_args(&manifest_cmd, args);
    let mut child = TokioCommand::new(&bash)
        .arg("-c")
        .arg(&interpolated)
        .current_dir(&component_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(OrchestratorError::IoError)?;

    if let Some(pid) = child.id() {
        active_streams.lock().unwrap().insert(key, pid);
    }

    if let Some(stdout) = child.stdout.take() {
        spawn_line_reader(stdout, bus.clone(), component_id.clone(), command_id.clone(), "stdout");
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_line_reader(stderr, bus.clone(), component_id.clone(), command_id.clone(), "stderr");
    }

    // Waiter — emit `stream-end` with the exit code once the process finishes.
    let end_bus = bus.clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        let exit_code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        end_bus.emit("stream-end", payload(StreamEnd { component_id, command_id, exit_code }));
    });

    Ok(())
}

/// Stop a running stream (best-effort kill); a no-op if none is running for that key.
pub fn stop_stream(
    active_streams: &ActiveStreams,
    component_id: &str,
    command_id: &str,
) -> Result<(), OrchestratorError> {
    let key = format!("{component_id}:{command_id}");
    let pid = active_streams.lock().unwrap().remove(&key);
    if let Some(pid) = pid {
        kill_pid(pid);
    }
    Ok(())
}

/// Read `reader` line-by-line, emitting each as a `stream-line` event tagged with the stream name.
fn spawn_line_reader<R: AsyncRead + Unpin + Send + 'static>(
    reader: R,
    bus: EventBus,
    component_id: String,
    command_id: String,
    stream: &'static str,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let event = StreamLine {
                component_id: component_id.clone(),
                command_id: command_id.clone(),
                line,
                stream,
            };
            bus.emit("stream-line", payload(event));
        }
    });
}

fn kill_pid(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill").arg(pid.to_string()).output();
    }
}

/// Interpolate `${name}` placeholders in the command with single-quote-escaped arg values.
fn interpolate_stream_args(cmd: &ManifestCommand, args: &HashMap<String, String>) -> String {
    let mut result = cmd.command.clone();
    for (key, value) in args {
        let safe_value = format!("'{}'", value.replace('\'', "'\\''"));
        result = result.replace(&format!("${{{key}}}"), &safe_value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::orchestrator::manifest::Manifest;

    /// Build a one-command synthetic component whose `echo` command runs `command_body`.
    fn echo_component(command_body: &str) -> DiscoveredComponent {
        let yaml = format!(
            "identity:\n  id: test\n  name: Test\n  version: '1.0'\n  description: t\n  role: placeholder\ncommands:\n  - id: echo\n    name: Echo\n    command: \"{command_body}\"\n",
        );
        let manifest: Manifest = serde_yaml::from_str(&yaml).expect("minimal manifest parses");
        DiscoveredComponent {
            manifest,
            component_dir: std::env::temp_dir().to_string_lossy().to_string(),
        }
    }

    #[tokio::test]
    async fn start_stream_runs_a_real_command_and_emits_each_line_then_end() {
        let components = vec![echo_component("printf 'one\\ntwo\\n'")];
        let active = ActiveStreams::default();
        let bus = EventBus::new();
        let mut rx = bus.subscribe(); // subscribe BEFORE starting (broadcast delivers from-now-on)

        start_stream(&components, &active, &bus, "test".into(), "echo".into(), &HashMap::new())
            .await
            .expect("spawns");

        let mut lines = Vec::new();
        let exit = loop {
            let env = tokio::time::timeout(Duration::from_secs(10), rx.recv())
                .await
                .expect("an event within the timeout")
                .expect("bus open");
            match env.event.as_str() {
                "stream-line" => lines.push(env.payload["line"].as_str().unwrap().to_string()),
                "stream-end" => break env.payload["exit_code"].as_i64().unwrap(),
                _ => {}
            }
        };
        assert_eq!(lines, vec!["one", "two"], "each stdout line is a stream-line event");
        assert_eq!(exit, 0, "stream-end carries the exit code");
    }

    #[tokio::test]
    async fn start_stream_unknown_component_is_not_found() {
        let active = ActiveStreams::default();
        let bus = EventBus::new();
        let err = start_stream(&[], &active, &bus, "nope".into(), "x".into(), &HashMap::new())
            .await
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::ComponentNotFound(_)));
    }

    #[test]
    fn interpolate_escapes_single_quotes_to_prevent_injection() {
        let yaml = "identity:\n  id: t\n  name: T\n  version: '1'\n  description: d\n  role: placeholder\ncommands:\n  - id: c\n    name: C\n    command: \"echo ${msg}\"\n";
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        let mut args = HashMap::new();
        args.insert("msg".to_string(), "a'; rm -rf /".to_string());
        let out = interpolate_stream_args(&m.commands[0], &args);
        assert_eq!(out, "echo 'a'\\''; rm -rf /'");
    }
}
