# ADR-0019 — Headless daemon + on-demand GUI viewer split

**Status:** Proposed — design (Phase B of the lean background-process initiative); Phase A
(lazy/destroy-on-close webview, shipped in v0.7.1-rc2) is the prerequisite and the fallback if
this split is deferred
**Companion plan:** lean background-process architecture (`~/.claude/plans/glimmering-meandering-babbage.md`), Phase B
**Cross-references:** [ADR-0008](0008-tauri-desktop-shell.md) · [ADR-0009](0009-five-container-perimeter.md) · [ADR-0011](0011-zero-trust-self-sufficient-bootstrap.md) · [ADR-0018](0018-idle-auto-pause-host-waker.md) · [trifecta.md](../trifecta.md) · [CLAUDE.md §5 generic-backend, §11 verification](../../CLAUDE.md)

---

## Context

OpenTrApp is meant to be a *silent background process* that wraps an autonomous CLI agent
"like a warm blanket" — present, watchful, and cheap enough to leave running on a small laptop
(the 7.2 GB reference machine). The application's job is to **own the lifetime of the
five-container perimeter** and to **project** what the containers are doing through a thin,
manifest-driven GUI. The GUI is a viewer; the orchestrator is the product.

That separation is true in the backend already. The perimeter lifetime, the 30 s watchdog
(`lifecycle.rs`), the status aggregator (`status_aggregator.rs`), the idle auto-pause + host
waker (`idle.rs`, ADR-0018), the bootstrap (`bootstrap/`), the `RunGuard` single-instance lock,
and the `~/.opentrapp` state are **all window-independent tokio tasks** — none of them need a
webview to do their work. But today they are *hosted inside the GUI process*: there is a single
`opentrapp` binary (plus the `opentrapp_lib` `cdylib`/`staticlib` that Tauri links), and that one
process links **wry + WebKitGTK 4.1**. WebKit is mapped into the process whether or not a window
is open.

Phase A (v0.7.1-rc2) made the webview *transient* — created on demand, **destroyed on close** —
so the heavy `WebKitWebProcess` (~222 MB measured) is no longer resident at rest. That was the
make-or-break memory win and it shipped. But the resting GUI process still carries what WebKit
leaves mapped even with zero windows: a GTK main process (~156 MB measured) + the shared
`WebKitNetworkProcess` (~47 MB) + the wrapper (~17 MB) ≈ **~220 MB resting**. That is the floor
Phase A cannot go below, because the binary that runs the watchdog *is* the binary that links
WebKit. To get to a true "silent background process" (~30–60 MB resting — just the Rust core),
the orchestrator must stop linking WebKit at all.

### The forces

1. **Resting RAM.** The single largest remaining resting cost is WebKit/GTK linked into the
   always-on process. The only way to reclaim it with certainty is for the always-on process to
   not contain WebKit. Per-process death (a whole process exiting) is the only reliable reclaim;
   "the window is closed but the process lives" still pays the GTK/network-process floor.
2. **Survival of the waker (ADR-0018).** Idle auto-pause is only safe if *something* keeps
   peeking Telegram while the perimeter is dormant. Today that "something" is the GUI process,
   kept alive at zero windows by `RunEvent::ExitRequested → prevent_exit()`. That is a structural
   dependency on the GUI process never dying — including not dying to a `WebKitNetworkProcess`
   SIGBUS under memory pressure (which we observed live). A perimeter whose waker can be killed by
   a webview crash is fragile in exactly the way §11 warns about: "alive but subtly wrong."
3. **The generic-backend / projection vision (CLAUDE.md §5).** "The GUI is just a projection of
   what's underneath." A projection should be disposable: openable, closable, and crash-tolerant
   without touching the thing it projects. Today the projection *owns* the thing it projects.
4. **Boundary correctness (CLAUDE.md §11).** A security boundary that is torn down or made
   incorrect by a *UI* event is not a boundary. The perimeter lifetime must not be coupled to a
   webview's lifetime.

## Decision

Split the single GUI-hosted binary into **two processes with one owner**:

- **`opentrapp-daemon`** — a **headless binary that does not link wry or WebKit**. It owns the
  perimeter lifetime, the watchdog, the status aggregator, the idle auto-pause + waker, the
  bootstrap, the `RunGuard` single-instance lock, and the `~/.opentrapp` state. It is the
  long-lived "background blanket." Resting RSS target: **~30–60 MB** (the Rust/tokio core +
  podman invocations).

- **`opentrapp` (the GUI viewer)** — the Tauri/wry app, **launched on demand** (autostart, tray,
  or the user opening it). It **reads** perimeter/component state and **drives a thin control
  channel** to the daemon. It owns *no* perimeter state and **never tears the perimeter down**.
  It may be opened, closed, and may even crash; the daemon is unaffected. WebKit lives and dies
  with the viewer process — already transient after Phase A, now in a process that can fully exit.

The daemon is the single source of truth and the single owner of the perimeter. The viewer is a
projection in the strict sense: closing or crashing it is a no-op for the boundary.

### The daemon ↔ viewer contract

| Concern | Owner | Mechanism |
|---|---|---|
| Perimeter lifetime (up/down/pause/resume) | **daemon only** | daemon's existing `perimeter`/`podman` paths |
| `RunGuard` single-instance lock | **daemon only** | the daemon takes the lock; the viewer never does |
| Idle auto-pause + Telegram waker (ADR-0018) | **daemon only** | unchanged `idle.rs`, now never coupled to a window |
| Bootstrap (first-run, zero-trust, ADR-0011) | **daemon** | daemon runs it; viewer shows progress via reads |
| `~/.opentrapp/{dormant,paused,activated,credentials-ok,runguard.pid}` + `settings.json` + `.env` | **daemon writes; viewer reads** | the existing cross-process marker contract (already host-side, `idle.rs:105`) |
| Status/component projection (read) | viewer reads daemon | control channel `get_*` (or marker/state-file reads) |
| Commands that **mutate** perimeter state (restart/pause/resume/run-command) | viewer **requests**, daemon **executes** | control channel RPC — the viewer never calls podman itself |
| WebView lifetime | viewer only | transient (Phase A); dies with the viewer process |

**Two transport options for the control channel** (decision deferred to the implementation slice,
but the contract above is transport-independent):

- **(a) Extend the existing file-marker contract** with a small request inbox (e.g. a
  `~/.opentrapp/control/` request file the daemon watches, atomic-rename in, ack out). Zero new
  network surface; reuses what ADR-0018 already established; survives either process restarting;
  but request/response is coarse and polling-latent.
- **(b) A local Unix domain socket** owned by the daemon (`~/.opentrapp/daemon.sock`, `0600`,
  user-only, never a TCP port — preserves CLAUDE.md §10 "no network services"). Crisp RPC,
  immediate responses, natural for streaming status; costs a small framed-RPC layer and a
  reconnect story for daemon restarts.

Recommendation: **(b) a user-only Unix socket** for the live control/projection channel, keeping
**(a) the durable markers** as the source-of-truth state (so a viewer launched before the socket
is up still renders correct state, and a daemon restart re-derives state from disk, not from a
live connection). Markers are the truth; the socket is the fast path. This matches §11's "verify
at the consumption end": the durable markers are the consumed contract; the socket is an
optimization over them.

### Binary structure

`opentrapp_lib` already exists as the shared crate. The split is primarily a **bin/feature
reorganization, not a rewrite**:

- The orchestrator/lifecycle/idle/bootstrap/status modules move behind a headless entrypoint in
  the lib, with **no Tauri/wry imports on that path** (today some of them are reachable only via
  `#[tauri::command]` wrappers — those wrappers stay in the viewer; the underlying functions move
  to a transport-neutral API the daemon calls directly and the viewer calls over the channel).
- New `src/bin/daemon.rs` (or a second workspace member) builds **without the Tauri/wry
  dependency tree** — this is the crux: the daemon's `Cargo` feature set must not pull WebKit, or
  the whole exercise is moot. Verified by inspecting the daemon binary's linked libs (no
  `libwebkit2gtk`).
- The viewer keeps the `tauri`/`wry` deps and the `#[tauri::command]` handlers, but those handlers
  become **thin clients** of the control channel for anything that mutates perimeter state.

### Lifecycle & ownership rules (the invariants)

1. **The daemon owns the `RunGuard`.** Exactly one daemon per user. The viewer never acquires it;
   launching a second viewer is harmless (it just connects to the running daemon).
2. **The viewer never tears the perimeter down.** Closing/quitting the viewer closes the window
   and exits the *viewer* process; the daemon (and the perimeter, and the waker) keep running.
   "Quit OpenTrApp entirely" becomes an explicit *daemon* shutdown request over the channel — a
   distinct, deliberate action, not a side effect of closing a window.
3. **The daemon survives a viewer crash.** A `WebKitNetworkProcess` SIGBUS kills at most the
   viewer; the boundary is untouched. (This directly fixes the rc2-era fragility where a webview
   crash could take the perimeter down.)
4. **Autostart launches the daemon, not the viewer.** Boot/login starts `opentrapp-daemon`
   headless (~30–60 MB). The viewer is launched only when the user asks for it (tray, click). The
   resting state of a freshly-booted machine carries **no WebKit at all**.
5. **Bootstrap/first-run stays daemon-owned** but is *shown* by the viewer: the daemon runs the
   ADR-0011 zero-trust bootstrap; the viewer projects progress by reading the markers. (First-run
   still auto-opens the viewer for the wizard — the one case the viewer launches eagerly.)

## Consequences

### Positive

- **Resting RAM ~30–60 MB** (target) vs ~220 MB after Phase A — a true silent background process.
  The reclaim is *certain* because the always-on process never links WebKit.
- **The waker (ADR-0018) is structurally robust.** It lives in a process with no webview to crash
  it; idle auto-pause no longer depends on `prevent_exit()` keeping a WebKit-linked process alive.
- **The projection becomes honestly disposable** — the GUI can open/close/crash without touching
  the boundary, which is what "the GUI is just a projection" was always supposed to mean.
- **§11 boundary correctness:** the perimeter lifetime is decoupled from UI events; a UI failure
  can no longer make the boundary "alive but subtly wrong."

### Negative / costs

- **An ADR-level refactor.** The `#[tauri::command]` handlers that mutate perimeter state must be
  re-routed through the control channel; the daemon's dependency graph must be proven WebKit-free.
  This is the highest-risk change since the monorepo consolidation and must land behind its own
  verification gate, not bundled into a release.
- **A second process to supervise.** Autostart, restart-on-crash, and "is the daemon running?"
  become first-class concerns. The viewer must degrade gracefully when the daemon is absent
  (offer to start it) rather than assume in-process state.
- **A new (local, user-only) IPC surface** if option (b) is chosen. It is not a network surface
  (no TCP, `0600` socket), so CLAUDE.md §10 "no network services" still holds — but it is new
  attack surface between the viewer and the daemon and must be threat-modeled (a `docs/threat-model.md`
  row): only the local user can reach the socket; the daemon authenticates by filesystem
  ownership/permittion, not by trusting message content.
- **Two-process state coherence.** The markers-are-truth / socket-is-fast-path rule must be
  enforced so a stale socket view can never override on-disk truth (the §11 consumption-end is the
  marker, not the live message).

### Neutral / deferred

- **Transport choice (a vs b)** is deferred to the implementation slice; the contract table above
  is transport-independent, so either can be built without re-opening this ADR.
- **Phase A remains the fallback.** If the daemon cannot be made WebKit-free without unacceptable
  churn (e.g. a Tauri API the daemon genuinely needs), we stop at Phase A's ~220 MB resting and
  record that here — Phase A already shipped the make-or-break webview reclaim, so this split is an
  optimization on top, not a prerequisite for the memory goal being "good enough to ship."

## Verification (at the consumption end — CLAUDE.md §11)

The claim is **not** "the daemon builds" or "the daemon runs." The consuming ends are:

1. **No WebKit in the daemon.** Inspect the linked libraries of the built `opentrapp-daemon`
   binary (`ldd`/`otool`): **zero** `libwebkit2gtk`/`libwry`. If WebKit is linked, the split
   failed regardless of RAM numbers.
2. **Resting RSS.** `smem`/`podman stats` on a booted machine with the daemon up and no viewer:
   ~30–60 MB, no `WebKitWebProcess`, no GTK main process. Measured, not `free`.
3. **Boundary survives viewer death.** Kill the viewer (`kill -BUS` the webview process, or just
   crash it): the daemon, the perimeter, and the waker keep running; a Telegram message still
   wakes a dormant perimeter exactly once (ADR-0018's exactly-once contract, re-verified post-split).
4. **Ownership invariants.** Two viewers → one daemon (RunGuard held by the daemon); closing the
   viewer does **not** tear the perimeter down; an explicit "Quit" request **does**; autostart
   brings up the daemon headless (no WebKit) and not the viewer.
5. **Markers are truth.** Launch the viewer with the daemon's socket down → it still renders
   correct state from `~/.opentrapp` markers; bring the socket up → live updates resume. A stale
   socket view never overrides on-disk truth.
6. **Security-correct resume still holds.** Because the daemon owns resume, the ADR-0018 /
   §11 resume self-test (network isolation, credential inject, allowlist loaded, proxy CA
   unchanged, L3 filter active) runs in the daemon and must pass *after a daemon-driven resume*
   exactly as after a cold start.

Until items 1–3 are demonstrated on capable hardware (the reference laptop can host the GUI/daemon
alone but **not** the full perimeter under load — it swap-storms), the RAM and survival claims are
**unverified, not done** (§11), and are routed to capable hardware / CI.
