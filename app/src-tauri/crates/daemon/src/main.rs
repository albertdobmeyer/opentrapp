//! opentrapp-daemon — the headless perimeter daemon (Phase B, ADR-0019).
//!
//! **Slice B1 scaffold.** This binary links ONLY `opentrapp-core` — no
//! tauri/wry/webkit (CI asserts the dependency graph is WebKit-free, which is
//! the entire point of the split). Today it reports the durable perimeter state
//! from `~/.opentrapp` and can self-test the marker contract end-to-end. Later
//! slices give it ownership of the perimeter lifecycle, the 30 s watchdog, the
//! idle auto-pause waker (ADR-0018), bootstrap, the RunGuard, and the control
//! socket; the Tauri app becomes an on-demand viewer over this same state.

use std::path::PathBuf;
use std::process::ExitCode;

use opentrapp_core::markers;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--selftest") {
        return selftest();
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("opentrapp-daemon (Phase B / ADR-0019, slice B1 scaffold)");
        println!("  (no args)    report durable perimeter state from ~/.opentrapp");
        println!("  --selftest   exercise the marker contract end-to-end, exit 0/1");
        return ExitCode::SUCCESS;
    }

    let data_dir = markers::default_data_dir();
    let s = markers::snapshot(&data_dir);
    println!(
        "opentrapp-daemon (B1 scaffold)  data_dir={}",
        data_dir.display()
    );
    println!(
        "  activated={} paused={} dormant={} credentials_ok={}",
        s.activated, s.paused, s.dormant, s.credentials_ok
    );
    ExitCode::SUCCESS
}

/// Exercise the marker contract round-trip in a temp dir (no perimeter needed),
/// so the daemon's dependence on core is verified at the consumption end, not
/// merely that it links. Returns non-zero on any contract violation.
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
