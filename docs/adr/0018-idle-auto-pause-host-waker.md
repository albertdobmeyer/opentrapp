# ADR-0018 — Idle auto-pause and the host-side wake-on-message waker

**Status:** Accepted — design (Phase 3 Slice D of the memory-optimization initiative); backend pause path (Slices A–C) landed and CI-green behind `IDLE_AUTO_PAUSE_ENABLED=false`; the waker is the activation step
**Companion plan:** memory-optimization plan (`~/.claude/plans/glimmering-meandering-babbage.md`), Phase 3
**Cross-references:** [ADR-0009](0009-five-container-perimeter.md) · [ADR-0011](0011-zero-trust-self-sufficient-bootstrap.md) · [ADR-0001](0001-proxy-side-api-key-injection.md) · [threat-model.md](../threat-model.md) T1

---

## Context

OpenTrApp's resting footprint is dominated by two long-lived containers:
`vault-agent` (~600 MB, the Node/OpenClaw runtime) and `vault-proxy` (~150 MB,
mitmproxy). These stay resident for as long as the perimeter is up, even when the
user has not spoken to the bot in hours. On a small laptop (the 7.2 GB reference
machine) that resting cost is the difference between "a silent background process"
and "swap-storms the box." Phase 1 already moved the two idle shields off the boot
set (5→3 resting containers), but the agent and proxy are the real weight, and they
cannot simply be made on-demand: the agent *is* the thing the user talks to.

The lever that actually releases that RAM is **idle auto-pause**: when the agent has
been idle for N minutes, stop the whole perimeter (resident RAM → ~0), and bring it
back the instant the user sends another Telegram message. The backend pause path is
already in place and CI-green, hard-gated off:

- **Idle signal** — `podman::read_egress_log_last_activity_ms()` (mtime of the
  proxy's `requests.jsonl`; the agent polls Telegram *through* the proxy, so every
  poll is a logged request). `None` = no signal = never auto-pause.
- **Dormant markers** — `~/.opentrapp/dormant`, a top-level state override distinct
  from `paused` (`lifecycle::{write,clear,is}_dormant_*`).
- **Dormant display** — `AssistantStatus::Dormant`, surfaced by a marker-override in
  `status_aggregator::evaluate` and a dormant-aware tray ("sleeping to save memory").
- **Auto-pause trigger** — `lifecycle::auto_pause_to_dormant` (write marker +
  `podman::perimeter_stop`, which preserves volumes so the agent's persisted
  getUpdates offset survives) on the existing 30 s watchdog, gated by
  `const IDLE_AUTO_PAUSE_ENABLED = false`.

The piece that makes activation *safe* is the missing one: **once the perimeter is
stopped, nothing is polling Telegram, so nothing can notice the next message.** The
agent was the poller, and it is now stopped. A host-side **waker** must take over
that single responsibility while dormant, and hand it back cleanly on wake.

This adds one genuinely new runtime behavior — the host process making outbound
Telegram calls on its own, outside the wizard flow — so it warrants its own ADR and
a threat-model row rather than a silent merge.

## Decision

A host-side getUpdates **peek** waker (`app/src-tauri/src/idle.rs`), spawned only
while dormant, that detects the arrival of the next message and resumes the
perimeter — without ever consuming, advancing, or reading the message.

1. **Reuse the existing host→Telegram channel, do not open a new one.** The waker
   calls the same `getUpdates` endpoint, with the same bot token, in the same
   outbound-only direction as the wizard activation handoff
   (`commands/telegram.rs::telegram_poll_for_start`). The token is read host-side
   from the app `.env` (`TELEGRAM_BOT_TOKEN`) via the existing `vault_env_path`,
   with a small value-getter alongside the current presence-parser in
   `status_aggregator`. Like the activation handoff, this is a **direct**
   host→`api.telegram.org` call — the proxy is stopped while dormant, so there is
   nothing to route through. This is the same network surface category that already
   shipped and was reviewed in the wizard; the waker is a new *caller*, not a new
   *surface*.

2. **Peek only — never acknowledge, never advance, never read content.** The waker
   long-polls `getUpdates` with **no `offset` parameter at all** (which returns every
   currently-unconfirmed update without confirming any) and `timeout ≈ 30`. It
   **never** passes `update_id + 1`, and it **deliberately avoids a negative offset**
   (`offset = -1`): the Telegram Bot API specifies that a negative offset causes "all
   previous updates [to] be forgotten," which would drop earlier queued messages and
   break exactly-once. Confirmation (dropping) only happens when a later request names
   a higher offset, and Telegram queues unconfirmed bot updates for ~24 h. The waker
   inspects only whether *any* update is present (a non-empty `result`); it never
   parses the message body and never calls `telegram_advance_offset`. The agent's
   queue is therefore untouched: when the agent comes back up it consumes from its own
   persisted offset exactly as if the waker had never run.

3. **One getUpdates consumer at all times (the 409 invariant).** Telegram permits a
   single getUpdates consumer per bot and returns HTTP 409 on a second. While
   dormant, `vault-agent` is stopped and therefore not polling, so the waker is the
   sole consumer — no conflict. The dangerous window is wake: there must never be a
   moment where both the waker and a resuming agent poll. So resume is strictly
   ordered: **cancel the waker and await its teardown *first*, then resume the
   perimeter.** A `CancellationToken` (or `Notify` + an awaited `JoinHandle`) makes
   "the waker has fully stopped polling" an observable event the resume path waits
   on before it brings the agent up. If the waker itself ever sees a 409 it backs
   off rather than hot-looping (it implies a consumer race / an in-flight resume,
   which cancellation is about to resolve).

4. **Resume = exactly once, by construction.** On a detected arrival the waker
   signals the lifecycle layer, which: cancels+awaits the waker (per 3), clears the
   dormant marker, and resumes the perimeter via the existing resume path (reuse
   `commands/lifecycle.rs` resume / `podman` bring-up — do **not** fork a second
   resume implementation). `vault-agent` starts, polls from its persisted offset,
   and consumes the queued message exactly once. The waker advanced nothing, so the
   message is neither lost nor double-processed. Cold start is a few seconds; this is
   the documented latency cost of the RAM win.

5. **Dormant is a runtime-only state; app launch is always awake.** On startup the
   app clears any stale `~/.opentrapp/dormant` marker (e.g. left by a crash while
   dormant) and follows the normal boot path, so a crashed waker self-heals on the
   next launch rather than stranding the user in a dormant state with nothing
   polling. Opening the app means the assistant is awake.

6. **closeToTray is a hard prerequisite (lands with Slice E).** The waker lives in
   the host process; if closing the window exits the app, the waker dies and a
   dormant assistant can never wake. The existing-but-unhooked `closeToTray` setting
   must be wired (a Tauri `on_window_event` close handler that hides instead of
   exits) before idle auto-pause is enabled by default.

7. **Ships behind a setting, default decided at activation.** Slice E replaces
   `const IDLE_AUTO_PAUSE_ENABLED` with an `idleAutoPause` user setting (plus
   `idleTimeoutMinutes`, default chosen to exceed OpenClaw's getUpdates long-poll
   cadence so a normally-polling agent is never mistaken for idle). The waker code
   from this ADR is inert until that setting turns it on.

## Consequences

- **One new documented behavior, recorded in the threat model.** While dormant, the
  host process makes outbound getUpdates calls to `api.telegram.org`. Same endpoint,
  token, and direction as the already-shipped activation handoff; **no inbound
  surface, no new listener, metadata only** (presence of an `update_id`, never the
  message body), and the offset is never advanced. The single new T1 row records
  exactly this delta.
- **Two invariants carry the safety and are pinned by tests where feasible.**
  (a) *Peek-no-advance*: the waker must never call the offset-advancing path —
  testable by construction (it has no edge to `telegram_advance_offset`) and by a
  unit test asserting the resume sequence does not advance. (b) *Cancel-before-resume
  ordering*: testable with a fake waker that records whether teardown completed
  before the resume call fired.
- **Failure modes degrade safe.** Network down → the waker backs off and retries; the
  perimeter stays dormant and the next successful poll wakes it (no message is lost —
  Telegram holds it). App killed while dormant → next launch clears the marker and
  boots awake (consequence 5). A 409 → back off; cancellation resolves it.
- **Latency, stated honestly.** The first message after sleep pays a few-seconds cold
  start while the perimeter comes back up. This is the explicit trade for ~0 resting
  RAM and is surfaced in the Dormant UI copy ("wakes on your next message").
- **No container-side change.** The agent keeps managing its own Telegram offset; the
  proxy is unchanged. The waker is entirely host-side and reuses existing primitives.

## Alternatives considered

- **Keep a minimal poller alive (stop only the agent, keep the proxy + a tiny
  watcher).** Rejected for Slice D — the proxy is part of the ~150 MB we want to
  release, and a partial stop complicates the single-consumer model (the agent polls
  *through* the proxy; the waker polls direct). Full `perimeter_stop` is the cleanest
  ~0-RAM state. A lighter "doze" tier could be revisited later as an optimization.
- **Telegram webhook instead of long-poll.** Rejected — a webhook requires an
  inbound, publicly reachable endpoint on the host (a real new network surface, plus
  NAT/tunnel setup). Long-poll is outbound-only and reuses the channel that already
  exists; it adds no listener.
- **Advance the offset in the waker and re-inject the message into the agent on
  wake.** Rejected — it forces the host to parse and hold message *content* (a real
  information-exposure delta) and introduces a hand-off that can drop or duplicate
  the message. Peek-only + agent-consumes-from-its-own-offset exposes nothing and is
  exactly-once for free.
- **Poll on a fixed short timer instead of a long-poll loop.** Rejected — short
  fixed-interval polling is both higher latency (up to the interval) and more
  request volume than a 30 s long-poll, which blocks server-side and returns the
  instant a message lands.

---

## Addendum (2026-06-11) — waker survival is now structural (lazy-window leanness)

Consequence 6 originally made `closeToTray` a hard prerequisite: "if closing the
window exits the app, the waker dies." The lean background-process work (plan
*Lean background-process architecture*, Phase A) changes the window-close mechanism
from **hide** to **destroy**, so the ~222 MB `WebKitWebProcess` is freed at rest
instead of staying resident. The waker's survival no longer depends on hiding a
window — it is now **structural**: closing the dashboard destroys the window, and
`RunEvent::ExitRequested` is vetoed via `api.prevent_exit()` (unless an explicit
Quit set `AppState.quitting`), so the tray-only daemon — watchdog, idle waker, and
perimeter — lives on with zero windows. `closeToTray` is retained but redefined:
`true` (default) → close keeps the daemon; `false` → close == quit (sets the flag).
The waker prerequisite holds for the default, now via `prevent_exit` rather than
`window.hide()`. Implemented in `lib.rs` (`open_dashboard`/`request_quit`,
`ExitRequested` veto), `orchestrator/state.rs` (`quitting: AtomicBool`),
`lifecycle.rs` (signal handlers set the flag), `tauri.conf.json` (`windows: []`).

---

## Addendum (2026-06-12) — the resumed boundary must equal the cold boundary

Idle auto-pause introduces a new lifecycle event this ADR did not originally treat
as security-relevant: **resume**. A perimeter that wakes from dormant (or a user
pause, or a daemon restart) and is "alive but subtly wrong" — network isolation not
re-applied, the L3 egress filter not reloaded, the proxy CA silently swapped — is
*worse* than a visible failure, because the breach is silent (CLAUDE.md §11). The
contract is now explicit: **a resumed boundary must pass the SAME boundary
self-test as a cold start, fail-closed.**

- **The check.** [`tests/boundary-selftest.sh`](../../tests/boundary-selftest.sh)
  (six checks: network isolation, L7 allowlist, vendor-credential injection, L3
  egress filter, proxy-CA pinning, no host-side untrusted content) is embedded in
  the daemon (`opentrapp_core::selftest`, via `include_str!`), staged to
  `~/.opentrapp/boundary/` at runtime, and run after every (re)start in
  `supervisor::run` (cold), `resume_now` (dormant/user-pause wake), and `restart_now`.
- **Fail-closed.** Verdict from the script's exit code: `Pass` (0) clears any alert;
  `Fail` (1 / killed / unknown) → **stop the perimeter** + raise the `boundary-failed`
  marker (a half-built boundary serves no traffic); `CannotAssess` (2) → raise the
  alert but leave the perimeter up (could-not-measure ≠ failed). The marker is the
  viewer's signal to surface a security alert.
- **Default ON (hardware-verified 2026-06-26, §11).** The boundary self-test runs on
  EVERY resume path by default — both the control-channel resume (`resume_now`/
  `restart_now`) and `idle::resume_from_dormant` (wake-on-message) call
  `verify_boundary_fail_closed`. It was verified on the 7.2 GB box (the product-path
  T0: `opentrapp-daemon vault verify` pass=7, cold==resumed), so *unverifiable ≠
  verified* is no longer the blocker. Opt-out only via `OPENTRAPP_SELFTEST_ON_RESUME=0`
  (not recommended; road-to-recommendable §1A/§1B, task #45).
- **Operator escape hatch.** `opentrapp-daemon --boundary-selftest` runs the live
  check once and reports the verdict (exit 0/1/2) *without* tearing the perimeter
  down — for verifying a cold or resumed boundary by hand.

Implemented in `crates/core/src/selftest.rs`, `crates/core/src/supervisor.rs`
(`verify_boundary_fail_closed` + the three call sites), `crates/core/src/markers.rs`
(`BOUNDARY_FAILED`), and `crates/daemon/src/main.rs` (`--boundary-selftest`).
