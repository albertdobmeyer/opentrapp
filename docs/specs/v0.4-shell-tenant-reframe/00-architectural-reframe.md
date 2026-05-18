# 00 — Architectural Reframe

**Status:** Draft
**Companions:** [`01-state-machine`](01-state-machine.md), [`02-bootstrap-service`](02-bootstrap-service.md), [`03-activation-flow`](03-activation-flow.md), [`04-stop-and-recovery-ux`](04-stop-and-recovery-ux.md), [`05-bot-first-message`](05-bot-first-message.md), [`06-migration`](06-migration.md), [`07-container-name-cleanup`](07-container-name-cleanup.md)

## Context

Today's mental model — encoded in [`app/src/pages/Setup.tsx`](../../../app/src/pages/Setup.tsx) gating all first-run users behind a `<Navigate to="/setup" replace/>` redirect at [`app/src/App.tsx:114`](../../../app/src/App.tsx) — is *OpenTrApp = OpenClaw installer with a wizard*. Install ends with the perimeter up and the agent running.

This contradicts the architecture's own vocabulary. From [`CLAUDE.md`](../../../CLAUDE.md) §1: *"The application is the perimeter orchestrator: it composes the four containers, owns their lifetime, and exposes a manifest-driven GUI for user-facing operations."* From [`config/orchestrator-workflows.yml`](../../../config/orchestrator-workflows.yml) existing as a top-level cross-component layer: the codebase already thinks of itself as a *host that composes tenants*. The wizard-as-entry has been working *against* that architecture, not with it.

Three pieces of evidence the security model was always designed for shell/tenant separation:

1. **Placeholder key by design.** [`components/opencli-container/scripts/entrypoint.sh:54-60`](../../../components/opencli-container/scripts/entrypoint.sh) writes a literal placeholder string (`sk-ant-api03-placeholder-vault-proxy-will-inject-real-key-placeholder`) as the agent's auth profile. The agent was *required* to never see the real key — the proxy injects it at request time. The wizard collecting the key before the perimeter exists has been a UX choice, not a security necessity.
2. **Per-request env lookup in the proxy.** [`components/opencli-container/proxy/vault-proxy.py:176-181`](../../../components/opencli-container/proxy/vault-proxy.py) reads `ANTHROPIC_API_KEY` per request and logs a warning if absent — never gates startup. The proxy is engineered to run idle, with no key, indefinitely.
3. **One-directional `depends_on`.** [`compose.yml`](../../../compose.yml) declares `vault-agent.depends_on: vault-proxy` only. Forge and pioneer have no startup dependency on the agent. The shell (proxy + forge + pioneer) and the tenant (agent) are already separable in the compose graph.

Empirically verified: OpenClaw with empty `TELEGRAM_BOT_TOKEN` boots cleanly with Telegram silently disabled (`components/opencli-container/docs/phase1-findings.md:134`). The agent container itself is happy to run without credentials; we just don't need it to.

## The reframe in concrete terms

### Phase 1 — Install (silent, no decisions)
Karen runs the OS installer. The Tauri app appears in the system tray. End. No wizard, no terminal, no choices.

### Phase 2 — Bootstrap (background, automatic)
The tray app, on first run, runs everything that doesn't require Karen's input. Pipeline detail in [`02-bootstrap-service.md`](02-bootstrap-service.md). Summary:
1. Detect Podman/Docker; install if absent (Podman sidecar)
2. Copy [`.env.example`](../../../components/opencli-container/.env.example) to `.env` if missing
3. Build local images (vault-agent, vault-forge, vault-pioneer — all `build:` stanzas)
4. Pull remote images (mitmproxy)
5. Bring up the **shell** containers only: `podman compose up -d vault-proxy vault-forge vault-pioneer`
6. Verify shell health

Karen sees: tray icon goes amber → green. One OS-level admin/sudo prompt during step 1 on Windows/macOS; framed as *"Setting up your safe room — your computer will ask for permission."*

### Phase 3 — Activation (just-in-time, user-triggered)
Exactly one entry point: a **Launch your assistant** button in [`HeroStatusCard.tsx`](../../../app/src/components/HeroStatusCard.tsx) when state is `(ShellReady, Absent)`. Click opens a modal over the home screen (not a route navigation — see [`03-activation-flow.md`](03-activation-flow.md)). Two real steps:
- Anthropic API key entry, with a single-token live `/v1/messages` ping
- Telegram bot setup with the BotFather walkthrough plus a test-message that uses a polling-handoff sequence so the wizard and OpenClaw don't collide on `getUpdates`

On commit: write keys to `.env`, `compose up -d --force-recreate vault-proxy` (proxy reads env at process start), `compose up -d vault-agent`, persist activation marker. State transitions to `(ShellReady, Running)`.

### Phase 4 — Running
Today's happy path. Hero status shows "running safely" with three affordances: **Open Telegram** (primary) · **Stop your assistant** (single button replacing today's Pause; calls the existing `pause_perimeter` primitive which preserves all volumes — see [`04-stop-and-recovery-ux.md`](04-stop-and-recovery-ux.md)).

The bot's *first* message to Karen on Telegram is its tutorial: warm welcome + three tappable example prompts as an inline keyboard. See [`05-bot-first-message.md`](05-bot-first-message.md).

### Subsequent activations are one-click
After Phase 3 has succeeded once, marker files persist. Subsequent app launches: install (already done) → bootstrap (idempotent, fast) → if `~/.opentrapp/activated` and `credentials-ok < 7 days old`: bring up agent automatically, no wizard. See [`06-migration.md`](06-migration.md).

## Refactor / rebuild / new (honest categorization)

| Surface | Status | Notes |
|---|---|---|
| `compose.yml` topology, networks, volumes | **Untouched** | Container-name cleanup ([`07`](07-container-name-cleanup.md)) is a separable PR |
| `vault-proxy.py` (key injection, allowlist, exfil thresholds) | **Untouched** | |
| `entrypoint.sh` (config, CA cert, placeholder auth) | **Untouched** | New CONSTRAINTS section added in submodule; existing structure unchanged |
| Schema-alignment trio | **Untouched** | No new manifest fields needed |
| 87 MITRE patterns / CDR pipeline | **Untouched** | |
| Seccomp profiles, cap drops | **Untouched** | |
| 28-banned-term test | **Untouched** | Extended with new surfaces' coverage |
| Wizard step components (`ConnectStep`, `InstallStep`, `ReadyStep`) | **Refactored** | Components survive; mounted in modal instead of route; gain live-ping + test-message validation |
| `HeroStatusCard.tsx` | **Refactored** | New `(ShellReady, Absent)` state with launch button; Pause→Stop renaming; recovery card hookup |
| `Setup.tsx` route | **Refactored** | Stays as deep-link fallback; no longer the install gate |
| `lifecycle.rs::bring_perimeter_up_async` | **Rebuilt** | One-line function becomes pipeline driver |
| `PerimeterState` enum | **Rebuilt** | 5-variant enum replaced by `(BootstrapState, TenantState)` pair |
| `compute_perimeter_status` | **Rebuilt** | Reads marker files + bootstrap subsystem state, not just running counts |
| `PerimeterStateStore` | **Rebuilt** | Gains `bootstrap_progress` field, new persistence semantics |
| Migration code path | **New** | No analog today |
| Background bootstrap subsystem | **New** | Whole pipeline driver |
| Podman bootstrapper sidecar | **New** | Tauri sidecar binary, three OS variants |
| `/v1/messages` ping helper | **New** | Used in activation, weekly re-validation, migration |
| Telegram polling-handoff orchestration | **New** | `deleteWebhook` → `getMe` → `getUpdates` → `sendMessage` → confirm-by-offset |
| Marker files `activated`, `credentials-ok` | **New** | Extending the existing `paused` pattern at [`lifecycle.rs:114-132`](../../../app/src-tauri/src/lifecycle.rs) |
| Failure-cause taxonomy + recovery card | **New** | |
| Bootstrap progress events | **New** | 3 new event types alongside existing `perimeter-state-changed` |
| Single-instance plugin | **New** | `tauri-plugin-single-instance`, ~10 LOC |
| Tray icon variants (amber/green/red) | **New** | 3 PNG files + `set_icon()` calls |
| Bot first-message tutorial | **New** | Submodule PR in opencli-container; new CONSTRAINTS section |

**Net read:** the security architecture is preserved exactly. The entry/lifecycle subsystem is rebuilt on top of it, with substantial new infrastructure (bootstrap pipeline + sidecar). The Anthropic narrative for this work: *"we keep the security architecture verbatim and rebuild the onboarding and lifecycle subsystem to match it."*

## Pluggable-shell scoping (honest)

The shell/tenant split structurally generalizes: a neutral host that pre-flights a perimeter and waits for a tenant; OpenClaw is the first tenant. Adding a second tenant (OpenCode, a future Anthropic agent SDK release, anything) is *structurally* a matter of another `commands.json`, another set of container definitions, another card on the home screen.

**v0.4 ships only the OpenClaw tenant.** Spec language is "this architecture makes future tenants cheap" — never "multi-tenant today."

## Verification approach

End-to-end dogfood scenario on a clean machine:

1. Fresh install — installer runs to completion, app appears in tray, no other UI
2. Tray icon transitions amber → green within ~5-8 minutes (image build + pull dominant)
3. Click tray → home shows "Ready to launch" with the launch button as the primary affordance
4. Click launch → modal opens with two steps; Anthropic ping returns 200; Telegram /start arrives in wizard; test message arrives on Karen's phone
5. Wizard commits → tray status changes to "running safely"; bot's first Telegram message is the tutorial with three tappable prompts
6. Quit and relaunch — app comes back up to "running safely" without re-running the wizard

Failure-path verification:
- Cancel admin prompt during Podman install → tray goes red, recovery card surfaces
- Disconnect network during image pull → bootstrap retries with backoff, then surfaces recovery card
- Wrong Anthropic key during activation → live ping fails inline, wizard shows error, no .env write
- Stop button click → all four containers stop, volumes preserved, hero shows "Stopped" with Resume

## Out of scope

- **Bundled Podman.** v1.0 destination; sidecar interface designed so the swap is internal
- **OS-level code-signing.** Procurement-bound, separate track
- **OS keychain credential migration.** Plaintext `.env` stays for v0.4; keychain is a v1.0 conversation that requires redesigning the proxy's env-var injection
- **Demo video.** Tracked in `HUMAN-TODO.md` §5; depends on this reframe shipping first
- **Generalizing to a second tenant.** Architecture supports it; not built
- **Embedded webview for Anthropic console signup.** Manual modal stays; live-ping closes the "wrong key" failure mode
- **Webhook-based Telegram model.** Long-polling is OpenClaw's default; webhooks would require a public HTTPS endpoint on the user's laptop, which `CLAUDE.md` §10 forbids
