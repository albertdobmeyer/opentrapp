# 01 — State Machine: Bootstrap × Tenant

**Status:** Draft
**Parent:** [`00-architectural-reframe`](00-architectural-reframe.md)

## Why two axes

Today's [`PerimeterState`](../../../app/src-tauri/src/lifecycle.rs) enum (`NotSetup` / `Starting` / `RunningSafely` / `Recovering` / `Stopped`) maps `running_count` of the four containers linearly — 4-of-4 is healthy, 1-3-of-4 is `Recovering`, 0-of-4 is `Stopped`. This works when the only valid configuration is "all four up." After the reframe, the valid configurations are:

- 3-of-4 (proxy + forge + pioneer) with agent intentionally absent → **healthy** (shell ready, no tenant)
- 4-of-4 → **healthy** (shell ready, tenant running)
- 1-2-of-4 → unhealthy (recovering or shell-failed, depending on which)

The single-axis enum can't represent "3-of-4 is healthy iff the user hasn't activated yet." Two orthogonal axes make this expressible.

## The two axes

```rust
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapState {
    Installing,      // First-launch: writing .env, no containers yet
    Bootstrapping,   // Podman install / image build+pull / shell-up in progress
    ShellReady,      // Proxy + forge + pioneer up; CA cert exchanged
    ShellFailed,     // Bootstrap halted; recovery card surfaces cause
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TenantState {
    Absent,          // No agent container; not yet activated, or post-stop
    Activating,      // Wizard committed, bringing agent up
    Running,         // Agent container up; bot operational
    Paused,          // User-initiated stop; persisted via marker file
    Errored,         // Agent container expected up but isn't
}
```

The pair `(BootstrapState, TenantState)` is the perimeter's high-level state. Reported up to the frontend as a struct via the existing [`perimeter-state-changed`](../../../app/src-tauri/src/lifecycle.rs) event channel.

> **Note:** `TenantState::Paused` covers what the existing UX called "Pause." Per [`04-stop-and-recovery-ux.md`](04-stop-and-recovery-ux.md), the user-facing copy becomes "Stopped"; the underlying primitive (`pause_perimeter` → `compose stop`) is unchanged. The variant name stays `Paused` to match the existing marker file (`~/.lobster-trapp/paused`) and Rust function naming.

## Allowed combinations and `HeroStatusCard` mapping

Concrete reference to the existing component: [`app/src/components/HeroStatusCard.tsx`](../../../app/src/components/HeroStatusCard.tsx) renders state-specific UI via a `COPY` constant (lines 27-70) and conditional buttons (lines 140-227). Each new state below adds one row to `COPY` and one branch to the button render block.

| `(bootstrap, tenant)` | Label | Sub-line | Primary affordance | Notes |
|---|---|---|---|---|
| `(Installing, Absent)` | "Getting your assistant ready…" | "We're setting things up in the background." | (none — passive) | Brief, sub-second; .env write |
| `(Bootstrapping, Absent)` | "Setting up your safe room…" | "First-time setup, ~5 minutes. You can keep working." | (cancel button on long pulls) | Phase 2; image build/pull/shell-up |
| `(ShellReady, Absent)` | "Ready to launch your assistant" | "Two quick steps and you're chatting on Telegram." | **Launch your assistant** | Phase 3 entry point |
| `(ShellReady, Activating)` | "Starting your assistant…" | "Almost there." | (none — wait) | Brief, ~3-5s |
| `(ShellReady, Running)` | "Your assistant is running safely" | "Open Telegram to start chatting." | Open Telegram · **Stop your assistant** | Today's happy path |
| `(ShellReady, Paused)` | "Your assistant is stopped" | "Tap Resume when you're ready." | **Resume** | Replaces today's "Paused" copy |
| `(ShellReady, Errored)` | "Your assistant didn't recover" | "Something stopped it from running. We can try to fix it." | **Try to fix** | Existing error flow |
| `(ShellFailed, Absent)` | "Background setup needs your help" | (cause-specific copy from taxonomy in [`04`](04-stop-and-recovery-ux.md)) | **Recovery card** | Phase 2 recovery surface |

## Disallowed combinations

| `(bootstrap, tenant)` | Why disallowed | What we do |
|---|---|---|
| `(Installing, *!=Absent)` | Tenant can't exist before bootstrap | Defensive transition: force tenant to `Absent`; log warning |
| `(Bootstrapping, *!=Absent)` | Same | Same |
| `(ShellFailed, *!=Absent)` | Tenant runs only on a healthy shell | Force tenant to `Absent`; if agent container is somehow up, stop it |

These shouldn't occur in normal flow but are defensive guards against state corruption (manual `podman` intervention, partial crashes).

## Persistence model

Bootstrap state is **computed** on every watchdog poll from observable system state, never persisted:
- `Installing`: `.env` doesn't exist
- `Bootstrapping`: bootstrap subsystem reports an in-flight step via in-memory state in `PerimeterStateStore`
- `ShellReady`: proxy + forge + pioneer all running AND CA cert present at `proxy-ca` volume
- `ShellFailed`: bootstrap subsystem reports a halted-with-cause state

Tenant state is **mostly computed**, with three persisted markers extending the existing pattern at [`lifecycle.rs:114-132`](../../../app/src-tauri/src/lifecycle.rs):

| Marker file | Meaning | Set when | Cleared when |
|---|---|---|---|
| `~/.lobster-trapp/paused` | User intent to stay stopped (already exists) | User clicks Stop | User clicks Resume |
| `~/.lobster-trapp/activated` | Activation flow has succeeded at least once | Activation commits successfully | User clicks "Reset assistant" in Preferences |
| `~/.lobster-trapp/credentials-ok` | Last credential validation succeeded; contains a unix-millis timestamp | After a successful `/v1/messages` live-ping | On 401 response from Anthropic; on `.env` rewrite |

Tenant computation:
- `Activating`: bootstrap subsystem reports `compose up -d vault-agent` in flight
- `Running`: agent container running AND not paused
- `Paused`: paused marker present (regardless of container state, per existing pattern at [`lifecycle.rs:99-107`](../../../app/src-tauri/src/lifecycle.rs))
- `Errored`: agent expected up (activated marker present, not paused) but container absent
- `Absent`: not activated, or post-reset

## Watchdog refactor

[`compute_perimeter_status()` in `lifecycle.rs:333-356`](../../../app/src-tauri/src/lifecycle.rs) currently:

```rust
let state = match running_count {
    0 => PerimeterState::Stopped,
    n if n == PERIMETER_CONTAINERS.len() => PerimeterState::RunningSafely,
    _ => PerimeterState::Recovering,
};
```

After the refactor:

```rust
fn compute_status(observed: &Observed) -> PerimeterStatus {
    let bootstrap = compute_bootstrap_state(observed);
    let tenant = compute_tenant_state(observed, &bootstrap);
    PerimeterStatus { bootstrap, tenant, containers: observed.containers.clone(), .. }
}
```

`compute_bootstrap_state` reads:
- Container presence for vault-proxy, vault-forge, vault-pioneer
- CA cert presence (read from `proxy-ca` volume via `podman exec`)
- The bootstrap subsystem's in-flight reports from `PerimeterStateStore.bootstrap_progress`

`compute_tenant_state` reads:
- Agent container running flag
- Marker files (`paused`, `activated`)
- Bootstrap state (tenant can't be anything but `Absent` if shell isn't ready)

`restart: unless-stopped` interaction is unchanged — it self-heals individual container deaths during `Running`. **Verified semantics:** an explicit `compose stop vault-agent` does not auto-restart under `unless-stopped` (compose-spec contract: "always restart, *unless* the user stopped it"). The shell-without-tenant state is therefore stable; Podman won't fight us. The watchdog stays a reporter, not a controller.

## `PerimeterStateStore` extensions

Today's struct at [`lifecycle.rs:86-108`](../../../app/src-tauri/src/lifecycle.rs):

```rust
pub struct PerimeterStateStore {
    pub status: Mutex<PerimeterStatus>,
    pub paused: RwLock<bool>,
}
```

Refactored:

```rust
pub struct PerimeterStateStore {
    pub status: Mutex<PerimeterStatus>,
    pub paused: RwLock<bool>,                       // unchanged; mirrors marker file
    pub activated: RwLock<bool>,                     // mirrors `~/.lobster-trapp/activated`
    pub credentials_ok_at: RwLock<Option<u64>>,      // unix-ms; None means unverified
    pub bootstrap_progress: RwLock<Option<BootstrapProgress>>,  // in-flight pipeline step
}
```

`BootstrapProgress` is defined in [`02-bootstrap-service.md`](02-bootstrap-service.md).

## Tray icon mapping

Three icon variants in [`app/src-tauri/icons/`](../../../app/src-tauri/icons/), called via `tray.set_icon()` from [`update_tray_for_state()`](../../../app/src-tauri/src/lifecycle.rs):

| Icon | States |
|---|---|
| Amber | `Installing × Absent`, `Bootstrapping × Absent`, `ShellReady × Absent` (ready to launch), `ShellReady × Activating`, `ShellReady × Paused` (intentional "off") |
| Green | `ShellReady × Running` |
| Red | `ShellFailed × Absent`, `ShellReady × Errored` |

> Paused is amber, not red. Pause is intentional and reversible; red implies something's wrong. Amber says "intentionally off, you can resume." This is a deliberate revision from the original draft.

macOS template-image rules: use template PNGs that auto-tint to system menubar contrast. The color-by-state model degrades gracefully on macOS to a state-shape model if template tinting fights us.

## Migration handling

Detail in [`06-migration.md`](06-migration.md). Summary: existing installs (with non-placeholder `.env` keys + `wizardCompleted: true`) get one verification live-ping on first launch of v0.4. If it succeeds, markers are written and state computes to `(ShellReady, Running)` with no UX surface change. If it fails, state computes to `(ShellReady, Absent)` and the activation modal opens for re-credentialing.

## Test coverage

Unit tests in [`app/src-tauri/src/lifecycle.rs`](../../../app/src-tauri/src/lifecycle.rs):
- All allowed `(bootstrap, tenant)` pairs serialize to expected JSON shape
- Disallowed pairs trigger defensive transitions (verified via deterministic mock observed-state)
- Marker-file presence/absence drives tenant state correctly
- `redact_secrets()` round-trip on failure-cause payloads doesn't leak keys

Frontend unit tests in [`app/src/components/HeroStatusCard.test.tsx`](../../../app/src/components/) (new):
- Each `(bootstrap, tenant)` pair renders the documented label, sub-line, and affordance
- Disabled states (e.g. "Launch" button is grayed during `Bootstrapping × Absent`) match spec
- Buttons fire the documented Tauri commands

E2E in [`app/e2e/`](../../../app/e2e/):
- Phase-transition sequence: `Installing → Bootstrapping → ShellReady × Absent → ShellReady × Activating → ShellReady × Running`
- Recovery: simulate shell-up-failed; assert recovery card renders; click retry; assert state recovers
- Subsequent-launch: assert no wizard re-shows when markers present

## Out of scope

- Persisting bootstrap state across launches (it's computed, not persisted, by design — simpler and self-correcting)
- Multi-tenant state (architecture supports it; the state machine only encodes one tenant for v0.4)
- Cross-platform tray-icon-color edge cases beyond macOS template-image handling
- Auto-recovery from `ShellFailed × Absent` (user-initiated retry only — see [`04-stop-and-recovery-ux.md`](04-stop-and-recovery-ux.md))
