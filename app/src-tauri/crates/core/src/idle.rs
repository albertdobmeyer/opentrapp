//! Host-side wake-on-message waker for idle auto-pause (Phase 3, ADR-0018).
//!
//! When the watchdog auto-pauses the perimeter to save memory (idle → dormant,
//! see `lifecycle::auto_pause_to_dormant`), `vault-agent` is stopped and nothing
//! is polling Telegram anymore. This module owns the single responsibility of
//! noticing the user's next message while dormant and resuming the perimeter.
//!
//! Design invariants (ADR-0018):
//!
//! - **Peek only.** The waker long-polls `getUpdates` with **no `offset`
//!   parameter** — which returns every currently-unconfirmed update *without*
//!   confirming any. It never passes `update_id + 1` and never uses a negative
//!   offset (a negative offset makes Telegram "forget" earlier updates, which
//!   would drop queued messages). It reads only whether *any* update is present,
//!   never the message body. The agent therefore consumes the queued message
//!   from its own persisted offset exactly once when it comes back up.
//! - **One consumer at a time.** Telegram allows a single `getUpdates` consumer
//!   per bot (HTTP 409 otherwise). While dormant the agent is stopped, so the
//!   waker is the sole consumer. On a message it stops polling *before* the
//!   perimeter (and thus the agent's own poller) comes back up. External resume
//!   paths call `stop_waker`, which cancels and awaits teardown before bringing
//!   the perimeter up.
//!
//! The waker is spawned by `lifecycle::auto_pause_to_dormant` when the watchdog
//! drops the perimeter to dormant. Idle auto-pause is a user setting
//! (`idleAutoPause` in the frontend store), default ON since Slice E; see
//! `lifecycle::read_idle_settings` / `maybe_auto_pause_idle`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use crate::orchestrator::state::AppState;

/// getUpdates long-poll window, in seconds. The HTTP client deadline is this
/// plus a margin to accommodate Telegram's server-side delay.
const POLL_TIMEOUT_SECS: u32 = 30;
/// Back-off after a transient failure (network error / 409 / non-200) so the
/// waker doesn't hot-loop.
const BACKOFF_SECS: u64 = 15;

/// A running waker: a cancellation signal plus the task handle so an external
/// resume path can stop it and await its teardown before resuming the perimeter.
pub struct IdleWaker {
    cancel: Arc<Notify>,
    handle: tokio::task::JoinHandle<()>,
}

impl IdleWaker {
    /// Cancel + abort the waker task without awaiting it. Used to replace a
    /// stale waker (the GUI shim stores at most one). For an ordered teardown
    /// that awaits the task before a resume, use [`stop_waker`].
    pub fn cancel(&self) {
        self.cancel.notify_one();
        self.handle.abort();
    }
}

/// The result of a single peek poll. Carries no message content — only whether
/// the queue is non-empty (`Arrived`), empty (`Quiet`), or the poll failed and
/// should be retried (`Transient`).
#[derive(Debug, PartialEq, Eq)]
enum PeekOutcome {
    Arrived,
    Quiet,
    Transient,
}

#[derive(serde::Deserialize)]
struct PeekResponse {
    ok: bool,
    result: Option<Vec<PeekUpdate>>,
}

/// We deserialize only `update_id` — never the message body. Its presence is
/// the entire signal the waker needs.
#[derive(serde::Deserialize)]
struct PeekUpdate {
    #[allow(dead_code)]
    update_id: i64,
}

// ─── Bot token (host-side, from `.env`) ───────────────────────────────────

/// Parse `TELEGRAM_BOT_TOKEN` out of a `.env` file body. Mirrors the parsing in
/// `status_aggregator::parse_env_keys` (placeholder + quote handling) but
/// returns the token value. `None` when absent, blank, or still a placeholder.
fn parse_token(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "TELEGRAM_BOT_TOKEN" {
            continue;
        }
        let v = value.trim().trim_matches(|c| c == '"' || c == '\'');
        if v.is_empty() || v.contains("REPLACE") {
            return None;
        }
        return Some(v.to_string());
    }
    None
}

/// Read the bot token host-side from `<data_dir>/.env`. The waker runs while the
/// perimeter is down, so it cannot ask any container — it reads the same `.env`
/// the orchestrator injects into `vault-agent`.
pub fn read_telegram_token(data_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(data_dir.join(".env")).ok()?;
    parse_token(&content)
}

// ─── Peek (one poll) ──────────────────────────────────────────────────────

/// Build the getUpdates query. Deliberately omits `offset`: omitting it returns
/// unconfirmed updates *without* confirming any, which is the whole point — the
/// waker must never advance/forget the agent's queue. Pinned by a unit test.
fn peek_query(timeout_secs: u32) -> Vec<(&'static str, String)> {
    vec![("timeout", timeout_secs.to_string())]
}

/// Classify a parsed getUpdates response. Pure, so it can be unit-tested without
/// a network. A non-`ok` body is treated as transient (retry), not as quiet.
fn classify(ok: bool, result_len: usize) -> PeekOutcome {
    if !ok {
        return PeekOutcome::Transient;
    }
    if result_len > 0 {
        PeekOutcome::Arrived
    } else {
        PeekOutcome::Quiet
    }
}

/// One long-poll peek. Any failure (network, 409, non-200, malformed JSON) maps
/// to `Transient` so the caller backs off and retries rather than treating the
/// failure as silence.
async fn peek_once(client: &reqwest::Client, token: &str, timeout_secs: u32) -> PeekOutcome {
    let url = format!("https://api.telegram.org/bot{token}/getUpdates");
    let resp = match client.get(&url).query(&peek_query(timeout_secs)).send().await {
        Ok(r) => r,
        Err(_) => return PeekOutcome::Transient,
    };
    if !resp.status().is_success() {
        // Includes 409 (a second getUpdates consumer) — back off; cancellation
        // resolves the race when resume is in flight.
        return PeekOutcome::Transient;
    }
    match resp.json::<PeekResponse>().await {
        Ok(body) => classify(body.ok, body.result.map(|r| r.len()).unwrap_or(0)),
        Err(_) => PeekOutcome::Transient,
    }
}

// ─── Waker task ───────────────────────────────────────────────────────────

async fn waker_loop(data_dir: PathBuf, token: String, cancel: Arc<Notify>) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(POLL_TIMEOUT_SECS as u64 + 10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[idle] waker could not build HTTP client: {e}");
            return;
        }
    };

    loop {
        let outcome = tokio::select! {
            _ = cancel.notified() => return,
            o = peek_once(&client, &token, POLL_TIMEOUT_SECS) => o,
        };
        match outcome {
            // The waker has stopped polling by the time it gets here, so resuming
            // the perimeter (which restarts the agent's own poller) never overlaps.
            PeekOutcome::Arrived => {
                resume_from_dormant(&data_dir).await;
                return;
            }
            PeekOutcome::Quiet => continue,
            PeekOutcome::Transient => {
                tokio::select! {
                    _ = cancel.notified() => return,
                    _ = tokio::time::sleep(Duration::from_secs(BACKOFF_SECS)) => {}
                }
            }
        }
    }
}

/// Resume the perimeter from dormant. Marker-gated so concurrent resume triggers
/// (the waker and an external `resume_perimeter`) don't both bring the perimeter
/// up: whoever clears the dormant marker first proceeds; the other no-ops.
async fn resume_from_dormant(data_dir: &Path) {
    if !crate::markers::is_set(data_dir, crate::markers::DORMANT) {
        return; // already resumed by another path
    }
    crate::markers::clear(data_dir, crate::markers::DORMANT);
    let dir = data_dir.to_path_buf();
    let up_dir = dir.clone();
    let ok = tokio::task::spawn_blocking(move || {
        crate::orchestrator::podman::perimeter_up(&up_dir).is_ok()
    })
    .await
    .unwrap_or(false);
    if !ok {
        eprintln!("[idle] resume from dormant failed; the perimeter may need a manual restart");
        return;
    }
    // §11: the resumed boundary must re-pass the self-test, fail-closed — the SAME
    // contract the control-channel resume (`supervisor::resume_now`) enforces. Without
    // this the wake-on-message path (the most common resume) would be "alive but
    // unverified". `on_resume_enabled()` defaults ON; no-ops only on explicit opt-out.
    crate::supervisor::verify_boundary_fail_closed(&dir).await;
}

// ─── Public control surface ───────────────────────────────────────────────

/// Build + spawn the waker for a freshly-dormant perimeter, returning the
/// handle. `None` (with a log) when no bot token is configured — without a token
/// there is nothing to poll. The GUI shim (`crate::idle::spawn_waker`) stores the
/// returned waker in `AppState`, replacing any previous one; keeping the AppHandle
/// glue out of core is the whole point of the Phase B split (ADR-0019).
pub fn spawn(data_dir: PathBuf) -> Option<IdleWaker> {
    let Some(token) = read_telegram_token(&data_dir) else {
        eprintln!("[idle] no Telegram token; not spawning a waker (perimeter stays dormant)");
        return None;
    };

    let cancel = Arc::new(Notify::new());
    let cancel_task = cancel.clone();
    let handle = tokio::spawn(async move {
        waker_loop(data_dir, token, cancel_task).await;
    });
    Some(IdleWaker { cancel, handle })
}

/// Cancel the waker (if any) and await its teardown. Must be called *before* an
/// external resume brings the perimeter up, so the agent's poller never overlaps
/// the waker's. Safe no-op when no waker is running.
pub async fn stop_waker(state: &AppState) {
    let waker = state.waker.lock().ok().and_then(|mut g| g.take());
    if let Some(w) = waker {
        w.cancel.notify_one();
        let _ = w.handle.await;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_token_plain() {
        assert_eq!(
            parse_token("TELEGRAM_BOT_TOKEN=123:abc\n").as_deref(),
            Some("123:abc")
        );
    }

    #[test]
    fn parses_token_strips_quotes() {
        assert_eq!(
            parse_token("TELEGRAM_BOT_TOKEN=\"123:abc\"").as_deref(),
            Some("123:abc")
        );
    }

    #[test]
    fn ignores_placeholder_and_unrelated_keys() {
        assert_eq!(parse_token("TELEGRAM_BOT_TOKEN=REPLACE_ME"), None);
        assert_eq!(parse_token("# TELEGRAM_BOT_TOKEN=123\n"), None);
        assert_eq!(parse_token("ANTHROPIC_API_KEY=sk-ant"), None);
        assert_eq!(parse_token(""), None);
    }

    #[test]
    fn classify_arrived_quiet_transient() {
        assert_eq!(classify(true, 1), PeekOutcome::Arrived);
        assert_eq!(classify(true, 0), PeekOutcome::Quiet);
        assert_eq!(classify(false, 5), PeekOutcome::Transient);
    }

    #[test]
    fn peek_query_never_advances_offset() {
        // The core peek-only invariant: no `offset` is ever sent, so the agent's
        // queue is never confirmed/forgotten by the waker.
        let q = peek_query(POLL_TIMEOUT_SECS);
        assert!(q.iter().all(|(k, _)| *k != "offset"));
        assert!(q
            .iter()
            .any(|(k, v)| *k == "timeout" && v == &POLL_TIMEOUT_SECS.to_string()));
    }
}
