//! opentrapp-daemon — the headless perimeter daemon (Phase B, ADR-0019).
//!
//! **Slice B3.** The daemon now *owns* the perimeter rather than merely
//! reporting it: in its default `run` mode it takes the RunGuard, brings the
//! perimeter up, supervises it (idle → dormant + arm the wake-on-message waker,
//! ADR-0018), and tears it down cleanly on SIGTERM/SIGINT. It links ONLY
//! `opentrapp-core` + `tokio` — no tauri/wry/webkit (CI asserts the dependency
//! graph is WebKit-free). Later: B4 adds the control socket and the Tauri app
//! becomes an on-demand viewer that defers perimeter ownership to this daemon.
//!
//! Modes:
//!   (default / `run`)  own + supervise the perimeter until signalled
//!   `vault <verb>`     friendly CLI surface: up|down|status|verify|pause|resume|restart
//!   `--status`         print the durable perimeter state and exit
//!   `--selftest`       exercise the marker contract end-to-end, exit 0/1
//!   `--help`           usage

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use opentrapp_core::control::{self, ControlRequest};
use opentrapp_core::{markers, runguard, supervisor};
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Friendly `vault <verb>` CLI surface (CLI-first; ADR-0020/0024) — a thin alias
    // layer over the same operations as the engine flags/verbs below. The bare
    // `opentrapp` command + GUI demotion lands in Phase 3 (de-Tauri); until then the
    // operator CLI is `opentrapp-daemon vault <verb>`. Checked before the global
    // `--help` scan so `vault --help` reaches the vault help, not the top-level one.
    if args.first().map(String::as_str) == Some("vault") {
        return dispatch_vault(args.get(1).map(String::as_str)).await;
    }
    // `configure` — open the on-demand web control panel. SPAWNS the viewer-server as a separate
    // process; the always-on daemon never links it / exposes a network surface (CLAUDE.md §10).
    if args.first().map(String::as_str) == Some("configure") {
        return configure();
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--selftest") {
        return selftest();
    }
    if args.iter().any(|a| a == "--boundary-selftest") {
        return boundary_selftest();
    }
    if args.iter().any(|a| a == "--status") {
        return print_status();
    }
    // Control verbs: queue a request for the running daemon, then exit.
    if let Some(req) = args.first().and_then(|a| ControlRequest::from_token(a)) {
        return submit_control(req);
    }
    run().await
}

/// Queue a control request into the running daemon's inbox + exit.
fn submit_control(req: ControlRequest) -> ExitCode {
    let data_dir = markers::default_data_dir();
    match control::submit(&data_dir, req) {
        Ok(()) => {
            println!("opentrapp-daemon: queued '{}' for the running daemon", req.as_token());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("opentrapp-daemon: failed to queue '{}': {e}", req.as_token());
            ExitCode::FAILURE
        }
    }
}

/// Own + supervise the perimeter until a shutdown signal arrives.
async fn run() -> ExitCode {
    let data_dir = markers::default_data_dir();

    if let Some(pid) = runguard::held_by_other(&data_dir) {
        eprintln!(
            "opentrapp-daemon: another owner (pid={pid}) already holds the perimeter — refusing to start"
        );
        return ExitCode::FAILURE;
    }
    runguard::establish(&data_dir);

    let shutdown = Arc::new(Notify::new());
    {
        let s = shutdown.clone();
        tokio::spawn(async move {
            wait_for_shutdown().await;
            eprintln!("opentrapp-daemon: shutdown signal received");
            s.notify_one();
        });
    }

    let idle_ms = supervisor::idle_threshold_ms();
    eprintln!(
        "opentrapp-daemon: owning the perimeter (idle threshold {} s)",
        idle_ms / 1000
    );
    supervisor::run(data_dir.clone(), idle_ms, shutdown).await;

    runguard::clear(&data_dir);
    eprintln!("opentrapp-daemon: perimeter down, exiting cleanly");
    ExitCode::SUCCESS
}

/// Block until SIGTERM/SIGINT (Unix) or Ctrl-C (otherwise).
async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = term.recv() => {}
                    _ = tokio::signal::ctrl_c() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn print_status() -> ExitCode {
    let data_dir = markers::default_data_dir();
    let s = markers::snapshot(&data_dir);
    let owner = runguard::held_by_other(&data_dir);
    println!("opentrapp-daemon status  data_dir={}", data_dir.display());
    println!(
        "  activated={} paused={} dormant={} credentials_ok={}",
        s.activated, s.paused, s.dormant, s.credentials_ok
    );
    match owner {
        Some(pid) => println!("  runguard: held by another live owner (pid={pid})"),
        None => println!("  runguard: free (no live owner)"),
    }
    ExitCode::SUCCESS
}

/// Friendly `vault <verb>` surface mapping to the engine operations. `up` owns +
/// supervises the perimeter (long-running); the rest are one-shot. This is the
/// CLI-first projection (ADR-0020/0024) over the same `opentrapp-core` calls the
/// engine flags use — no new perimeter logic.
async fn dispatch_vault(verb: Option<&str>) -> ExitCode {
    match verb {
        Some("up") => run().await,
        Some("down") => submit_control(ControlRequest::Shutdown),
        Some("pause") => submit_control(ControlRequest::Pause),
        Some("resume") => submit_control(ControlRequest::Resume),
        Some("restart") => submit_control(ControlRequest::Restart),
        Some("status") => print_status(),
        Some("verify") => boundary_selftest(),
        Some("--help") | Some("-h") | None => {
            print_vault_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("opentrapp vault: unknown verb '{other}' (try `vault --help`)");
            print_vault_help();
            ExitCode::FAILURE
        }
    }
}

fn print_vault_help() {
    println!("opentrapp vault — control the containment perimeter (the Vault)");
    println!("  up        bring the perimeter up and supervise it (idle auto-pause + wake)");
    println!("  down      tear the perimeter down (stop owning it)");
    println!("  status    print durable perimeter state + the current owner");
    println!("  verify    run the live boundary self-test now (0 hold / 1 fail / 2 can't assess)");
    println!("  pause     pause the running perimeter");
    println!("  resume    resume a paused perimeter");
    println!("  restart   restart the perimeter");
    println!();
    println!("  Invoked via the headless daemon today: `opentrapp-daemon vault <verb>`.");
    println!("  The bare `opentrapp` command arrives with the GUI demotion (ADR-0022 / Phase 3).");
}

/// `opentrapp configure` — open the on-demand web control panel (ADR-0022 §2.3 / step 4).
///
/// Spawns the `viewer-server` as a SEPARATE process and waits on it. The always-on daemon NEVER
/// links the viewer-server or exposes a network service (CLAUDE.md §10): the config surface is a
/// transient, loopback-only, token-gated child that exists ONLY while you configure — started here
/// on explicit user action, and torn down when you close it (Ctrl-C reaches the child via the shared
/// foreground process group). The viewer-server opens your browser to the loopback URL itself.
///
/// This adds NO axum/network dependency to the daemon — it only execs a sibling binary.
fn configure() -> ExitCode {
    let override_bin = std::env::var_os("OPENTRAPP_VIEWER_SERVER_BIN").map(PathBuf::from);
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let bin = match resolve_viewer_bin(override_bin, exe_dir.as_deref()) {
        Some(b) => b,
        None => {
            eprintln!(
                "opentrapp configure: couldn't find the viewer-server binary next to the daemon.\n\
                 Install it alongside the daemon, or set OPENTRAPP_VIEWER_SERVER_BIN to its path."
            );
            return ExitCode::FAILURE;
        }
    };
    eprintln!("opentrapp configure: opening the web control panel (Ctrl-C to close)…");
    // Spawn + wait. The child inherits our env (so OPENTRAPP_VIEWER_DIST passes through) + stdio, and
    // shares the foreground process group, so Ctrl-C stops both.
    match std::process::Command::new(&bin).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("opentrapp configure: failed to start the viewer-server ({e})");
            ExitCode::FAILURE
        }
    }
}

/// Resolve the `viewer-server` binary: an explicit `OPENTRAPP_VIEWER_SERVER_BIN` override wins;
/// otherwise look for it next to this daemon binary (the install layout ships them side by side).
/// Pure (takes its inputs) so the resolution is unit-testable without touching the global env.
fn resolve_viewer_bin(
    override_path: Option<PathBuf>,
    daemon_exe_dir: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(p) = override_path {
        return Some(p);
    }
    let name = if cfg!(windows) { "viewer-server.exe" } else { "viewer-server" };
    let sibling = daemon_exe_dir?.join(name);
    sibling.is_file().then_some(sibling)
}

fn print_help() {
    println!("opentrapp-daemon (Phase B / ADR-0019)");
    println!("  vault <verb> friendly CLI: up|down|status|verify|pause|resume|restart");
    println!("               (see `opentrapp-daemon vault --help`)");
    println!("  configure    open the on-demand web control panel in your browser");
    println!("               (spawns the loopback viewer-server; Ctrl-C to close)");
    println!("  (no args)    own + supervise the perimeter until SIGTERM/SIGINT");
    println!("  pause|resume|restart|shutdown");
    println!("               queue a control request for the running daemon");
    println!("  --status     print durable perimeter state + runguard owner");
    println!("  --selftest   exercise the marker contract end-to-end, exit 0/1");
    println!("  --boundary-selftest");
    println!("               run the live boundary self-test now (exit 0 hold / 1 fail / 2 can't assess)");
    println!("  --help       this message");
}

/// Run the boundary self-test once against the live perimeter and exit with its
/// verdict (road-to-recommendable §1A, task #45). Unlike the always-on resume
/// check this is on-demand and does NOT tear the perimeter down — it just
/// reports, so an operator can verify a cold or resumed boundary by hand.
fn boundary_selftest() -> ExitCode {
    let data_dir = markers::default_data_dir();
    let (verdict, output) = opentrapp_core::selftest::run_blocking(&data_dir);
    print!("{output}");
    match verdict {
        opentrapp_core::selftest::Verdict::Pass => {
            println!("opentrapp-daemon: boundary self-test PASS");
            ExitCode::SUCCESS
        }
        opentrapp_core::selftest::Verdict::CannotAssess => {
            eprintln!("opentrapp-daemon: boundary self-test could not assess");
            ExitCode::from(2)
        }
        opentrapp_core::selftest::Verdict::Fail => {
            eprintln!("opentrapp-daemon: boundary self-test FAILED");
            ExitCode::FAILURE
        }
    }
}

/// Exercise the marker contract round-trip in a temp dir (no perimeter needed).
fn selftest() -> ExitCode {
    let dir: PathBuf = std::env::temp_dir()
        .join(format!("opentrapp-daemon-selftest-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let ok = (|| -> std::io::Result<bool> {
        let before = markers::snapshot(&dir);
        markers::set_flag(&dir, markers::DORMANT)?;
        let after = markers::snapshot(&dir);
        markers::clear(&dir, markers::DORMANT);
        let cleared = markers::snapshot(&dir);
        Ok(!before.dormant && after.dormant && !cleared.dormant)
    })()
    .unwrap_or(false);

    let _ = std::fs::remove_dir_all(&dir);

    if ok {
        println!("opentrapp-daemon selftest: marker contract OK");
        ExitCode::SUCCESS
    } else {
        eprintln!("opentrapp-daemon selftest: marker contract FAILED");
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_viewer_bin_prefers_the_explicit_override() {
        let got = resolve_viewer_bin(Some(PathBuf::from("/opt/custom/viewer-server")), None);
        assert_eq!(got, Some(PathBuf::from("/opt/custom/viewer-server")));
    }

    #[test]
    fn resolve_viewer_bin_finds_the_sibling_next_to_the_daemon() {
        let dir = std::env::temp_dir().join(format!("otd-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let name = if cfg!(windows) { "viewer-server.exe" } else { "viewer-server" };
        let sibling = dir.join(name);
        std::fs::write(&sibling, b"binary").unwrap();
        assert_eq!(resolve_viewer_bin(None, Some(&dir)), Some(sibling));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_viewer_bin_is_none_without_override_or_sibling() {
        let dir = std::env::temp_dir().join(format!("otd-cfg-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(resolve_viewer_bin(None, Some(&dir)), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
