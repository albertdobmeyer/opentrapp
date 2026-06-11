# Release notes — v0.7.1-rc2 (release candidate)

**This is a release candidate, not a general release.** It bundles a substantial run
of fixes + a leanness pass on top of rc1, for end-to-end validation on real hardware
before v0.7.1 ships.

## Why this RC exists

rc1 was never actually tested as a build (an old v0.6.0 install was being run by
mistake). In the meantime, live testing on a constrained laptop surfaced — and this RC
fixes — a chain of issues that made the headline features either inert or fragile:

- **Idle auto-pause was silently doing nothing**, for two independent reasons, both fixed:
  1. **The egress log never persisted** (a rootless-podman volume permission problem) — the
     idle signal had nothing to read. Fixed by chowning the log volume to the proxy user in
     the container entrypoint.
  2. **The idle signal counted the agent's own keep-alive polling as activity**, so it never
     went stale. Fixed by measuring idle from the last *real-activity* request and ignoring
     Telegram keep-alive polls. (Verified live: the corrected signal climbs toward the idle
     threshold where the old one stalled at ~15 s.)
- **The app was far heavier than we thought, and crashed under pressure.** A live measurement
  showed the **GUI is ~442 MB** (the WebKitGTK webview) — the single heaviest part of the app,
  *more than the whole perimeter*, and the cause of a `WebKitNetworkProcess` SIGBUS crash. This
  RC makes the dashboard webview **transient**: it's destroyed on close (freeing ~222 MB),
  leaving a lean tray-only background daemon (watchdog + idle waker + perimeter). The webview is
  rebuilt on demand when you open the dashboard.
- **The skills scanner didn't actually run in its container.** Component commands ran on the
  *host*; the supply-chain scanner now runs **inside** the `vault-skills` container via
  `podman exec` (untrusted content never touches the host; available in packaged builds).
- **Clean shutdown + a leaner privileged container.** The skills/social gears now shut down
  cleanly on stop (no 10 s SIGKILL hang); `vault-egress` dropped its unused Node runtime
  (alpine base — attack-surface cut in the one NET_ADMIN container).

## What to test

1. Install, complete the wizard, reach the running state.
2. **Lean GUI:** close the dashboard window → it should disappear from memory (the perimeter
   keeps running in the tray). Re-open it from the tray icon → it rebuilds. Quit from the tray →
   the perimeter tears down.
3. **Idle auto-pause:** leave it idle ~15 min → confirm it drops to *Dormant* and the perimeter
   stops (RAM drops); send a Telegram message → it wakes and replies exactly once.
4. **Skills scan:** run a skill scan → confirm it completes (it now runs in the container).

## Known issues / caveats

- This RC's per-process memory win (destroy-on-close) is what we ask you to confirm; if anything
  feels off, the dashboard can be reopened from the tray at any time.
- The headless-daemon split (Phase B) and the per-component GUI projection (Phase C) are the
  next architectural steps, not in this RC.

## Full commit range

`git log --oneline v0.7.1-rc1..v0.7.1-rc2` — the ZONE 3 entrypoint-chown, the non-poll
idle-signal fix, the SIGTERM clean-shutdown, the Phase A lazy-window leanness, the in-container
`podman exec` channel, the alpine egress base, and the footprint-doc corrections.
