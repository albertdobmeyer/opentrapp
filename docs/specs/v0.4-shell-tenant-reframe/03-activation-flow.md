# 03 — Activation Flow

**Status:** Draft
**Parent:** [`00-architectural-reframe`](00-architectural-reframe.md)
**Siblings:** [`01-state-machine`](01-state-machine.md), [`02-bootstrap-service`](02-bootstrap-service.md)

## What this is

The just-in-time activation flow that runs when Karen clicks **Launch your assistant** from the home screen in the `(ShellReady, Absent)` state. Repositions the existing wizard step components from install-time to user-triggered. Two real steps, both genuinely external; everything else is already done by the bootstrap subsystem.

## UI: modal over home, not route navigation

Today's wizard is mounted as the `/setup` route, gated by a `<Navigate to="/setup" replace/>` redirect at [`app/src/App.tsx:114`](../../../app/src/App.tsx). The reframe converts this to a modal opened from `Home.tsx`:

- The `/setup` route stays in the router as a deep-link fallback
- The `<Navigate>` redirect is removed; first-mount logic instead opens the modal when state is `(ShellReady, Absent)` AND the post-bootstrap auto-activation logic in [`02-bootstrap-service.md`](02-bootstrap-service.md) §"Post-bootstrap: auto-activation" determined the user must intervene (markers absent, or markers stale and last live-ping failed)
- The wizard's connection-step content (specifically the API-key block at [`ConnectStep.tsx:149-169`](../../../app/src/components/wizard/ConnectStep.tsx) and the Telegram-token block at [`ConnectStep.tsx:171-191`](../../../app/src/components/wizard/ConnectStep.tsx)) is reused unchanged inside the new modal. `WelcomeStep`, `InstallStep`, and `ReadyStep` are NOT used in the activation modal — install is now bootstrap; welcome is unnecessary because the launch button is the welcome; ready becomes a final toast.

This is feasible without a routing rewrite — the connection blocks are pure presentational fragments and don't depend on `Setup.tsx` being a route. `useSettings` is hook-based.

A new `<ActivationModal>` component:
- Uses the existing modal pattern in the codebase
- Backdrop click does NOT dismiss (this is a deliberate flow; users dismiss via explicit cancel)
- Has `Esc` for cancel
- Mounts a 2-step state machine: `connect` (Anthropic key + live ping) → `verify` (Telegram token + handoff sequence)

## The two steps

### Step 1 — Anthropic API key

Reuse [`ConnectStep.tsx`](../../../app/src/components/wizard/ConnectStep.tsx) lines 149-169 (the API key input + modal). One change: after format validation passes (`isAnthropicKeyLike()` at line 164 returns true), trigger a **live `/v1/messages` ping** via a new Tauri command before letting the user proceed.

#### Live-ping Tauri command

New command in `app/src-tauri/src/commands/credentials.rs` (new file). Shape:

```rust
#[tauri::command]
pub async fn validate_anthropic_key(key: String) -> Result<ValidationOutcome, String> {
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "."}]
    });
    let res = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .timeout(Duration::from_secs(15))
        .send().await
        .map_err(|e| redact_secrets(&e.to_string()))?;

    match res.status().as_u16() {
        200 => Ok(ValidationOutcome::Ok),
        401 => Ok(ValidationOutcome::AuthFailure),
        402 => Ok(ValidationOutcome::Billing),
        403 => Ok(ValidationOutcome::Permission),
        429 => Ok(ValidationOutcome::Rate),
        500..=599 => Ok(ValidationOutcome::ServerError),
        _ => Ok(ValidationOutcome::Unknown),
    }
}
```

Why this model: `claude-haiku-4-5-20251001` is the cheapest current model with `max_tokens: 1`. `api.anthropic.com` is already on the proxy allowlist at [`components/opencli-container/proxy/allowlist.txt:4`](../../../components/opencli-container/proxy/allowlist.txt) (verified during investigation).

> **Routing concern:** during activation, the proxy may already be running with empty `ANTHROPIC_API_KEY`. The validate command bypasses the perimeter (direct `reqwest` from the host) — this is a *pre-flight* check, not a perimeter-routed request. The user's machine talks directly to api.anthropic.com once with their key. Post-activation, all real agent traffic still goes through the proxy.

#### UX during validation

- "Continue" button shows a spinner while validation is in-flight (15s timeout)
- 200 → green check next to the input; "Continue" enabled
- 401 → red `AuthFailure` banner: "That key isn't being accepted. Double-check it's the latest one from console.anthropic.com."
- 402/403 → "Looks like there's an issue with your account billing. [link to console.anthropic.com]"
- 429 → "Anthropic is rate-limiting right now. Wait a moment and try again."
- 5xx → "Anthropic's having a moment. Try again in a few seconds."
- Network error → "Can't reach Anthropic. Check your internet connection."

### Step 2 — Telegram bot

Reuse [`ConnectStep.tsx`](../../../app/src/components/wizard/ConnectStep.tsx) lines 171-191 (BotFather walkthrough modal). Format validation via `isTelegramTokenLike()` stays. The new addition: **a polling-handoff sequence that sends a test message to Karen's phone before completion**.

#### The handoff sequence

The wizard-test-and-agent-bot contention problem: Telegram allows only one consumer of `getUpdates` per bot at a time. If the wizard polls and OpenClaw is also polling, both get HTTP 409 conflicts.

The verified-clean handoff (works because OpenClaw's grammY long-polling reads from a server-side per-bot offset, not a client-side cursor):

```
Pre-condition: vault-agent is NOT YET RUNNING
              (the reframe guarantees this — agent comes up at activation
              commit, after this flow completes)

1. POST https://api.telegram.org/bot<TOKEN>/deleteWebhook
   ?drop_pending_updates=false
   (Idempotent. Clears any leftover webhook from prior wizard runs;
    preserves any in-flight messages.)

2. GET https://api.telegram.org/bot<TOKEN>/getMe
   - 200 + {"result":{"is_bot":true,"username":"..."}} → token valid; capture
     bot username for the deep-link
   - 401 → wizard surfaces "That bot token isn't recognized..."
   - 404 → "Telegram doesn't recognize that token format..."

3. UI: deep-link to t.me/<username>; copy "Open your bot in Telegram and tap Start"

4. POLL: GET https://api.telegram.org/bot<TOKEN>/getUpdates
        ?timeout=30
        &allowed_updates=["message"]
   Loop until response contains a /start message. Capture chat.id.

5. POST .../sendMessage
   chat_id=<captured>
   text="Hi! I'm your new assistant. I'm working." 

6. UI: "Did you see the test message?" with [Yes] / [No, send again] buttons

7. CONFIRM-BY-OFFSET: GET .../getUpdates
                     ?offset=<last_update_id+1>
                     &timeout=0
   This advances the SERVER-SIDE offset past /start so vault-agent doesn't
   re-process it as a new command on its first poll. Return value ignored.

8. STOP POLLING. Wizard step complete. Activation commits.
```

Server-side offset semantics: Telegram persists undelivered updates 24h. `getUpdates` with `offset=N+1` confirms updates ≤ N as delivered. After step 7, OpenClaw's grammY poller starts fresh, sees only updates *after* the wizard's confirmed offset.

#### UX timeouts and failure paths

- **Step 4 polling timeout**: 90 seconds total wait for the user's `/start`. If exceeded, modal shows "Still waiting for your /start in Telegram. [Skip and test later] [Wait another 90s]"
- **Skip and test later**: writes the token to .env without the test-message validation; sets a flag in marker so the next launch can prompt for re-validation. Does not block activation.
- **Test message lost**: user clicks "No, send again" → re-poll for the next user message, send another test message
- **409 during steps 4 or 5**: indicates someone else is polling this bot (unlikely if vault-agent is properly off, but possible if user has another instance somewhere). Surface error: "Another instance of this bot is active. Make sure you're using a fresh bot token from BotFather."

## On commit (activation)

Sequence after both steps validate:

1. Write `ANTHROPIC_API_KEY` and `TELEGRAM_BOT_TOKEN` to `.env` via existing [`upsertEnvVar`](../../../app/src/lib/wizardUtils.ts) helper
2. `podman compose up -d --force-recreate vault-proxy` — proxy reads env at process start; SIGHUP only reloads allowlist per [`vault-proxy.py:49`](../../../components/opencli-container/proxy/vault-proxy.py). `--force-recreate` is the correct primitive. Brief restart (~2s) is invisible because the agent isn't running yet.
3. `podman compose up -d vault-agent`
4. Write marker files:
   - `~/.opentrapp/activated` (file presence; no content)
   - `~/.opentrapp/credentials-ok` (unix-millis timestamp)
5. Update settings store: `wizardCompleted: true` (for legacy compatibility; the marker files are the new source of truth)
6. Watchdog observes `vault-agent` running; emits `(ShellReady, Running)` state
7. Modal closes; home screen shows "Your assistant is running safely" with **Open Telegram** primary action

## Half-completed activation handling

The user can close the modal at any point. Behaviour:

- **Cancel during step 1**: nothing persisted. `.env` unchanged. Modal closes; state stays `(ShellReady, Absent)`. Launch button still visible.
- **Cancel during step 2 (after Anthropic validated, before Telegram completes)**: the Anthropic key has been validated but NOT yet written to `.env`. We stash it in component state only. On cancel: discard. **No half-set state in `.env`.**
- **Cancel after step 2 commit but before agent up**: shouldn't normally happen (commit is a single transactional action), but if it does: `.env` has both keys, agent didn't start. Next launch's bootstrap detects `(ShellReady, Absent)` with markers present → migration logic in [`06-migration.md`](06-migration.md) handles it.

The principle: `.env` is only written when both steps are validated. The activation is transactional from the user's perspective — either both keys land or neither does.

## Subsequent activations are one-click

After Phase 3 has succeeded once (markers + `.env` present, `credentials-ok < 7 days`):

1. App launches, bootstrap subsystem runs (idempotent, ~1s)
2. State momentarily `(ShellReady, Absent)`
3. Activation logic detects markers → automatically runs the commit sequence (force-recreate proxy + bring up agent)
4. State transitions to `(ShellReady, Running)` without surfacing the modal
5. Karen sees the home screen go straight to "running safely"

The wizard re-shows only when:
- `~/.opentrapp/activated` is missing (first time, or user reset)
- `~/.opentrapp/credentials-ok` is missing (last activation failed key validation)
- `credentials-ok` is older than 7 days → run a one-token live-ping; if 401, clear marker and open wizard
- Anthropic returns 401 during a real request (proxy logs it; Tauri command clears the marker; next launch goes through wizard)
- User clicks "Reset assistant" in Preferences

`credentials-ok < 7 days` is a soft re-validation cadence to catch silently-revoked keys.

## Plaintext .env disclosure

The activation modal includes a small disclosure line beneath the Anthropic input:

> Your key is stored in plain text on this computer at `~/opentrapp/.env`. We're working on encrypted storage for a future release.

Honest, non-alarming. Honesty here builds trust more than vague reassurance.

## Test coverage

Unit tests in `app/src-tauri/src/commands/credentials.rs`:
- `validate_anthropic_key` returns `Ok` for stubbed 200; `AuthFailure` for 401; correct branches for 402/403/429/5xx
- Timeout returns clean error; never panics
- `redact_secrets` is applied to all error messages

Integration tests:
- Real call to `api.anthropic.com` with a known-bad key (401) and a known-good test-account key (200)
- Telegram handoff sequence against a real test bot: end-to-end /start → test-message → confirm-offset → no 409 from a follow-up poll

E2E tests in [`app/e2e/activation.spec.ts`](../../../app/e2e/) (new):
- Launch button click opens modal in `(ShellReady, Absent)`
- Cancel mid-flow leaves `.env` untouched
- Successful commit transitions hero to "running safely"
- "Skip and test later" path writes token without test message; flag is honored on next launch

## Out of scope

- **Embedded webview for Anthropic console signup** — manual modal stays; live-ping closes the failure mode
- **Multi-key management** (work account vs personal) — single key per install
- **Webhook-based Telegram** — long-polling is OpenClaw's default; webhooks need a public URL on the laptop, forbidden by `CLAUDE.md` §10
- **Re-running activation while the agent is already running** — requires stopping vault-agent first; the modal disables the launch trigger when state is `(ShellReady, Running)`
- **Telegram chat-ID allowlist** — the bot's `dmPolicy: "pairing"` ([`openclaw-hardening.json5:92`](../../../components/opencli-container/config/openclaw-hardening.json5)) handles this; we just send the test message to whoever sent /start
