# Running OpenTrApp headless (GUI-free)

OpenTrApp can be operated entirely from the command line, with no desktop GUI.
This is the lean way to run it, and it is the operational form of the headless
daemon split ([ADR-0019](adr/0019-headless-daemon-gui-viewer-split.md)) on the
way to the full CLI-first direction ([ADR-0020](adr/0020-product-identity-and-distribution.md),
[ADR-0022](adr/0022-daemon-control-surface.md)).

## Why headless

The perimeter daemon (`opentrapp-daemon`) links only `opentrapp-core` and
`tokio`. It has no Tauri, wry, WebKit, or GTK dependency, and CI fails the build
if that ever changes. So building and running it skips the entire desktop GUI
toolchain:

- **Lean.** The daemon is roughly 30 to 60 MB resident, versus roughly 442 MB
  for the Tauri GUI with its dashboard open (which is more than the whole
  five-container perimeter at rest).
- **Better co-tenancy.** On a small machine, dropping the GUI frees enough RAM
  to run the perimeter comfortably alongside an editor and a browser.
- **The same engine.** The daemon owns the perimeter lifecycle, the idle
  auto-pause and wake-on-message waker, the boundary self-test, and the durable
  state markers. The GUI, when present, is only a viewer over this same daemon.

## Build

```bash
make daemon
# → cargo build -p opentrapp-daemon --release  (no GUI, no WebKit/GTK)
# → app/src-tauri/target/release/opentrapp-daemon
```

This compiles only the daemon and its two dependencies, not the GUI crate, so it
is faster and lighter than a full app build.

## Operate

The operator surface is `opentrapp-daemon vault <verb>` (the CLI-first alias
layer; the bare `opentrapp` command arrives with the GUI demotion in a later
phase, ADR-0022):

```bash
opentrapp-daemon vault up        # take ownership and bring the perimeter up
opentrapp-daemon vault status    # print the durable perimeter state and exit
opentrapp-daemon vault verify    # run the boundary self-test (network isolation,
                                 #   credential injection, allowlist, proxy CA, L3 filter)
opentrapp-daemon vault pause     # idle-pause to dormant and arm the waker
opentrapp-daemon vault resume    # wake from dormant
opentrapp-daemon vault restart   # cycle the perimeter
opentrapp-daemon vault down      # tear the perimeter down
```

Run the daemon as a long-lived owner with no arguments (it brings the perimeter
up, supervises idle pause and wake, and tears down cleanly on SIGTERM or SIGINT):

```bash
opentrapp-daemon          # or: make daemon-run
```

`vault status` and the marker files in `~/.opentrapp/` are the source of truth;
the daemon and any viewer both read them.

## The honest constraint on small machines

The daemon itself is lean, but `vault up` brings the full perimeter up, and the
`vault-agent` container has a startup spike of roughly 1.35 GB (the agent
runtime, intrinsic to running the agent at all). On a machine with under about
8 GB of RAM, close heavy applications (editor, browser) before bringing the
perimeter up. With the box cleaned, the full perimeter and the T0 boundary
self-test have been verified to run on a 7.2 GB laptop with roughly 3.6 GB free,
no swap-storm, passing cold and after resume (see the note in the `Makefile`
perimeter section, recorded 2026-06-16).

So the swap-storm people hit on a small box is co-tenancy with the desktop
environment, not the perimeter being too heavy. Running headless removes the
single largest co-tenant, the GUI.

## What this is not (yet)

Headless operation works today, but the full CLI-first product still has open
work, tracked in ADR-0022:

- The user-facing command is still `opentrapp-daemon`, not a unified `opentrapp`.
- The headless daemon is opt-in for the bundled app (`OPENTRAPP_DAEMON_DEFER=1`);
  the default installer still launches the GUI, which self-owns the perimeter.
- There is no live status-streaming API yet (the CLI reads markers and stderr);
  the on-demand loopback web panel is scaffolded but excluded from the build,
  gated on the de-risking spike in ADR-0022.
- OS autostart still launches the GUI, not the daemon.

Those are the remaining steps to make daemon-plus-CLI the default and the GUI a
fully optional, separately installed projection.
