# Release notes — v0.7.0 (Runs quietly on a small laptop)

v0.7.0 is the footprint release. The goal of this cycle was to make OpenTrApp
behave like what it claims to be — a *silent background security wrapper* — so it
can run on a modest laptop without taking the machine hostage. The headline is
**idle auto-pause**: when your assistant has been idle for a while, the whole
perimeter pauses to near-zero memory and wakes the moment your next message
arrives. Alongside it, the resting set is lighter (the supply-chain and social
shields no longer idle in the background), the agent image is smaller on disk,
and the skill scanner can now borrow a model you already run instead of forcing a
download.

It also fixes a first-run dead-end that could block new users on packaged builds.

## What changed

### Idle auto-pause + wake-on-message — the big memory win

When the agent is idle past a configurable timeout (default ~12 minutes), the
perimeter pauses entirely and the app drops to a tiny resident footprint — only
the tray app and a lightweight host-side waker remain. The waker reuses the same
Telegram channel the setup wizard already established; it **peeks** for the next
incoming message (metadata only — it never advances the offset and never reads
message content), and on arrival it resumes the perimeter. Your assistant
reconnects and processes the queued message exactly once. Cold-start is a few
seconds.

This is **on by default**. A new "Let it sleep to save memory" toggle in
Preferences turns it off, and closing the window now hides to the tray (so the
waker survives a window close) rather than quitting. Design and the one
documented trade-off (a host-side metadata peek while dormant) are in
[ADR-0018](docs/adr/0018-idle-auto-pause-host-waker.md) and a new threat-model row.

### A lighter resting footprint

- **On-demand shields.** The supply-chain (`skills`) and social shields no longer
  run an idle daemon at boot. They start when a command needs them and stop after
  an idle grace, so the resting perimeter is three always-on containers instead of
  five — modest RAM, real hygiene.
- **Slimmer agent image.** The agent container dropped from ~754 MB to ~590 MB by
  stripping only files the runtime never reads (type declarations, source maps,
  `@types`). No package, no runtime asset removed. This is a disk/download/startup
  win, not a RAM one.

### Bring-your-own-model skill scanning + a hardened CDR

The skill scanner's Content Disarm & Reconstruction pipeline no longer forces a
specific local model. It speaks both Ollama and any OpenAI-compatible endpoint
(set in `config/cdr.conf`), so you can point it at a model you already run. The
default stays the lean `qwen2.5-coder:1.5b` — a measured A/B showed a larger model
*regressed* the reconstruction's post-verification, so bigger is not better here.
The retry-repair loop was also fixed so a valid reconstruction self-heals instead
of being false-quarantined, with deterministic regression tests covering the path.

### Measuring memory

A new `make profile-memory` (`tests/memory-profile.sh`) prints per-container RSS,
host memory, and image sizes — the before/after gate that drove the work above.

## Bug fixes

- **First-run no longer dead-ends on packaged builds.** Entering your API key and
  bot token and clicking *Continue* on the wizard's Connect step could fail with a
  "setting could not be saved" error, with no way forward. The wizard was writing
  the keys through the component-config editor, which on a packaged first-run
  resolves into the read-only application bundle. Credentials now go to the runtime
  configuration directly (where the perimeter actually reads them), and every other
  key-entry surface (re-credential, key rotation in Preferences) was moved onto the
  same path. Packaged-only — it never reproduced in a dev checkout.

## Breaking changes

None. No manifest contract change; existing manifests load unchanged.

## New runtime requirement (optional)

Unchanged from v0.6: the local-AI judgment rungs need [Ollama](https://ollama.com/)
reachable on the host with `qwen2.5-coder:1.5b`, `qwen2.5-coder:3b`, and
`all-minilm` pulled. Without it the fast static defences still run and the AI rungs
degrade fail-safe (hold for review). Skill-scanner CDR can now use any
OpenAI-compatible endpoint instead.

## Known issues

- **Idle auto-pause is new and default-on.** Its end-to-end resume path is
  CI-verified but has not yet been observed on memory-constrained hardware. If your
  assistant ever fails to wake on a message, turn the feature off in Preferences and
  it will stay resident.
- The Sentinel rung-1/rung-2 AI features remain inert (fail-safe) without Ollama.
- The live social adapter is opt-in and validated against AT Protocol; other
  networks are future adapters behind the same contract.

## Upgrade path

Standard auto-update — the Tauri updater will prompt in-app. To update manually,
download the installer for your platform from the assets below and run it over the
existing installation.

## Full commit range

`git log --oneline v0.6.0..v0.7.0` — the memory-optimization initiative (Phase 0
measurement harness; Phase 1 on-demand shields; Phase 2 agent-image slim; Phase 3
idle auto-pause + host-side wake-on-message, ADR-0018, default on), the skill-scanner
honest-docs audit + bring-your-own-model backend + CDR retry-loop fix and regression
tests, the OpenSSF Best Practices passing badge, and the packaged first-run
credential dead-end fix.
