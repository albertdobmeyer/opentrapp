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

/// The idle-to-dormant threshold in ms, read fresh at daemon launch. Defaults to
/// [`IDLE_TIMEOUT_MS_DEFAULT`]; override with `OPENTRAPP_IDLE_TIMEOUT_MS` for fast
/// on-box T1 verification / ops tuning (a `0` or unparseable value is ignored,
/// falling back to the default — never an accidental 0-second hair-trigger).
pub fn idle_threshold_ms() -> u64 {
    std::env::var("OPENTRAPP_IDLE_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&ms| ms > 0)
        .unwrap_or(IDLE_TIMEOUT_MS_DEFAULT)
}
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

/// Apply a human-**approved** held weakening request — the SOLE edge from
/// "pending approval" to an applied boundary-weakening op (ADR-0021), the
/// generalization of [`crate::orchestrator::allowlist::apply_always`]. Called
/// ONLY from the out-of-band approval surface (the GUI two-tap), **never** from
/// the agent-reachable inbox-drain path ([`gate_inbox_request`] holds the
/// request; this consumes the human's approval). Operates on the shared markers
/// + podman, so the (separate-process) approval surface applies it without the
/// daemon's in-memory waker. Returns whether a pending request with `id` was
/// found and applied.
pub async fn apply_approved(data_dir: &std::path::Path, id: &str) -> bool {
    let Some(req) = crate::approvals::take_approved(data_dir, id) else {
        return false; // nothing pending under that id (idempotent; no double-apply)
    };
    match req {
        ControlRequest::Pause => {
            let _ = crate::markers::set_flag(data_dir, crate::markers::PAUSED);
            let d = data_dir.to_path_buf();
            let _ = tokio::task::spawn_blocking(move || {
                crate::orchestrator::podman::perimeter_stop(&d)
            })
            .await;
            eprintln!("[approvals] applied human-approved PAUSE");
            true
        }
        ControlRequest::Shutdown => {
            // Stopping the daemon PROCESS is the owner's SIGTERM/quit, not a file
            // op the (separate-process) approval surface can perform. The approval
            // is consumed; the owner's signal is the stop. Honest, not a no-op trap.
            eprintln!("[approvals] shutdown approved — quit the app / SIGTERM the daemon to stop");
            true
        }
        // Neutral verbs are never enqueued for approval; defensive.
        ControlRequest::Resume | ControlRequest::Restart => false,
    }
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
/// CLAUDE.md §11). **Default ON** (opt-out via `OPENTRAPP_SELFTEST_ON_RESUME=0`):
/// hardware-verified (the 2026-06-26 product-path T0), so every (re)start re-tests
/// the boundary. Called by EVERY resume path — `resume_now`/`restart_now` (control
/// channel) and `idle::resume_from_dormant` (wake-on-message) — so a resumed
/// boundary is never left "alive but unverified".
///
/// Fail-closed on the script's verdict: `Fail` → stop the perimeter + raise the
/// `BOUNDARY_FAILED` alert (a half-built boundary serves nothing); `CannotAssess`
/// → alert but leave it up (couldn't measure ≠ failed); `Pass` → clear the alert.
pub(crate) async fn verify_boundary_fail_closed(data_dir: &PathBuf) {
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

/// ADR-0021 gate at the agent-reachable control-dispatch chokepoint. The
/// `~/.opentrapp/control` inbox is a file drop any host process (incl. a
/// prompt-injected host agent) can write, so a **boundary-weakening** request
/// (`Pause`/`Shutdown`) is never applied here — it is **held** in the
/// out-of-band approval queue ([`crate::approvals`]) and applied only when a
/// human approves it on the approval surface. Generalizes the ADR-0016
/// invariant ("the weakening writer has no agent call edge") from the contained
/// agent to the control inbox: the dispatch loop has no edge to the weakening
/// appliers — those are reached only from [`apply_approved`].
///
/// Returns `Some(req)` for a neutral request (apply it now), `None` if held.
pub(crate) fn gate_inbox_request(data_dir: &std::path::Path, req: ControlRequest) -> Option<ControlRequest> {
    if req.boundary_impact().agent_operable() {
        return Some(req);
    }
    let _ = crate::approvals::enqueue(data_dir, req);
    eprintln!(
        "[supervisor] HELD boundary-weakening request {req:?} for out-of-band approval \
         (ADR-0021 — the control inbox cannot weaken the perimeter; approve in the OpenTrApp app)"
    );
    None
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

        // 1. Explicit control requests. ADR-0021: the `control` inbox is an
        //    agent-writable file drop, so a boundary-WEAKENING request is HELD for
        //    out-of-band approval by `gate_inbox_request` (never applied here); only
        //    NEUTRAL requests (Resume/Restart) are applied. The daemon still stops
        //    on SIGTERM/SIGINT (`shutdown.notified`, the owner's own process control,
        //    daemon main.rs) — that is the un-amplified T4 escape the gate inherits.
        for req in crate::control::drain(&data_dir) {
            let Some(req) = gate_inbox_request(&data_dir, req) else {
                continue; // weakening — held for human approval, not applied
            };
            match req {
                ControlRequest::Resume => resume_now(&data_dir, &mut waker).await,
                ControlRequest::Restart => restart_now(&data_dir, &mut waker).await,
                // Pause/Shutdown are weakening — gate_inbox_request held them above,
                // so they never reach here (the inbox has no edge to the appliers).
                ControlRequest::Pause | ControlRequest::Shutdown => {}
            }
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

    #[test]
    fn idle_threshold_env_override() {
        // Default when unset; honored for a positive integer; ignored (→ default)
        // for 0 or garbage — never an accidental 0-second hair-trigger.
        std::env::remove_var("OPENTRAPP_IDLE_TIMEOUT_MS");
        assert_eq!(idle_threshold_ms(), IDLE_TIMEOUT_MS_DEFAULT);
        std::env::set_var("OPENTRAPP_IDLE_TIMEOUT_MS", "30000");
        assert_eq!(idle_threshold_ms(), 30_000);
        std::env::set_var("OPENTRAPP_IDLE_TIMEOUT_MS", "0");
        assert_eq!(idle_threshold_ms(), IDLE_TIMEOUT_MS_DEFAULT);
        std::env::set_var("OPENTRAPP_IDLE_TIMEOUT_MS", "notanumber");
        assert_eq!(idle_threshold_ms(), IDLE_TIMEOUT_MS_DEFAULT);
        std::env::remove_var("OPENTRAPP_IDLE_TIMEOUT_MS");
    }

    #[test]
    fn inbox_holds_weakening_and_admits_neutral() {
        // ADR-0021 §2/§3: a weakening request from the agent-writable control
        // inbox is HELD (enqueued for out-of-band approval), never applied here;
        // a neutral request is admitted for immediate application.
        let d = std::env::temp_dir().join(format!("opentrapp-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(gate_inbox_request(&d, ControlRequest::Pause), None, "Pause held");
        assert_eq!(gate_inbox_request(&d, ControlRequest::Shutdown), None, "Shutdown held");
        assert_eq!(
            crate::approvals::list(&d).len(),
            2,
            "both weakening requests are pending approval, not applied"
        );
        assert_eq!(
            gate_inbox_request(&d, ControlRequest::Resume),
            Some(ControlRequest::Resume),
            "Resume admitted"
        );
        assert_eq!(
            gate_inbox_request(&d, ControlRequest::Restart),
            Some(ControlRequest::Restart),
            "Restart admitted"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
