# 02 — Bootstrap Service + Podman Sidecar

**Status:** Draft
**Parent:** [`00-architectural-reframe`](00-architectural-reframe.md)
**Sibling:** [`01-state-machine`](01-state-machine.md)

## Subsystem responsibilities

The bootstrap service is the new background subsystem that runs on every app launch. Its job is to bring the system from "Tauri app running, no other guarantees" up to `(ShellReady, *)`. It is **idempotent** — every launch runs it; subsequent launches finish fast because most steps are already done.

This subsystem replaces the imperative reimplementation in [`app/src/components/wizard/install-step/pipeline-steps.ts`](../../../app/src/components/wizard/install-step/pipeline-steps.ts), which today runs ad-hoc as part of the wizard. The new service is backend-driven (Rust), runs on app launch (not on user click), and reports progress via the existing event channel.

### Pipeline

```
1. detect-runtime    → podman --version OR docker --version
2. install-runtime   → only if step 1 fails (Podman bootstrapper sidecar)
3. write-env         → cp .env.example .env if missing
4. build-images      → podman compose build (vault-agent, vault-forge, vault-pioneer)
5. pull-images       → podman compose pull (mitmproxy from registry; build-only services skipped)
6. up-shell          → podman compose up -d vault-proxy vault-forge vault-pioneer
7. verify-shell      → CA cert present + 3 containers running + proxy listening probe
```

After step 7, the watchdog reports `(ShellReady, Absent)` — or `(ShellReady, Running)` for migrated installs (see [`06-migration.md`](06-migration.md)).

> **Why steps 4 and 5 are separate:** podman-compose 1.0.6 silently skips build-only services on `compose pull` (override is `--force-local`). Docker Compose tries-then-warns. Three of our four services (vault-agent, vault-forge, vault-pioneer) are `build:` stanzas; only mitmproxy is `image:`-pinned. A single `compose pull` is insufficient on Podman. The original draft of this spec was wrong on this point.

### Idempotency rules

Every step checks before acting:

| Step | Skip if | Cost when skipped |
|---|---|---|
| 1 detect-runtime | always runs (cheap) | ~50ms (single binary exec) |
| 2 install-runtime | step 1 succeeded | not invoked |
| 3 write-env | `.env` exists | ~5ms (file stat) |
| 4 build-images | `podman images` shows all three local images with up-to-date Containerfile mtimes | ~200ms (`podman images` query) |
| 5 pull-images | `podman image inspect` returns the pinned mitmproxy SHA | ~500ms (registry HEAD on miss) |
| 6 up-shell | three containers already running | ~200ms (`podman ps`) |
| 7 verify-shell | CA cert present AND last verify < 5min ago | ~10ms (file stat) |

Subsequent launches finish the pipeline in well under 1s when nothing has changed. The first launch is dominated by step 4 (image build, ~3-5min) and step 5 (image pull, ~250 MB mitmproxy, ~30-60s). Step 2 if Podman is absent adds ~2-3min OS install plus an admin prompt.

> **Realistic first-launch budget: 5-8 minutes** with Podman pre-installed; 8-11 minutes if Podman needs installing. The earlier "3-5 minutes" claim in the draft was wrong because it didn't account for `compose build`.

### Recovery from partial state

The pipeline is restartable from any step:
- Crashed during step 4? Next launch sees missing/stale images, runs build again (idempotent).
- Crashed during step 5? Next launch resumes the pull (`podman compose pull` is itself resumable).
- Crashed during step 6? Next launch sees containers partially up, runs step 6 again (`compose up -d` is idempotent).
- Crashed during step 2? Next launch detects no Podman, re-invokes the sidecar.

No "bootstrap-resume" state is persisted; the live system state is always the source of truth.

## Post-bootstrap: auto-activation

After step 7 (verify-shell) succeeds, the bootstrap subsystem dispatches to one of three terminal states based on marker files. **This is the single canonical location for the post-bootstrap "should I bring up the agent automatically?" decision.** Both [`03-activation-flow.md`](03-activation-flow.md) §"Subsequent activations" and [`06-migration.md`](06-migration.md) describe this same logic from different angles.

Logic in `app/src-tauri/src/bootstrap/auto_activate.rs` (new file):

```rust
pub async fn after_shell_ready(app: AppHandle) {
    let env = read_env(env_path());
    let activated = marker_present(MARKER_ACTIVATED);
    let credentials_ok_at = read_marker_ts(MARKER_CREDENTIALS_OK);

    // Branch 1: fresh install or user explicitly reset
    if !activated || !has_real_keys(&env) {
        return; // state computes to (ShellReady, Absent); user clicks launch
    }

    // Branch 2: existing install with markers, possibly stale
    let needs_revalidation = match credentials_ok_at {
        None => true,
        Some(ts) => age_days(ts) > 7,
    };

    if needs_revalidation {
        match validate_anthropic_key(&env.anthropic_key).await {
            Ok(ValidationOutcome::Ok) => {
                write_marker_with_ts(MARKER_CREDENTIALS_OK);
                // fall through to commit
            }
            Ok(ValidationOutcome::AuthFailure) => {
                clear_marker(MARKER_CREDENTIALS_OK);
                // state stays (ShellReady, Absent); banner from 06-migration surfaces
                return;
            }
            _ => {
                // Transient — defer; don't bring agent up, don't clear markers
                return;
            }
        }
    }

    // Branch 3: markers valid (or freshly re-validated) — auto-commit
    run_compose(&root, &["up", "-d", "--force-recreate", "vault-proxy"], 30s);
    run_compose(&root, &["up", "-d", "vault-agent"], 60s);
    // Watchdog observes vault-agent running, emits (ShellReady, Running)
}
```

Three callers, all on the same primitive:
- **First-time activation** ([`03-activation-flow.md`](03-activation-flow.md)): user completes wizard → write markers → call `after_shell_ready` (which now takes the markers-valid branch and brings up the agent)
- **Subsequent normal launches**: app start → bootstrap pipeline → call `after_shell_ready` (markers valid, auto-commit)
- **Migration from v0.3** ([`06-migration.md`](06-migration.md)): app start (first v0.4 launch) → bootstrap pipeline → migration check writes initial markers → call `after_shell_ready`

The `validate_anthropic_key` Tauri command is the same one defined in [`03-activation-flow.md`](03-activation-flow.md) §"Live-ping Tauri command."

## Progress reporting

Emits via the existing [`perimeter-state-changed`](../../../app/src-tauri/src/lifecycle.rs) event channel, plus three new sub-events:

| Event | Payload | When |
|---|---|---|
| `bootstrap-step-started` | `{ step: "build-images" \| "pull-images" \| ..., total_steps: 7, current: 4 }` | Step begins |
| `bootstrap-step-progress` | `{ step, percent: f32, detail: Option<String> }` | During measurable steps (build, pull) |
| `bootstrap-step-failed` | `{ step, cause: CauseId, message, retry_count, last_error: Option<String> }` | Step fails; cause matches the taxonomy in [`04-stop-and-recovery-ux.md`](04-stop-and-recovery-ux.md) |

`BootstrapProgress` shape held in `PerimeterStateStore`:

```rust
pub struct BootstrapProgress {
    pub step: BootstrapStep,
    pub percent: Option<f32>,
    pub detail: Option<String>,
    pub started_at_unix_ms: u64,
}

pub enum BootstrapStep {
    DetectRuntime,
    InstallRuntime,
    WriteEnv,
    BuildImages,
    PullImages,
    UpShell,
    VerifyShell,
}
```

The frontend's existing `useAlerts()` hook can subscribe and render an unobtrusive progress chip inside `HeroStatusCard`. Build and pull are the only steps that benefit from a percentage; others are step-by-step.

## Image build + pull (explicit)

**Step 4 (build):** `podman compose build --pull` — the `--pull` flag pulls the *base image* layer (alpine, debian, etc.) before building. Parses stderr for `STEP X/Y` lines per service to compute progress.

**Step 5 (pull):** `podman compose pull --quiet=false` — pulls explicitly-pinned images. With our compose.yml that's only mitmproxy. Parses stderr for `Trying to pull <ref>…` / `Copying blob <sha> [...]` for byte-level progress.

Without a separate build step, `compose up -d` would build inline and Karen sees "starting…" for 5 minutes with no signal of what's happening. The two-step explicit form is for progress visibility, not strictly for correctness.

## The Podman bootstrapper sidecar

A Tauri sidecar binary, one per OS triple. Tauri 2 sidecar config:

**`app/src-tauri/tauri.conf.json`:** add to `bundle`:
```json
"externalBin": ["binaries/podman-bootstrap"]
```

**`app/src-tauri/capabilities/default.json`:** add a sidecar permission:
```json
{
  "identifier": "shell:allow-execute",
  "allow": [{ "name": "binaries/podman-bootstrap", "sidecar": true }]
}
```

**`app/src-tauri/Cargo.toml`:** no change — `tauri-plugin-shell = "2"` (line 13) is already present and initialized in [`lib.rs:189`](../../../app/src-tauri/src/lib.rs).

**Naming:** `binaries/podman-bootstrap-{rust-host-triple}` per Tauri's auto-resolve convention. e.g.:
- `binaries/podman-bootstrap-x86_64-pc-windows-msvc.exe`
- `binaries/podman-bootstrap-aarch64-apple-darwin`
- `binaries/podman-bootstrap-x86_64-unknown-linux-gnu`

### Per-OS install command (verified)

Latest Podman stable as of 2026-04: 5.8.2.

| OS | Installer | Silent command | Admin? | Gotcha |
|---|---|---|---|---|
| Windows (x64) | `podman-installer-windows-amd64.msi` | `msiexec /i podman-installer-windows-amd64.msi /quiet /qn /norestart` | Yes (UAC) | Standard MSI flags; **NOT documented by Podman**, must be empirically validated on a Windows VM |
| macOS (universal) | `podman-installer-macos-universal.pkg` | `sudo installer -pkg podman-installer-macos-universal.pkg -target /` | Yes (sudo) | Extrapolated from `installer(8)`; not Podman-documented |
| Linux (Debian/Ubuntu) | apt | `sudo apt-get update && sudo apt-get -y install podman` | Yes (sudo) | Documented in podman.io |
| Linux (Fedora/RHEL) | dnf | `sudo dnf -y install podman` | Yes (sudo) | Documented |

**No upstream-supported Linux rootless static binary** exists. The earlier draft's "rootless tarball" path was incorrect. All Linux paths require sudo.

**macOS and Windows additionally require `podman machine init && podman machine start` after install.** Bootstrap step 2 has two sub-phases on those OSes; treat them as "install Podman" and "initialize Podman" with separate progress reporting.

### Admin-prompt UX

One framed prompt before the OS dialog appears:

```
Setting up your safe room

We need your permission to install Podman, the runtime that
keeps your assistant inside a sealed container. Your computer
will ask for permission in a moment — this only happens once.

[Continue]   [Install manually instead]
```

If the user cancels the OS dialog after clicking Continue:
- Sidecar exits non-zero
- State transitions to `ShellFailed × Absent` with cause `podman-install-denied`
- Recovery card surfaces with **Try again** (re-prompts) and **Install manually** link

No silent retries on cancel.

### Detection logic

```
1. `podman --version` succeeds → use podman; check `podman ps` works (catches Podman Machine not started)
2. `podman --version` fails, `docker --version` succeeds → use docker
3. Both fail → invoke install path
4. After install, re-run step 1
```

`runtime-misdetected` failure cause covers `--version` succeeding while `ps` errors (Podman Machine stopped, WSL2 issue, user not in `podman` group).

### Sidecar Rust API

Tauri 2's modern API (the legacy `tauri::api::process` is removed):

```rust
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

let cmd = app.shell().sidecar("podman-bootstrap")?
    .args(["install", "--report-progress"]);
let (mut rx, mut child) = cmd.spawn()?;

while let Some(event) = rx.recv().await {
    match event {
        CommandEvent::Stdout(line) => emit_progress_from_line(&handle, &line),
        CommandEvent::Stderr(line) => log_redacted(&line),
        CommandEvent::Terminated(payload) => handle_exit(payload.code),
        _ => {}
    }
}
```

The sidecar emits one JSON line per progress update on stdout; logs go to stderr. `redact_secrets` from [`lifecycle.rs:141-163`](../../../app/src-tauri/src/lifecycle.rs) is applied to stderr before logging.

## Single-instance enforcement

`tauri-plugin-single-instance` is **not currently configured**. Two concurrent app launches would race during bootstrap (parallel `compose pull` on the same images, `compose up` on the same services).

Add to `Cargo.toml`:
```toml
tauri-plugin-single-instance = "2"
```

In `lib.rs:188`, register **first** (per docs: "must be the first one to be registered to work well"):
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
    }))
    .plugin(tauri_plugin_shell::init())
    // ... rest unchanged
```

The callback fires in the already-running instance when a second launch is attempted; the second process exits immediately. Reuses `show_main_window` logic already in [`lib.rs:158-164`](../../../app/src-tauri/src/lib.rs).

## Edge cases

### Laptop sleeps mid-bootstrap
OS suspends; child commands suspend. On wake, they resume. If the network changed during sleep, the next network-touching step retries via standard backoff. No special wake handling.

### Network disappears during image pull
`compose pull` exits non-zero. Step 5 retries with exponential backoff (5s, 30s, 120s). After three failures, transitions to `ShellFailed × Absent` with cause `network-unavailable`. Recovery card auto-re-probes every 60s; re-attempts when network returns.

### User has existing Podman but it's broken
Detection: `podman --version` succeeds, `podman ps` errors. Catches stopped Podman Machine on macOS, WSL2 issues on Windows, missing `podman` group on Linux. Failure cause: `runtime-misdetected`. Recovery card links to per-OS troubleshooting; we don't auto-fix.

### First-launch image pull is large
Mitmproxy ~250 MB compressed; locally-built images add ~500 MB after build. Total ~750 MB - 1 GB on first install. Build progress + pull progress are separate UI elements. Cancel during pull → `image-pull-cancelled` cause; user retries.

### User denies admin prompt and we can't go rootless
On Windows and macOS, system-wide Podman install requires admin. If denied and OS isn't Linux:

> Setting up the safe room needs your permission. Without it, we can't continue automatically. You can either grant permission and try again, or install Podman yourself by following these instructions.

No silent fallback path.

## Plaintext .env disclosure

Anthropic and Telegram keys persist in plaintext at `<repo-root>/components/openclaw-vault/.env` per the existing pattern. Defense-in-depth measures already in place:
- `.env` is in `.gitignore`
- `redact_secrets` at [`lifecycle.rs:141-163`](../../../app/src-tauri/src/lifecycle.rs) scrubs known token-bearing env vars from logs
- Frontend redaction at [`install-step/utils.ts:60-64`](../../../app/src/components/wizard/install-step/utils.ts) handles streaming logs

The activation flow's wizard surface (see [`03-activation-flow.md`](03-activation-flow.md)) discloses storage location to the user.

OS keychain migration is a v1.0 conversation. Reasoning: the proxy reads keys from process env at container creation; moving to keychain requires redesigning the env-var injection path (currently `${ANTHROPIC_API_KEY}` interpolation in compose.yml). Stronghold is a file-vault with password-prompt UX, not the right fit. The right keychain path is the `keyring` crate (covers macOS Keychain Services / Windows Credential Manager / Linux libsecret), but that's substantial work that doesn't gate v0.4.

## v1.0 follow-on (bundled Podman)

Sidecar interface designed so the swap to bundled Podman is internal:

- The sidecar's contract: "make `podman ps` work after I exit zero." Bundled Podman would honor the same contract by extracting and registering bundled binaries instead of running an external installer.
- Bundled + sidecar coexist: bundled is preferred when present (offline-capable), sidecar falls back when bundled is absent.
- Karen sees no UX difference — the "Setting up your safe room…" surface is identical.

The v1.0 work is then: (a) cross-platform Podman binary packaging and (b) the bundled-vs-sidecar selector. The state machine, progress reporting, and recovery surfaces are untouched.

## Test coverage

Unit tests:
- Each pipeline step in isolation, with mocked filesystem and process invocations
- Idempotency: running the pipeline twice in a row results in only step 1 + cheap checks the second time
- Failure-cause classification: synthetic failures map to the correct cause IDs

Integration tests (require Podman in the test environment):
- Fresh-state bootstrap: empty `~/.lobster-trapp/`, no `.env`, with Podman pre-installed → reaches `ShellReady × Absent`
- Missing-Podman simulation: env-var-toggled `PATH` that hides Podman → sidecar invoked, fails as expected (CI env can't grant admin), surfaces `podman-install-denied` cause
- Restart resilience: kill the bootstrap mid-step-4 → next launch resumes correctly

Manual smoke test on a clean VM (per OS, at least once per release cut):
- macOS Sequoia, Apple Silicon: full flow including admin prompt + `podman machine init`
- Windows 11, x64: full flow including UAC prompt + `podman machine init`
- Ubuntu 24.04: full flow with sudo prompt

> **Important precondition for integration tests:** [`07-container-name-cleanup.md`](07-container-name-cleanup.md) must land first. The hardcoded `container_name:` lines in compose.yml prevent `--project-name` isolation; without that fix, integration tests would collide with any locally-running perimeter.

## Out of scope

- **Bundled Podman** — v1.0 follow-on; sidecar interface designed for forward compatibility
- **Docker bootstrap** — if the user has Docker, we use it; we don't install Docker if absent (Podman is project default)
- **Auto-updating Podman** — surface a recovery card if version too old; never silently upgrade user-installed runtimes
- **OS keychain credential storage** — v1.0 conversation
- **Air-gapped environments** — bootstrap requires internet for image pull on first run
- **Telemetry / analytics** — none in v0.4; no anonymous error reporting wired up
