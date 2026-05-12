# Release notes — v0.4.0 (Shellfish Reframe)

## What changed

### Shell/tenant reframe — two-axis state machine

The app now models two independent concerns separately: whether the sandbox environment is ready (bootstrap axis) and whether the user has activated their assistant (tenant axis). Previously these were conflated, which caused confusing UI states on first run.

New states added to the hero card:

| State | Meaning |
|---|---|
| `installing` | First-run package download in progress |
| `bootstrapping` | Sandbox containers being built for the first time |
| `shell_ready_absent` | Sandbox is up but user hasn't activated yet |
| `shell_failed` | Sandbox setup failed; recovery action shown |

### Activation modal

New onboarding flow for first-time activation. Walks the user through entering their Anthropic API key and pairing a Telegram bot — replaces the previous credential-entry scattered across setup wizard steps. Opens automatically when `shell_ready_absent` is detected; also reachable from the hero card at any time.

### Auto-activate migration

Existing v0.3.x users who already have credentials stored are migrated automatically on first launch. If the migrated Anthropic key is rejected, the app emits a `migration-needs-recredential` event and re-opens the activation modal in re-credential mode so only the API key needs to be re-entered.

### Stop and recovery UX

The hero card now shows a stop/restart flow with a confirmation step before stopping the sandbox. During recovery (sandbox restarting itself), the hero shows a distinct "Recovering" state with appropriate copy rather than the generic error state.

### Tray icon improvements

Tray icon state tracks running/stopped/recovering and updates in real time. Stop action available directly from the tray menu.

---

## Breaking changes

None. Existing credentials and settings carry forward automatically via the migration path.

## Upgrade path

Standard auto-update — the Tauri updater will prompt in-app. If updating manually, download the installer for your platform from the assets below and run it over the existing installation.
