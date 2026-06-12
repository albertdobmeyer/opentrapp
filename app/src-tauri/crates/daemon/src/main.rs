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
//!   `--status`         print the durable perimeter state and exit
//!   `--selftest`       exercise the marker contract end-to-end, exit 0/1
//!   `--help`           usage

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use opentrapp_core::control::{self, ControlRequest};
use opentrapp_core::{markers, runguard, supervisor};
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--selftest") {
        return selftest();
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

    eprintln!(
        "opentrapp-daemon: owning the perimeter (idle threshold {} min)",
        supervisor::IDLE_TIMEOUT_MS_DEFAULT / 60_000
    );
    supervisor::run(data_dir.clone(), supervisor::IDLE_TIMEOUT_MS_DEFAULT, shutdown).await;

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

fn print_help() {
    println!("opentrapp-daemon (Phase B / ADR-0019)");
    println!("  (no args)    own + supervise the perimeter until SIGTERM/SIGINT");
    println!("  pause|resume|restart|shutdown");
    println!("               queue a control request for the running daemon");
    println!("  --status     print durable perimeter state + runguard owner");
    println!("  --selftest   exercise the marker contract end-to-end, exit 0/1");
    println!("  --help       this message");
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
