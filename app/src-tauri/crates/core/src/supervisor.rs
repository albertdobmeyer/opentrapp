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

/// Wake from dormant. Stops the waker BEFORE bringing up (so the agent's poller
/// never overlaps it — ADR-0018) and clears the marker so a racing waker no-ops.
async fn resume_now(data_dir: &PathBuf, waker: &mut Option<IdleWaker>) {
    if let Some(w) = waker.take() {
        w.cancel();
    }
    if !crate::markers::is_set(data_dir, crate::markers::DORMANT) {
        return; // already awake
    }
    crate::markers::clear(data_dir, crate::markers::DORMANT);
    perimeter_up(data_dir.clone()).await;
    eprintln!("[supervisor] resumed from dormant");
}

/// Bring the perimeter down and back up (clears dormant + stops the waker).
async fn restart_now(data_dir: &PathBuf, waker: &mut Option<IdleWaker>) {
    if let Some(w) = waker.take() {
        w.cancel();
    }
    crate::markers::clear(data_dir, crate::markers::DORMANT);
    perimeter_down(data_dir.clone()).await;
    perimeter_up(data_dir.clone()).await;
    eprintln!("[supervisor] restarted");
}

/// Own the perimeter until `shutdown` fires (or a Shutdown control request).
///
/// Brings it up, then each [`TICK`]: first drains the [`crate::control`] inbox
/// (pause/resume/restart/shutdown — these work in any state), then, while awake,
/// auto-pauses if [`should_auto_pause`] holds (arming the wake-on-message waker,
/// ADR-0018). On shutdown, cancels the waker and brings the perimeter down.
pub async fn run(data_dir: PathBuf, threshold_ms: u64, shutdown: Arc<Notify>) {
    perimeter_up(data_dir.clone()).await;

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
                ControlRequest::Pause => arm_pause(&data_dir, &mut waker).await,
                ControlRequest::Resume => resume_now(&data_dir, &mut waker).await,
                ControlRequest::Restart => restart_now(&data_dir, &mut waker).await,
            }
        }
        if stop {
            eprintln!("[supervisor] shutdown requested via control channel");
            break;
        }

        // 2. Idle auto-pause (only while awake).
        let dormant = crate::markers::is_set(&data_dir, crate::markers::DORMANT);
        if dormant {
            continue; // asleep — the armed waker owns the resume path
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
