# v0.4 — Shell/Tenant Reframe

**Status:** Draft (8 specs, 2026-05-09)
**Vision:** OpenTrApp becomes a security shell that runs after install. OpenClaw is a tenant the user activates just-in-time when they're ready. Install and activation are two separate decisions, not one.

## Why this exists

Today's UX collapses two genuinely different decisions — *"I installed your app"* and *"I committed to running an autonomous AI agent"* — into a single first-launch wizard. The architecture has always supported the separation (placeholder API key by design, per-request env lookup in the proxy, one-directional `depends_on`); the wizard-as-entry was a UX anachronism. This overhaul aligns the entry model with the architecture that was already there.

For the contradiction in the architecture's own vocabulary that motivates this work, see [`00-architectural-reframe.md`](00-architectural-reframe.md) §Context.

## Reading order

```
00 — architectural reframe          ← read first; the umbrella
├── 01 — state machine                bootstrap × tenant axes
├── 02 — bootstrap service            background subsystem + Podman sidecar
├── 03 — activation flow              JIT wizard + Telegram handoff + Anthropic ping
├── 04 — stop & recovery UX           one-button Stop, recovery card, tray icons
├── 05 — bot first-message            tutorial in Telegram on user's first /start
├── 06 — migration                    existing-install detection + verification
└── 07 — container-name cleanup       small precondition PR (lands first)
```

00 establishes the "why" and the surface map. 01 + 02 are the design-heavy backend specs. 03 + 04 + 05 cover the user-visible surfaces. 06 covers existing users. 07 is a small standalone PR that unblocks project-isolated testing for everything else.

## Implementation sequence

The specs above are organized by concept; the PRs that implement them ship in this dependency order:

1. **PR-1: container_name cleanup** ([`07`](07-container-name-cleanup.md)) — small, low-risk, unblocks isolated testing for the rest
2. **PR-2: state machine refactor** ([`01`](01-state-machine.md)) — `BootstrapState` + `TenantState` enums, watchdog refactor, marker files. No behaviour change yet; just the new shape.
3. **PR-3: bootstrap service** ([`02`](02-bootstrap-service.md)) — the new subsystem, Podman sidecar wiring, single-instance plugin
4. **PR-4: activation flow** ([`03`](03-activation-flow.md)) — JIT wizard repositioning, Telegram handoff, Anthropic live-ping
5. **PR-5: stop & recovery UX** ([`04`](04-stop-and-recovery-ux.md)) — one-button Stop, recovery card, tray icon variants
6. **PR-6: bot first-message** ([`05`](05-bot-first-message.md)) — submodule PR in opencli-container, plus parent reference bump
7. **PR-7: migration** ([`06`](06-migration.md)) — existing-install detection + live-ping verification

PR-1 lands first and blocks none of the others. PR-2 through PR-7 each have natural review checkpoints before the next begins. The whole sequence is ~6-8 reviewable PRs.

## Cross-spec invariants

These names and shapes are stable across all 8 specs — implementing agents must keep them aligned:

| Concept | Canonical form |
|---|---|
| Two enums | `BootstrapState ∈ { Installing, Bootstrapping, ShellReady, ShellFailed }` and `TenantState ∈ { Absent, Activating, Running, Paused, Errored }` |
| Combined state | `(BootstrapState, TenantState)` — pair, not a flat enum |
| Marker files | `~/.opentrapp/{paused,activated,credentials-ok}` — paused already exists |
| New events | `bootstrap-step-started`, `bootstrap-step-progress`, `bootstrap-step-failed` (alongside the existing `perimeter-state-changed`) |
| Stop primitive | `pause_perimeter` (calls `compose stop`, preserves all volumes) — *never* `nuclear-kill` or `hard-kill` for user-facing Stop |
| Failure causes | `podman-install-failed`, `podman-install-denied`, `image-build-failed`, `image-pull-failed`, `image-pull-cancelled`, `image-pull-denied`, `runtime-misdetected`, `network-unavailable`, `shell-up-failed` (canonical taxonomy in [`04-stop-and-recovery-ux.md`](04-stop-and-recovery-ux.md)) |

## Verification approach

Each spec has its own verification section. End-to-end coverage:

- **Unit:** Rust tests in `app/src-tauri/src/lifecycle.rs` and the new bootstrap module
- **Integration:** test environments with Podman pre-installed, simulating partial states
- **E2E:** Playwright in `app/e2e/` covers phase-transition sequences and recovery paths
- **Manual smoke:** clean VMs per OS (macOS Sequoia, Windows 11, Ubuntu 24.04) — at least once per release cut

## Honest scoping

The shell/tenant split structurally generalizes (the architecture earns "pluggable security shell" framing), but **v0.4 ships only the OpenClaw tenant**. Spec language is "this architecture makes future tenants cheap" — never "multi-tenant today." That's the stronger and more honest claim.

OS-level code-signing (Apple Developer ID, Authenticode), bundled Podman, demo video — all out of scope. Tracked as v1.0 destinations or operator-driven items in `HUMAN-TODO.md`.

## Source

The brainstorming sketch that seeded this spec set — the "Karen from HR" UX — is preserved in [`_source-karen-from-hr.md`](_source-karen-from-hr.md). It is not a spec; it is the original prompt-response that the eight specs above formalize and reconcile against the existing architecture.
