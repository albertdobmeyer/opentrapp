# 04 — Stop & Recovery UX

**Status:** Draft
**Parent:** [`00-architectural-reframe`](00-architectural-reframe.md)
**Sibling:** [`01-state-machine`](01-state-machine.md)

## One Stop button (not two)

Today's UX has a "Pause" button in [`HeroStatusCard.tsx:151-159`](../../../app/src/components/HeroStatusCard.tsx) when state is `running_safely`. The reframe replaces this with a single **Stop your assistant** button — same primitive underneath, more honest framing.

### Why one button, not two

The earlier draft considered two affordances ("Pause" temporary + "Stop" intentional) sharing the same `pause_perimeter` primitive. Two buttons that do exactly the same thing is UI noise; it forces the user to interpret a non-existent distinction. One button with clear copy ("Stop your assistant" / "Resume") is the simpler, more honest design.

### The primitive

Both Stop and Resume call existing Tauri commands, unchanged:

| Action | Command | What it does |
|---|---|---|
| Stop | `pause_perimeter` ([`commands/lifecycle.rs:87-119`](../../../app/src-tauri/src/commands/lifecycle.rs)) | `compose stop` against root compose.yml: stops all 4 containers, **preserves all volumes** (`vault-data` for session history, `forge-deliveries` for installed skills, `vault-proxy-logs`, `proxy-ca`), writes `~/.lobster-trapp/paused` marker |
| Resume | `resume_perimeter` ([`commands/lifecycle.rs:140-144`](../../../app/src-tauri/src/commands/lifecycle.rs)) | Clears paused marker, `compose up -d` brings all four back |

**Critical: never `nuclear-kill` or `hard-kill`.** Both wipe `vault-data` and the agent image:
- `hard-kill` runs `compose down --volumes --remove-orphans` + `rmi openclaw-vault` ([`components/openclaw-vault/scripts/kill.sh:30-49`](../../../components/openclaw-vault/scripts/kill.sh))
- `nuclear-kill` on Linux is equivalent to hard-kill ([`kill.sh:71-72`](../../../components/openclaw-vault/scripts/kill.sh) explicit comment); on Windows additionally tears down WSL VM

Karen losing her conversation history and installed skills on a "Stop" click would be a trust-breaking event. `pause_perimeter` is the only primitive that meets the data-preservation bar.

`hard-kill` and `nuclear-kill` remain in the openclaw-vault manifest for developer/operator use via the dev-mode advanced UI; they are NOT exposed to user-mode.

## Stop button UX

In `HeroStatusCard.tsx`, replace the existing Pause block (lines 141-159) for the `(ShellReady, Running)` state:

```jsx
<div className="flex justify-center gap-3 mt-6">
  <button
    type="button"
    onClick={openTelegram}
    className="btn btn-lg btn-primary"
  >
    <MessageCircle size={18} />
    Open Telegram
  </button>
  <button
    type="button"
    onClick={handleStop}
    className="btn btn-lg btn-ghost"
    disabled={stopLoading}
  >
    <StopCircle size={18} />
    {stopLoading ? "Stopping…" : "Stop your assistant"}
  </button>
</div>
```

Confirmation: a small inline confirm (not a separate dialog) appears below the button on first click:

> **Stop your assistant?** It'll stop responding on Telegram until you tap Resume. Your conversation history and installed skills stay safe.
>
> [Stop now] [Cancel]

This is one extra tap but Karen sees explicitly that data is preserved — that's the trust message worth ~200ms of friction. Subsequent stops within the same session can skip the confirm (settings: "ask before stopping").

### Toast feedback

- Stop initiated: `"Stopping your assistant…"` (in-progress toast, dismissable)
- Stop succeeded: `"Stopped. Your data is safe. Tap Resume any time."` (success toast, 4s)
- Stop failed: `"Couldn't stop your assistant. [Try again]"` (error toast, persistent, retry CTA)

### Resume button

In the `(ShellReady, Paused)` state:

```jsx
<button
  type="button"
  onClick={handleResume}
  className="btn btn-lg btn-primary"
  disabled={resumeLoading}
>
  <Play size={18} />
  {resumeLoading ? "Resuming…" : "Resume"}
</button>
```

Toast: `"Resuming your assistant…"` → `"Back online. Open Telegram to keep chatting."`

## Recovery card for `ShellFailed × Absent`

A peer of the launch button, not a future TODO. Lives in `HeroStatusCard.tsx`, reusing existing card patterns. Shown only when state is `(ShellFailed, Absent)`.

### Layout

The card replaces the regular `HeroStatusCard` body in this state. Title, sub-line, primary affordance, optional secondary, optional details disclosure.

### Failure-cause taxonomy

| Cause ID | Trigger | Title | Body | Primary | Secondary |
|---|---|---|---|---|---|
| `podman-install-failed` | Sidecar exit code != 0, not denial | "We hit a snag setting up the runtime" | "We couldn't install Podman, the runtime that keeps your assistant in a sealed container." | **Try again** | "Install manually" link |
| `podman-install-denied` | User cancelled UAC/sudo | "We need your permission" | "We need your permission to set up the safe room. We'll only ask once." | **Try again** (re-prompts UAC/sudo) | "Install manually" link |
| `image-build-failed` | `compose build` exit != 0 | "Couldn't build the safe-room components" | "Something stopped us from preparing the safe room on your computer." | **Try again** | "Show details" disclosure |
| `image-pull-failed` | `compose pull` transient error | "Couldn't download a safe-room component" | "Network hiccup while downloading. We'll keep trying." | **Try again now** (auto-retries also continue) | — |
| `image-pull-cancelled` | User cancelled in-progress pull | "Download was cancelled" | "Want to finish setting up your safe room?" | **Continue download** | — |
| `image-pull-denied` | 401/403 from registry | "Can't access the safe-room download" | "Something's wrong with the download — likely a registry issue." | "Get help" link | "Show details" |
| `runtime-misdetected` | `podman --version` ok but `podman ps` errors | "Podman's installed but not working right" | "Your computer has Podman installed but it's not running right now." | "Open guide" (per-OS troubleshooting) | "Try again" |
| `network-unavailable` | DNS/connectivity probe fails 3min sustained | "Can't reach the internet" | "We need an internet connection to set up your safe room." | (auto-re-probes; passive UI) | — |
| `shell-up-failed` | `compose up -d` for shell containers fails | "We couldn't start the safe room" | "The safe room couldn't start up. We can try again." | **Try again** | "Show details" |

Each cause emits a structured payload `{ step: BootstrapStep, cause: CauseId, message: String, retry_count: u32, last_error: Option<String> }` on the `bootstrap-step-failed` event channel (the `step` field identifies which pipeline stage in [`02-bootstrap-service.md`](02-bootstrap-service.md) emitted the failure). Cross-spec: this taxonomy is the single source of truth for failure causes.

### Retry semantics

- **Auto-retries** (image-pull-failed, network-unavailable): handled inside the bootstrap subsystem with exponential backoff; UI shows "Trying again…" without surfacing the recovery card during the retry window
- **Manual retries** (most causes): triggered from the recovery-card primary button; clears the failure state and re-runs the failed step from scratch
- **Max-retry exhaustion**: card stays visible until the user takes action; bootstrap doesn't re-attempt without user input (avoids retry storms)

### "Show details" disclosure

A small expandable section below the primary buttons. Reveals:

```
Step that failed: build-images
What we tried: podman compose build vault-forge
What it said: 
  STEP 5/12: RUN apk add --no-cache curl python3
  /bin/sh: apk: not found
  ERRO[0001] failed to build: exit status 127

[Copy to clipboard]
```

Output is run through [`redact_secrets()` at `lifecycle.rs:141-163`](../../../app/src-tauri/src/lifecycle.rs) to scrub any TELEGRAM_BOT_TOKEN / ANTHROPIC_API_KEY values. For Karen this disclosure should typically be empty; for support-channel debugging it gives an attachable snippet that's already key-redacted.

### Persistent log

`~/.lobster-trapp/bootstrap.log` accumulates all bootstrap events (started, progress samples, failed). Rotated at 10 MB (existing log-rotate primitive in vault scripts). Karen can attach this log when reaching out for support; the "Show details" disclosure shows the most recent failure block.

## Tray icon mapping

Three icon variants in [`app/src-tauri/icons/`](../../../app/src-tauri/icons/), called via `tray.set_icon()` from [`update_tray_for_state()`](../../../app/src-tauri/src/lifecycle.rs):

| Icon | States | Tooltip |
|---|---|---|
| **Amber** | `Installing × Absent`, `Bootstrapping × Absent`, `ShellReady × Absent`, `ShellReady × Activating`, `ShellReady × Paused` | "Assistant — getting ready" / "ready to launch" / "starting…" / "stopped (intentional)" |
| **Green** | `ShellReady × Running` | "Assistant — running safely" |
| **Red** | `ShellFailed × Absent`, `ShellReady × Errored` | "Assistant — needs your attention" |

> Paused is amber, not red. Pause is intentional and reversible; red implies something's wrong. Amber says "intentionally off, you can resume any time."

macOS template-image rules: use template PNGs that auto-tint to system menubar contrast. If template tinting fights with the color-by-state model, fall back to state-shape variants (different silhouette per state).

## Test coverage

Unit tests in `app/src/components/HeroStatusCard.test.tsx`:
- `(ShellReady, Running)` renders Stop button with correct copy and disabled state during in-flight
- `(ShellReady, Paused)` renders Resume button
- `(ShellFailed, Absent)` renders recovery card with cause-appropriate copy
- All 9 failure causes render the expected title/body/primary/secondary
- "Show details" disclosure round-trips redacted error text correctly

E2E in `app/e2e/stop-and-recovery.spec.ts` (new):
- Stop click → confirm → toast → state transitions to `(ShellReady, Paused)` → containers stopped → volumes verified intact (`podman volume inspect` shows `vault-data` size > 0)
- Resume click → containers come back → state `(ShellReady, Running)`
- Recovery card retry → state recovers when underlying cause resolved
- Tray icon transitions match the documented mapping

Manual smoke test:
- Stop, restart the app, verify state is `(ShellReady, Paused)` (paused marker honored across restart)
- Stop, manually `rm ~/.lobster-trapp/paused`, restart app — verify state recovers to `(ShellReady, Running)` (marker is the source of truth)

## Out of scope

- **Per-tenant stop** (when v1.0+ has multi-tenant) — v0.4 has one tenant
- **Soft-stop primitive that keeps proxy running** — `pause_perimeter` already preserves data; making proxy stay up while agent is down adds complexity without clear benefit. The whole-perimeter pause is simpler.
- **"Pause for X minutes" / scheduled resume** — out of scope for v0.4
- **Recovery card auto-actions beyond retry** (e.g., automatic Podman repair) — diagnosing the user's runtime is outside our scope; we link to per-OS guides instead
