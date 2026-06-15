//! Boundary self-test runner (road-to-recommendable §1A/§1B, task #45).
//!
//! The daemon must prove that a *resumed* perimeter passes the SAME boundary
//! checks as a cold one — a boundary that is "alive but subtly wrong" after a
//! resume is worse than a visible failure (CLAUDE.md §11). This module embeds
//! `tests/boundary-selftest.sh` into the daemon binary, stages it to the data
//! dir at runtime, and runs it against the live perimeter. The wiring into the
//! supervisor's (re)start paths is **opt-in** (`OPENTRAPP_SELFTEST_ON_RESUME`,
//! default OFF) so shipping behavior is unchanged until the script is verified on
//! capable hardware — mirroring the `OPENTRAPP_DAEMON_DEFER` gating (ADR-0019).
//!
//! Fail-closed semantics come from the script's exit code: 0 = all boundaries
//! hold, 1 = a boundary FAILED (hold the perimeter closed + alert), 2 = could
//! not assess (unverifiable ≠ verified — alert, but do not tear down a perimeter
//! we simply couldn't measure).

use std::path::{Path, PathBuf};

/// The boundary self-test, embedded at compile time so the daemon is
/// self-contained. **Vendored copy** of `tests/boundary-selftest.sh` — the
/// canonical script `make boundary-selftest` runs. The copy lives INSIDE this
/// crate so `opentrapp-core` is crates.io-publishable (ADR-0023); an
/// `include_str!` from outside the crate is not packageable. A drift-check in
/// `tests/orchestrator-check.sh` keeps the two byte-identical (run
/// `make sync-core-embedded` after editing the canonical).
pub const SCRIPT: &str = include_str!("embedded/boundary-selftest.sh");

/// Opt-in: run the boundary self-test after every (re)start, fail-closed.
/// Default OFF — until the script runs green on capable hardware, shipping
/// behavior is unchanged (§11). Set `OPENTRAPP_SELFTEST_ON_RESUME=1` to enable.
pub fn on_resume_enabled() -> bool {
    matches!(
        std::env::var("OPENTRAPP_SELFTEST_ON_RESUME").ok().as_deref(),
        Some("1") | Some("true")
    )
}

/// Stage the embedded script into `<data_dir>/boundary/boundary-selftest.sh`
/// (0700 on unix) and return its path.
pub fn stage(data_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = data_dir.join("boundary");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("boundary-selftest.sh");
    std::fs::write(&path, SCRIPT)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

/// The outcome of a boundary self-test run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Exit 0 — every boundary holds.
    Pass,
    /// Exit 1 (or a signal / unknown code) — a boundary FAILED; hold fail-closed.
    Fail,
    /// Exit 2 — could not assess (perimeter down / tool missing). Not a pass.
    CannotAssess,
}

impl Verdict {
    /// Map the script's process exit code to a verdict. Anything that is not a
    /// clean 0 or an explicit 2 is treated as a failure (fail-closed): a killed
    /// or crashed self-test is not evidence the boundary holds.
    pub fn from_exit(code: Option<i32>) -> Verdict {
        match code {
            Some(0) => Verdict::Pass,
            Some(2) => Verdict::CannotAssess,
            _ => Verdict::Fail,
        }
    }
}

/// Stage + run the boundary self-test against the live perimeter. Blocking
/// (spawns `bash`) — call from `spawn_blocking`. Returns the verdict plus the
/// combined stdout/stderr for logging/alerting.
pub fn run_blocking(data_dir: &Path) -> (Verdict, String) {
    let path = match stage(data_dir) {
        Ok(p) => p,
        Err(e) => return (Verdict::CannotAssess, format!("could not stage self-test: {e}")),
    };
    match std::process::Command::new("bash")
        .arg(&path)
        .env("OPENTRAPP_DATA_DIR", data_dir)
        .output()
    {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (Verdict::from_exit(o.status.code()), s)
        }
        Err(e) => (Verdict::CannotAssess, format!("could not spawn bash: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_script_is_the_boundary_selftest() {
        assert!(SCRIPT.starts_with("#!/usr/bin/env bash"));
        // A few anchors so a silent divergence from the repo copy is caught.
        assert!(SCRIPT.contains("OpenTrApp Boundary Self-Test"));
        assert!(SCRIPT.contains("B4-l3-egress"));
        assert!(SCRIPT.contains("vault_egress_drop_private"));
    }

    #[test]
    fn exit_code_maps_fail_closed() {
        assert_eq!(Verdict::from_exit(Some(0)), Verdict::Pass);
        assert_eq!(Verdict::from_exit(Some(2)), Verdict::CannotAssess);
        assert_eq!(Verdict::from_exit(Some(1)), Verdict::Fail);
        assert_eq!(Verdict::from_exit(None), Verdict::Fail); // killed by signal → fail-closed
        assert_eq!(Verdict::from_exit(Some(137)), Verdict::Fail);
    }

    #[test]
    fn opt_in_defaults_off() {
        // The wiring must be inert unless explicitly enabled (§11).
        std::env::remove_var("OPENTRAPP_SELFTEST_ON_RESUME");
        assert!(!on_resume_enabled());
    }

    #[test]
    fn stage_writes_executable_copy() {
        let d = std::env::temp_dir()
            .join(format!("opentrapp-selftest-stage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let p = stage(&d).expect("stage");
        let body = std::fs::read_to_string(&p).expect("read staged");
        assert_eq!(body, SCRIPT);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&p).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}
