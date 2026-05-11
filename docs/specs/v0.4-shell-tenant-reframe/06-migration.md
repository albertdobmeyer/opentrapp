# 06 — Migration: Existing Installs

**Status:** Draft
**Parent:** [`00-architectural-reframe`](00-architectural-reframe.md)
**Siblings:** [`01-state-machine`](01-state-machine.md), [`03-activation-flow`](03-activation-flow.md)

## The problem

Users who already completed v0.3's wizard have:
- `.env` at `<repo-root>/components/openclaw-vault/.env` with non-placeholder `ANTHROPIC_API_KEY` and `TELEGRAM_BOT_TOKEN`
- A previously-running 4-container perimeter (or had one until they last quit the app)
- `settings.wizardCompleted: true` in the persisted UI settings

Without explicit migration logic, the v0.4 build would either re-run the wizard (annoying) or skip validation entirely and assume the keys still work (incorrect — keys can be revoked between sessions). Neither is acceptable.

## Detection

On first launch of v0.4, the bootstrap subsystem (after step 7 verify-shell, before computing tenant state) runs migration logic:

```rust
async fn check_migration(env_path: &Path) -> MigrationDisposition {
    if !env_path.exists() { return MigrationDisposition::FreshInstall; }

    let env = parse_env(env_path);
    let anthropic = env.get("ANTHROPIC_API_KEY")
        .filter(|v| !is_placeholder(v));
    let telegram = env.get("TELEGRAM_BOT_TOKEN")
        .filter(|v| !is_placeholder(v));

    match (anthropic, telegram) {
        (Some(a), Some(_)) => MigrationDisposition::ExistingInstall { anthropic_key: a },
        _ => MigrationDisposition::FreshInstall,
    }
}
```

`is_placeholder` reuses the existing detection at [`status_aggregator.rs:587`](../../../app/src-tauri/src/status_aggregator.rs) (catches `REPLACE_ME`, `sk-ant-api03-REPLACE-...`, `placeholder`, etc.).

`MigrationDisposition::ExistingInstall` triggers the verification flow below. `FreshInstall` triggers the standard activation flow when the user clicks the launch button.

## Verification flow (one live ping)

The migration writes `~/.lobster-trapp/credentials-ok` only after a verified live `/v1/messages` ping using the existing key. **Never trust past success without checking.**

```rust
async fn migrate_existing_install(anthropic_key: String, app: AppHandle) {
    match validate_anthropic_key(anthropic_key).await {
        Ok(ValidationOutcome::Ok) => {
            write_marker(MARKER_ACTIVATED);
            write_marker_with_ts(MARKER_CREDENTIALS_OK);
            // Bring up vault-agent; state will compute to (ShellReady, Running)
            run_compose(&root, &["up", "-d", "vault-agent"], 60s);
            emit(app, "migration-completed", ());
        }
        Ok(ValidationOutcome::AuthFailure) => {
            // Key invalid; do NOT write markers; do NOT bring up agent
            // Next observation will compute (ShellReady, Absent)
            // Activation modal opens automatically (per the standard flow)
            emit(app, "migration-needs-recredential", ());
        }
        Ok(_other) => {
            // Network error, rate limit, etc. — try again on next launch
            // Don't block migration on transient issues
            emit(app, "migration-deferred", ());
        }
        Err(e) => {
            // Transport error; treat like above
            log_redacted(&e);
            emit(app, "migration-deferred", ());
        }
    }
}
```

`validate_anthropic_key` is the same Tauri command from [`03-activation-flow.md`](03-activation-flow.md); reused here for migration.

## UX surfaces

### Successful migration (silent)
Karen launches v0.4 for the first time. She sees:
1. Tray amber briefly (bootstrap idempotent run, ~1-2s on warm install)
2. Tray green
3. Window opens to home → "Your assistant is running safely" with "Open Telegram" + "Stop your assistant"

No modal, no "welcome to v0.4," no re-credential prompt. Migration is invisible. Bot's first-message tutorial does NOT fire because session history exists from v0.3 — Karen had prior conversations.

> **Special case for tutorial:** if Karen's prior install never produced session history (e.g., she activated v0.3 but never sent a message), the tutorial WILL fire on her first v0.4 message. That's correct behaviour — she hasn't met the bot yet from a conversational standpoint.

### Re-credential needed
Live ping returns 401:
1. Tray stays amber throughout (the migration check is a transient verification, not a `ShellFailed` state — see [`01-state-machine.md`](01-state-machine.md) tray mapping; `(ShellReady, Absent)` is amber because it's a healthy waiting state)
2. Window opens to home → "Ready to launch your assistant" with the launch button
3. A small banner via `ProactiveAlertsBanner`:

> **Your Anthropic key needs updating.** It used to work, but it doesn't anymore — likely the key was rotated or revoked. Tap Launch to update it.

Clicking Launch opens the activation modal pre-filled with the existing Telegram token (which we trust hasn't changed — Telegram tokens don't get auto-revoked) and an empty Anthropic field. Step 1 only.

### Deferred (transient error)
Network unreachable or 5xx during migration:
1. Tray stays amber on `Bootstrapping × Absent` (because we couldn't complete migration)
2. After 30 seconds with no resolution, soft-transition to `(ShellReady, Absent)` with no markers written
3. On next launch, retry the migration

Karen doesn't see an error explicitly; from her perspective the app is just "still getting ready." If transient errors persist for >5 minutes, surface as `(ShellFailed, Absent)` with cause `network-unavailable` and the standard recovery card.

## Marker files written

On successful migration:
- `~/.lobster-trapp/activated` — file presence; no content
- `~/.lobster-trapp/credentials-ok` — file with unix-millis timestamp of the validation

On failed re-credential or deferred: nothing is written. State stays `(ShellReady, Absent)` until a successful activation.

## Legacy `wizardCompleted` setting

The existing `settings.wizardCompleted` boolean (in the Tauri-managed settings store, accessed via `useSettings()`) becomes vestigial. We don't remove it for backward compat (downgrading from v0.4 → v0.3 should still work for users who try it), but the new logic uses marker files as the source of truth.

After migration:
- `wizardCompleted` is left as-is (probably `true` for migrated users)
- The frontend reads `state.bootstrap` and `state.tenant` from the perimeter event channel and ignores `wizardCompleted` for decision-making

## Edge cases

### User had v0.3 wizard incomplete
`.env` has placeholder values (or only one of two keys). `is_placeholder` returns true. Disposition is `FreshInstall`. State computes to `(ShellReady, Absent)`; standard launch button flow.

### User had v0.3 perimeter running when they upgraded
The old build's `bring_perimeter_down_sync` ran on Quit, so all four containers were stopped before the upgrade. After the upgrade, bootstrap step 6 brings up the shell only (proxy/forge/pioneer); migration brings up the agent. End state: same `(ShellReady, Running)` as before, just via the new path.

If the old build crashed without a clean shutdown, RunGuard ([`lifecycle.rs:266-293`](../../../app/src-tauri/src/lifecycle.rs)) reaps orphan containers on the next launch — unchanged behaviour.

### User had a custom .env with extra keys (OPENAI_API_KEY etc.)
We only check for `ANTHROPIC_API_KEY` and `TELEGRAM_BOT_TOKEN` (the required pair). Other env entries pass through unchanged in `.env`.

### User had v0.3's `~/.lobster-trapp/paused` marker
Migration honors it — we don't bring up the agent if the user explicitly paused. State computes to `(ShellReady, Paused)`. Karen sees "Stopped" with the Resume button.

### container_name cleanup interaction
If migration runs *before* [`07-container-name-cleanup`](07-container-name-cleanup.md) lands, the existing perimeter has hardcoded container names. After 07 lands, container names change to `lobster-trapp_vault-X_1` style. Existing volumes are scoped to compose project name (already `lobster-trapp` by default), so they migrate cleanly. Run a one-time `podman container rm` for the old hardcoded-name containers if they still exist in stopped state — handled by `compose down` before the new bring-up.

## Test coverage

Unit tests:
- `check_migration` returns correct disposition for: missing .env, placeholder-only .env, valid .env, partial .env (only one key)
- `is_placeholder` matches all known placeholder patterns and rejects real keys

Integration tests:
- Fresh `~/.lobster-trapp/`, valid `.env` → migration writes both markers, brings up agent, state is `(ShellReady, Running)`
- Fresh `~/.lobster-trapp/`, `.env` with revoked key (mocked 401) → no markers, agent doesn't start, state is `(ShellReady, Absent)` with re-credential banner
- Fresh `~/.lobster-trapp/`, `.env` with valid key, network unavailable → migration deferred, retried on next launch

Manual dogfood (per [`tests/dogfood/CHECKLIST.md`](../../../tests/dogfood/CHECKLIST.md)):
- Take a v0.3 install, upgrade to v0.4, verify silent successful migration
- Take a v0.3 install, revoke the API key in console.anthropic.com, upgrade — verify re-credential prompt
- Take a v0.3 install with `~/.lobster-trapp/paused` marker, upgrade — verify state is `(ShellReady, Paused)`

## Out of scope

- **Migrating from versions older than v0.3** — v0.3 is the floor; pre-v0.3 users go through fresh-install flow
- **Migration UI** — there is no migration UI; success is silent, failure surfaces via the standard re-credential banner
- **Backup/restore of marker files** across reinstalls — markers live in `~/.lobster-trapp/`; reinstalling the app preserves them, uninstalling does not
- **Cross-machine migration** — installing on a new machine is a fresh install; no copy-from-old-machine flow in v0.4
