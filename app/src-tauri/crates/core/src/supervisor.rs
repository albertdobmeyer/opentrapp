//! Headless perimeter supervision for the daemon (Phase B, ADR-0019).
//!
//! This is where the daemon stops *reporting* state and starts *owning* it: it
//! brings the perimeter up, watches for idle, drops to dormant + arms the
//! wake-on-message waker (ADR-0018), and tears the perimeter down on shutdown.
//! It reuses the orchestration core moved in B2 (podman, markers, idle) — no
//! tauri, no AppHandle. The AppHandle/settings dependency of the GUI watchdog
//! becomes a plain `threshold_ms` parameter here.
//!
//! The idle *decision* is pure + unit-tested. The run *loop* drives real
//! containers, so it is exercised on capable hardware, not in CI (§11).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use crate::control::ControlRequest;
use crate::idle::IdleWaker;

/// Idle-to-dormant threshold default (mirrors lifecycle.rs IDLE_TIMEOUT_MS_DEFAULT).
pub const IDLE_TIMEOUT_MS_DEFAULT: u64 = 12 * 60 * 1000;
/// Supervision tick (mirrors the GUI watchdog's 30 s cadence).
pub const TICK: Duration = Duration::from_secs(30);

/// Pure decision: should the perimeter auto-pause to dormant right now?
///
/// Pauses only when it is safe to wake again: the agent has been quiet past the
/// threshold AND a bot token exists (the waker needs it — never strand the
/// perimeter dormant with no way back). `last_activity_ms == None` (no activity
/// signal) and `dormant == true` (already asleep) never pause.
pub fn should_auto_pause(
    last_activity_ms: Option<u64>,
    threshold_ms: u64,
    dormant: bool,
    have_token: bool,
) -> bool {
    if dormant || !have_token {
        return false;
    }
    matches!(last_activity_ms, Some(ms) if ms >= threshold_ms)
}

async fn perimeter_up(data_dir: PathBuf) {
    // Stage the signed perimeter image tarballs (download from the release; the
    // BundleVerifier then digest-checks them on load). The GUI does this in its
    // bootstrap, but the headless daemon bring-up must too — otherwise a clean
    // machine has no tars for `podman load` and the perimeter never comes up
    // (#76, the CLI-first first-run gap). `fetch_perimeter_images` is idempotent:
    // a no-op once the tars are present, for external images (pulled by the
    // verifier, not here), or in dev (no bundled overlay).
    if let Err(e) = crate::orchestrator::podman::fetch_perimeter_images().await {
        eprintln!("[supervisor] perimeter image staging failed, not bringing up: {e}");
        return; // fail-closed: never run an un-staged / unverified perimeter
    }
    let _ = tokio::task::spawn_blocking(move || {
        crate::orchestrator::podman::perimeter_up(&data_dir)
    })
    .await;
}

async fn perimeter_down(data_dir: PathBuf) {
    let _ = tokio::task::spawn_blocking(move || {
        crate::orchestrator::podman::perimeter_down(&data_dir)
    })
    .await;
}

async fn pause_to_dormant(data_dir: &PathBuf) {
    let _ = crate::markers::set_flag(data_dir, crate::markers::DORMANT);
    let d = data_dir.clone();
    let _ = tokio::task::spawn_blocking(move || {
        crate::orchestrator::podman::perimeter_stop(&d)
    })
    .await;
}

/// Drop to dormant + arm the waker (no-op if already dormant). Shared by the
/// idle path and an explicit Pause control request.
async fn arm_pause(data_dir: &PathBuf, waker: &mut Option<IdleWaker>) {
    if crate::markers::is_set(data_dir, crate::markers::DORMANT) {
        return;
    }
    eprintln!("[supervisor] pausing to dormant");
    pause_to_dormant(data_dir).await;
    if let Some(old) = waker.take() {
        old.cancel();
    }
    *waker = crate::idle::spawn(data_dir.clone());
}

/// Explicit user pause: stop containers + set the PAUSED marker, with NO waker
/// (stays paused until an explicit Resume — distinct from idle DORMANT, which
/// auto-wakes on the next Telegram message). Routed from the GUI's
/// `pause_perimeter` when a daemon owns the perimeter.
async fn user_pause(data_dir: &PathBuf, waker: &mut Option<IdleWaker>) {
    if let Some(w) = waker.take() {
        w.cancel();
    }
    if crate::markers::is_set(data_dir, crate::markers::PAUSED) {
        return; // already paused
    }
    let _ = crate::markers::set_flag(data_dir, crate::markers::PAUSED);
    let d = data_dir.clone();
    let _ = tokio::task::spawn_blocking(move || {
        crate::orchestrator::podman::perimeter_stop(&d)
    })
    .await;
    eprintln!("[supervisor] paused (user)");
}

/// Wake from dormant OR user-pause. Stops the waker BEFORE bringing up (so the
/// agent's poller never overlaps it — ADR-0018), clears both markers, and brings
/// the perimeter up if it was asleep/paused.
async fn resume_now(data_dir: &PathBuf, waker: &mut Option<IdleWaker>) {
    if let Some(w) = waker.take() {
        w.cancel();
    }
    let was_down = crate::markers::is_set(data_dir, crate::markers::DORMANT)
        || crate::markers::is_set(data_dir, crate::markers::PAUSED);
    crate::markers::clear(data_dir, crate::markers::DORMANT);
    crate::markers::clear(data_dir, crate::markers::PAUSED);
    if !was_down {
        return; // already awake
    }
    perimeter_up(data_dir.clone()).await;
    verify_boundary_fail_closed(data_dir).await;
    eprintln!("[supervisor] resumed");
}

/// Bring the perimeter down and back up (clears dormant + stops the waker).
async fn restart_now(data_dir: &PathBuf, waker: &mut Option<IdleWaker>) {
    if let Some(w) = waker.take() {
        w.cancel();
    }
    crate::markers::clear(data_dir, crate::markers::DORMANT);
    perimeter_down(data_dir.clone()).await;
    perimeter_up(data_dir.clone()).await;
    verify_boundary_fail_closed(data_dir).await;
    eprintln!("[supervisor] restarted");
}

/// After a (re)start, prove the boundary holds — a resumed boundary that is
/// "alive but subtly wrong" must not serve traffic (road-to-recommendable §1B,
/// CLAUDE.md §11). **Opt-in**: inert unless `OPENTRAPP_SELFTEST_ON_RESUME=1`, so
/// shipping behavior is unchanged until the script is hardware-verified (§11,
/// mirrors `OPENTRAPP_DAEMON_DEFER`).
///
/// Fail-closed on the script's verdict: `Fail` → stop the perimeter + raise the
/// `BOUNDARY_FAILED` alert (a half-built boundary serves nothing); `CannotAssess`
/// → alert but leave it up (couldn't measure ≠ failed); `Pass` → clear the alert.
async fn verify_boundary_fail_closed(data_dir: &PathBuf) {
    if !crate::selftest::on_resume_enabled() {
        return;
    }
    let d = data_dir.clone();
    let (verdict, output) = tokio::task::spawn_blocking(move || crate::selftest::run_blocking(&d))
        .await
        .unwrap_or((
            crate::selftest::Verdict::Fail,
            "boundary self-test task panicked".to_string(),
        ));
    match verdict {
        crate::selftest::Verdict::Pass => {
            crate::markers::clear(data_dir, crate::markers::BOUNDARY_FAILED);
            eprintln!("[supervisor] boundary self-test PASS");
        }
        crate::selftest::Verdict::CannotAssess => {
            let _ = crate::markers::set(data_dir, crate::markers::BOUNDARY_FAILED, "cannot-assess");
            eprintln!("[supervisor] boundary self-test could NOT assess — alerting, leaving up\n{output}");
        }
        crate::selftest::Verdict::Fail => {
            let _ =
                crate::markers::set(data_dir, crate::markers::BOUNDARY_FAILED, "boundary-failed");
            eprintln!(
                "[supervisor] boundary self-test FAILED — holding the perimeter closed (fail-closed)\n{output}"
            );
            let d = data_dir.clone();
            let _ =
                tokio::task::spawn_blocking(move || crate::orchestrator::podman::perimeter_stop(&d))
                    .await;
        }
    }
}

/// Own the perimeter until `shutdown` fires (or a Shutdown control request).
///
/// Brings it up, then each [`TICK`]: first drains the [`crate::control`] inbox
/// (pause/resume/restart/shutdown — these work in any state), then, while awake,
/// auto-pauses if [`should_auto_pause`] holds (arming the wake-on-message waker,
/// ADR-0018). On shutdown, cancels the waker and brings the perimeter down.
pub async fn run(data_dir: PathBuf, threshold_ms: u64, shutdown: Arc<Notify>) {
    perimeter_up(data_dir.clone()).await;
    verify_boundary_fail_closed(&data_dir).await;

    let mut waker: Option<IdleWaker> = None;

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = tokio::time::sleep(TICK) => {}
        }

        // 1. Explicit control requests (valid in any state).
        let mut stop = false;
        for req in crate::control::drain(&data_dir) {
            match req {
                ControlRequest::Shutdown => stop = true,
                ControlRequest::Pause => user_pause(&data_dir, &mut waker).await,
                ControlRequest::Resume => resume_now(&data_dir, &mut waker).await,
                ControlRequest::Restart => restart_now(&data_dir, &mut waker).await,
            }
        }
        if stop {
            eprintln!("[supervisor] shutdown requested via control channel");
            break;
        }

        // 2. Idle auto-pause (only while awake + not user-paused).
        let dormant = crate::markers::is_set(&data_dir, crate::markers::DORMANT);
        let paused = crate::markers::is_set(&data_dir, crate::markers::PAUSED);
        if dormant || paused {
            continue; // asleep (waker owns resume) or user-paused (until Resume)
        }
        let have_token = crate::idle::read_telegram_token(&data_dir).is_some();
        let last = crate::orchestrator::podman::read_egress_log_last_activity_ms();
        if should_auto_pause(last, threshold_ms, dormant, have_token) {
            eprintln!("[supervisor] idle past threshold");
            arm_pause(&data_dir, &mut waker).await;
        }
    }

    if let Some(w) = waker.take() {
        w.cancel();
    }
    eprintln!("[supervisor] shutting down — bringing the perimeter down");
    perimeter_down(data_dir).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pauses_only_when_idle_past_threshold_with_a_token() {
        let t = IDLE_TIMEOUT_MS_DEFAULT;
        // Idle past threshold, awake, token present → pause.
        assert!(should_auto_pause(Some(t), t, false, true));
        assert!(should_auto_pause(Some(t + 1), t, false, true));
    }

    #[test]
    fn never_pauses_without_signal_token_or_when_dormant() {
        let t = IDLE_TIMEOUT_MS_DEFAULT;
        assert!(!should_auto_pause(None, t, false, true)); // no activity signal
        assert!(!should_auto_pause(Some(t), t, false, false)); // no token → no wake path
        assert!(!should_auto_pause(Some(t), t, true, true)); // already dormant
        assert!(!should_auto_pause(Some(t - 1), t, false, true)); // not idle long enough
    }
}
