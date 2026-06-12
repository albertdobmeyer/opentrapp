//! Frontend-facing perimeter lifecycle commands.
//!
//! Pass 4 shipped `get_perimeter_state` so the Pass-6 Home rebuild has a
//! live data source for the hero machine. Pass 7 Day 3 added
//! `restart_perimeter` so a key rotation in Preferences automatically
//! cycles vault-agent (which only reads `.env` on boot) without making
//! Karen reach for a terminal or manually relaunch. Pass 7 Day 4 adds
//! `pause_perimeter` + `resume_perimeter` for the user-initiated stop
//! state — closes the last hero state gap from Pass 2's spec.

use tauri::{AppHandle, State};

use crate::lifecycle::{
    bring_perimeter_down_sync, clear_paused_marker, write_paused_marker,
    PerimeterStateStore, PerimeterStatus,
};
use crate::orchestrator::podman;
use crate::orchestrator::state::AppState;

/// Read the latest cached perimeter state. Returns immediately — does not
/// trigger a fresh probe (the watchdog runs every 30s in the background).
/// If the watchdog hasn't ticked yet, `last_checked_unix_ms` will be 0.
#[tauri::command]
pub fn get_perimeter_state(
    store: State<'_, PerimeterStateStore>,
) -> Result<PerimeterStatus, String> {
    store
        .status
        .lock()
        .map(|guard| guard.clone())
        .map_err(|e| format!("perimeter state lock poisoned: {e}"))
}

/// Cycle the perimeter (down + up). Synchronous from the caller's
/// perspective — awaits both phases before returning so the frontend
/// can show "Restarting…" → "Your assistant is back online" with
/// accurate timing. Compose work runs on a blocking task so the tokio
/// reactor doesn't stall on the ~10–20s typical restart.
///
/// Returns Err with a friendly message when the up step fails — most
/// likely cause is a malformed key the user just saved (vault-agent
/// rejects it on boot). The user's previous keys remain on disk so
/// they can fix and retry.
#[tauri::command]
pub async fn restart_perimeter(state: State<'_, AppState>) -> Result<(), String> {
    let root = state
        .runtime_data_dir
        .read()
        .map(|g| g.clone())
        .map_err(|e| format!("runtime data dir lock poisoned: {e}"))?;

    // When a daemon owns the perimeter (B4b), route the restart through the
    // control channel instead of touching podman directly.
    if state.daemon_owned.load(std::sync::atomic::Ordering::SeqCst) {
        return opentrapp_core::control::submit(
            &root,
            opentrapp_core::control::ControlRequest::Restart,
        )
        .map_err(|e| format!("failed to queue restart: {e}"));
    }

    let result = tokio::task::spawn_blocking(move || {
        bring_perimeter_down_sync(&root);
        podman::perimeter_up(&root).is_ok()
    })
    .await
    .map_err(|e| format!("restart task join failed: {e}"))?;

    if !result {
        return Err(
            "Couldn't bring your assistant back up. Check the key you just saved \
             and try again."
                .to_string(),
        );
    }

    Ok(())
}

/// Pause the perimeter on user request. Stops the 4 containers but keeps
/// them around (no `down`, no destroy) so resume is fast. Persists the
/// paused state to `~/.opentrapp/paused` so it survives an app restart
/// — pausing yesterday shouldn't silently un-pause when the user reopens
/// the app today.
///
/// The status aggregator reads the paused flag every 60s and reports
/// `paused_by_user` regardless of container state, so the user sees a
/// calm "paused" hero rather than an alarming "didn't recover" one.
#[tauri::command]
pub async fn pause_perimeter(
    state: State<'_, AppState>,
    store: State<'_, PerimeterStateStore>,
) -> Result<(), String> {
    let root = state
        .runtime_data_dir
        .read()
        .map(|g| g.clone())
        .map_err(|e| format!("runtime data dir lock poisoned: {e}"))?;

    // When a daemon owns the perimeter (B4b), route the user pause through the
    // control channel; the daemon sets the paused marker + stops containers.
    if state.daemon_owned.load(std::sync::atomic::Ordering::SeqCst) {
        return opentrapp_core::control::submit(
            &root,
            opentrapp_core::control::ControlRequest::Pause,
        )
        .map_err(|e| format!("failed to queue pause: {e}"));
    }

    // Set the flag first so even if compose stop is slow the aggregator
    // already classifies the state correctly on the next tick.
    store.set_paused(true);
    if let Err(e) = write_paused_marker() {
        eprintln!("[lifecycle] couldn't persist paused marker: {e}");
    }

    let result = tokio::task::spawn_blocking(move || {
        podman::perimeter_stop(&root).is_ok()
    })
    .await
    .map_err(|e| format!("pause task join failed: {e}"))?;

    if !result {
        // Roll back the flag so the user isn't stuck in a fake-paused
        // state with running containers.
        store.set_paused(false);
        clear_paused_marker();
        return Err("Couldn't pause your assistant. Try again in a moment.".to_string());
    }

    Ok(())
}

/// Re-run the bootstrap pipeline from scratch after a failure.
///
/// Clears the `BootstrapProgress::Failed` state so the UI shows
/// "bootstrapping" immediately, then spawns a fresh pipeline run. Returns
/// immediately — the pipeline runs asynchronously.
#[tauri::command]
pub fn retry_bootstrap(
    handle: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, PerimeterStateStore>,
) -> Result<(), String> {
    let root = state
        .runtime_data_dir
        .read()
        .map(|g| g.clone())
        .map_err(|e| format!("runtime data dir lock poisoned: {e}"))?;

    // Clear failure state so the watchdog and status aggregator see "bootstrapping"
    // instead of "shell_failed" during the retry run.
    if let Ok(mut g) = store.bootstrap_progress.write() {
        *g = None;
    }

    // Tear down any half-built perimeter from the failed attempt before
    // re-running. Without this, the partially-created containers collide on the
    // next `podman run` (`name "vault-skills" is already in use`) and every retry
    // fails forever. Idempotent — safe when nothing is up.
    bring_perimeter_down_sync(&root);

    crate::bootstrap::spawn_bootstrap(handle, root);
    Ok(())
}

/// Resume from a paused state. Clears the persisted flag and brings
/// containers back online. Same `compose up -d` path as restart, since
/// `compose stop` left the containers around but stopped.
#[tauri::command]
pub async fn resume_perimeter(
    state: State<'_, AppState>,
    store: State<'_, PerimeterStateStore>,
) -> Result<(), String> {
    let root = state
        .runtime_data_dir
        .read()
        .map(|g| g.clone())
        .map_err(|e| format!("runtime data dir lock poisoned: {e}"))?;

    // When a daemon owns the perimeter (B4b), route the resume through the control
    // channel; the daemon stops its waker + clears the markers + brings up.
    if state.daemon_owned.load(std::sync::atomic::Ordering::SeqCst) {
        return opentrapp_core::control::submit(
            &root,
            opentrapp_core::control::ControlRequest::Resume,
        )
        .map_err(|e| format!("failed to queue resume: {e}"));
    }

    // If the perimeter was dormant (idle auto-pause), stop the host-side waker
    // and await its teardown BEFORE bringing the perimeter up — otherwise the
    // waker's getUpdates poll would overlap the agent's, tripping Telegram's
    // single-consumer 409 (ADR-0018). Safe no-op when no waker is running.
    crate::idle::stop_waker(state.inner()).await;
    crate::lifecycle::clear_dormant_marker();

    // Clear the flag first so a slow start still shows "starting/recovering"
    // rather than staying stuck on "paused".
    store.set_paused(false);
    clear_paused_marker();

    let result = tokio::task::spawn_blocking(move || {
        podman::perimeter_up(&root).is_ok()
    })
    .await
    .map_err(|e| format!("resume task join failed: {e}"))?;

    if !result {
        return Err(
            "Couldn't bring your assistant back online. Try again in a moment.".to_string(),
        );
    }

    Ok(())
}
