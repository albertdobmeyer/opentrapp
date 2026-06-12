# Release notes — v0.7.2-rc1 (release candidate)

**This is a release candidate, not a general release.** It introduces the headless
**daemon/viewer split** (ADR-0019) and bundles the daemon in every installer, for
end-to-end validation on real hardware before v0.7.2 ships.

## Headline — the perimeter now has a headless owner

Until now, OpenTrApp's GUI *was* the perimeter owner: closing it (or a webview
crash) could take the whole security boundary down, and the resting footprint was
dominated by the ~220 MB GUI process. This RC ships a separate, **WebKit-free
`opentrapp-daemon`** that can own the perimeter on its own — bring it up, supervise
it, idle-pause it (ADR-0018), and tear it down — so the GUI can become a thin
**viewer** that comes and goes without touching the boundary.

- **The daemon ships inside this build** (a tauri sidecar, all platforms).
- The orchestration core (perimeter lifecycle, podman, manifest, workflows, the
  idle waker) was moved into a tauri-free `opentrapp-core` crate; CI asserts the
  daemon's dependency graph contains **no WebKit**.
- A durable control channel (`pause`/`resume`/`restart`/`shutdown`) lets a viewer
  drive the daemon; you can also drive it directly: `opentrapp-daemon --status`,
  `opentrapp-daemon pause`, etc.

## Important: the GUI-defer is OPT-IN and OFF by default

**Out of the box this build behaves exactly like v0.7.1** — the GUI still owns the
perimeter; nothing changes unless you opt in. The defer (GUI → daemon hand-off,
and the resting-memory win that comes with it) is enabled with the environment
variable **`OPENTRAPP_DAEMON_DEFER=1`** and is **experimental**: the behavior that
matters most (the daemon outliving the GUI, the ~30–60 MB resting footprint,
crash-survival) **cannot be verified in CI** — it needs a machine that can run the
full perimeter. Until that hardware verification passes, the memory win is a
*candidate*, not a shipped guarantee.

## Also in this RC (since v0.7.1-rc2)

- **Phase A leanness — now gate-verified.** The destroy-on-close webview behaviour
  (close the dashboard → ~211 MB freed, no per-cycle leak) was confirmed live on a
  packaged build; see `footprint-and-device-usability.md` §10.4.
- **Generic per-component dashboard (developer mode).** A manifest-driven view that
  projects any discovered component — the "the GUI is a projection of the
  submodules" vision, reachable from the dev sidebar.

## What to test

1. **Default (no opt-in):** install, complete the wizard, reach running — confirm
   it behaves exactly like v0.7.1 (the GUI owns the perimeter).
2. **The daemon, standalone:** run the bundled `opentrapp-daemon --status` /
   `--help` next to the app — confirm it reports state and the control verbs work.
3. **The defer (experimental):** set `OPENTRAPP_DAEMON_DEFER=1` and work through
   **`docs/b4b-hardware-test-plan.md`** — the 7-test checklist (daemon launch +
   ownership, the resting-memory win, control routing, idle auto-pause + wake,
   crash resilience, fallback). This is the gate to making the defer the default.

## Known issues / caveats

- The defer is opt-in/experimental as above; the resting-memory claim is scoped to
  "verified on capable hardware," not asserted for the shipped default (CLAUDE.md
  §11). Any defer failure falls back to the GUI self-owning — never a broken app.
- The control channel is polling-latent (bounded by the supervisor's ~30 s tick);
  a low-latency Unix-socket fast path over the same durable inbox is future work.
- As with prior RCs, end-to-end perimeter behaviour (idle auto-pause firing,
  wake-exactly-once) is what we ask you to confirm on perimeter-capable hardware.

## Full commit range

`git log --oneline v0.7.1-rc2..v0.7.2-rc1` — the full Phase B daemon split
(B1 workspace/core, B2 core migration, B3 daemon ownership, B4a control channel,
B4b sidecar bundling + GUI defer), the Phase A gate verification, and the
per-component manifest projection.
