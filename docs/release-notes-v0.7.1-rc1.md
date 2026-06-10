# Release notes — v0.7.1-rc1 (release candidate)

**This is a release candidate, not a general release.** It exists to validate the
idle-auto-pause fixes end-to-end on real hardware before v0.7.1 ships. Use it for
testing; it is not yet recommended for everyday use.

## Why this RC exists

v0.7.0 (drafted, never published) introduced **idle auto-pause** — the perimeter
sleeps to near-zero memory when idle and wakes on your next Telegram message. Live
testing on a constrained laptop then proved that feature was, as shipped in the
v0.7.0 build, **silently inert** for two independent reasons, both now fixed:

1. **The egress log didn't persist** (a rootless-podman volume-permission issue):
   the idle signal had nothing to read. Fixed by chowning the log volume to the
   proxy user before it drops privileges.
2. **The idle signal counted the agent's own keep-alive polling as activity** —
   the agent long-polls Telegram every ~10–20 s forever, so "time since any
   egress" never went stale. Fixed by measuring idle from the last *real-activity*
   request (an LLM call, a tool fetch, a reply) and ignoring keep-alive polls.

Both fixes are unit-tested and verified live at the signal level (the corrected
signal climbs toward the idle threshold during idle, where the old one stalled).
**This RC is for confirming the last mile: that the app actually drops to dormant
and wakes correctly on real hardware.**

## What to test

1. Install, complete the wizard (Anthropic key + Telegram bot token), reach the
   running state.
2. Leave it **completely idle ~15 min** (default threshold ~12 min).
3. Confirm it switches to a *Dormant / "sleeping to save memory"* state and the
   perimeter stops (host RAM drops to roughly the app shell).
4. Send one message to your bot → confirm it **wakes within a few seconds and
   replies exactly once**.

(Full procedure, including the post-resume boundary checks, is in
`docs/perimeter-test-handoff.md`.)

## What this RC carries (the v0.7.x changes)

- **Idle auto-pause + wake-on-message** (default on) — now with both fixes above.
- **Lighter resting footprint** — on-demand supply-chain/social shields; slimmer
  agent image. Measured resting perimeter ≈ 0.4 GB.
- **Packaged first-run fix** — the wizard now writes credentials to the runtime
  config directly (the v0.7.0 first-run dead-end is gone).
- **Bring-your-own-model skill scanning** + the CDR retry-loop fix.
- Loud proxy-log fallback (a persistence failure now alarms instead of hiding).

## Known issues / caveats

- Idle auto-pause's end-to-end sleep/wake is exactly what this RC asks you to
  confirm; if your assistant ever fails to wake, turn the feature off in
  Preferences (it then stays resident).
- The always-on proxy's long-session memory growth is still under measurement
  (a separate work item); not a blocker for this RC.

## Full commit range

`git log --oneline v0.7.0..v0.7.1-rc1` — the ZONE 3 entrypoint-chown fix, the
non-poll idle-signal fix, and the verification discipline / footprint
documentation that surrounded them.
