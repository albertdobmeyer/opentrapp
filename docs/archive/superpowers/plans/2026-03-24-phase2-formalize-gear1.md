# Phase 2: Formalize Gear 1 (Manual) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Formalize the current vault as Gear 1 (Manual), set up Telegram for agent interaction, and prove the thesis: a non-technical user can safely interact with OpenClaw in fully locked-down mode.

**Architecture:** Gear 1 config is already working (Phase 1 proved it). This phase adds Telegram as the interaction channel, creates the gear-switching infrastructure, and tests the full user flow.

**Tech Stack:** Podman 4.9.3, JSON5 config, Telegram Bot API, bash scripts

**Spec reference:** Section 5.4 (Gear 1 definition) of the design spec.

**Working directory:** `components/openclaw-vault`

**SAFETY:** Haiku-only API key, $5 cap, no web tools. Exec security = deny. Tool profile = minimal. Telegram DM policy = pairing (must manually approve each sender).

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `config/openclaw-hardening.json5` | Add Telegram bot token via env var fallback |
| Modify | `compose.yml` | Pass TELEGRAM_BOT_TOKEN env var to vault container |
| Modify | `.env` | Add Telegram bot token |
| Create | `config/gear1-allowlist.txt` | Gear 1-specific allowlist (LLM APIs only) |
| Modify | `proxy/allowlist.txt` | Remove raw.githubusercontent.com for Gear 1 |
| Create | `scripts/switch-gear.sh` | Gear-switching infrastructure script |
| Modify | `component.yml` | Add gear-related states and commands |
| Modify | `scripts/verify.sh` | Add Gear 1-specific checks |
| Create | `docs/phase2-test-results.md` | End-to-end test documentation |
| Remove | `config/openclaw-hardening.yml` | Replace with .json5 (old YAML is dead code) |

---

### Task 1: Set Up Telegram Bot

**Files:**
- Modify: `.env`

This task requires the user to create a Telegram bot. The steps are manual (Telegram interaction), not automatable.

- [ ] **Step 1: User creates a Telegram bot via @BotFather**

User must open Telegram and chat with `@BotFather`:
1. Send `/newbot`
2. Choose a name (e.g., "OpenClaw Vault Test")
3. Choose a username (e.g., `openclaw_vault_test_bot`)
4. Copy the bot token (format: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`)

- [ ] **Step 2: Add bot token to .env file**

Append to `components/openclaw-vault/.env`:
```
TELEGRAM_BOT_TOKEN=<paste token here>
```

File permissions should already be 600 (set in Phase 1).

- [ ] **Step 3: Update compose.yml to pass token to vault container**

The vault container needs the token so OpenClaw's Telegram plugin can connect. Add to the vault service's environment section in `compose.yml`:
```yaml
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
```

**SECURITY NOTE:** The Telegram bot token is NOT an API key — it authenticates the bot with Telegram's servers. Unlike the Anthropic API key (which stays in the proxy), the Telegram token must be in the vault container because OpenClaw's Telegram plugin runs there. This is acceptable because:
- The token only lets someone send/receive messages AS this bot
- It can't access the user's personal Telegram account
- It can be revoked via @BotFather at any time

- [ ] **Step 4: Verify token is in the config**

OpenClaw reads `TELEGRAM_BOT_TOKEN` from the environment as a fallback when `channels.telegram.botToken` is not set in config. Our config already has `channels.telegram.enabled` implicitly via the `dmPolicy` setting.

No config file changes needed — the env var fallback handles it.

- [ ] **Step 5: Commit**

```bash
git add compose.yml
git commit -m "feat: pass Telegram bot token to vault container via env var"
```

(Do NOT commit .env — it's gitignored)

---

### Task 2: Clean Up Gear 1 Allowlist

**Files:**
- Modify: `proxy/allowlist.txt`
- Remove: `config/openclaw-hardening.yml` (dead YAML config)

- [ ] **Step 1: Remove raw.githubusercontent.com from allowlist**

Per the spec (Section 5.4, Gear 1): "Allowlist: LLM API providers only." `raw.githubusercontent.com` is not needed in Gear 1 (no skill downloading, no code review).

Edit `proxy/allowlist.txt` to comment out or remove `raw.githubusercontent.com`.

Final allowlist for Gear 1:
```
api.anthropic.com
api.openai.com
# Telegram API (needed for bot communication)
api.telegram.org
```

**IMPORTANT:** Add `api.telegram.org` — OpenClaw's Telegram plugin needs to reach Telegram's API. Without this, the bot can't connect.

- [ ] **Step 2: Remove dead YAML config**

```bash
rm config/openclaw-hardening.yml
```

This file is no longer used. The active config is `config/openclaw-hardening.json5`. The old YAML had wrong key paths and wrong format.

- [ ] **Step 3: Commit**

```bash
git add proxy/allowlist.txt
git rm config/openclaw-hardening.yml
git commit -m "chore: clean up Gear 1 allowlist and remove dead YAML config

- Remove raw.githubusercontent.com (not needed in Gear 1)
- Add api.telegram.org (needed for Telegram bot communication)
- Delete openclaw-hardening.yml (replaced by .json5 in Phase 1)"
```

---

### Task 3: Test Telegram Connection Inside Vault

**Files:** None created. Pure testing.

- [ ] **Step 1: Rebuild and start the stack**

```bash
podman rm -f vault-proxy openclaw-vault 2>/dev/null
podman rmi openclaw-vault_vault openclaw-vault 2>/dev/null
podman volume rm openclaw-vault_vault-proxy-logs openclaw-vault_proxy-ca 2>/dev/null
podman build -t openclaw-vault -f Containerfile . && podman tag openclaw-vault openclaw-vault_vault
podman-compose up -d
```

Wait ~60s for startup, then verify:
```bash
podman logs openclaw-vault 2>&1 | grep -E 'telegram|Telegram|listening|Gateway'
```

Expected: Telegram plugin starts, gateway listening, bot connects to Telegram API.

- [ ] **Step 2: Check proxy logs for Telegram API calls**

```bash
podman exec vault-proxy cat /var/log/vault-proxy/requests.jsonl | head -10
```

Expected: ALLOWED requests to `api.telegram.org` (Telegram long-polling).

**If Telegram requests are BLOCKED (403):** The allowlist is missing `api.telegram.org`. Fix and restart.

- [ ] **Step 3: Send a test message from your phone**

1. Open Telegram on your phone
2. Search for your bot username
3. Send: "Hello"
4. The bot should NOT respond yet (DM pairing required)

- [ ] **Step 4: Approve the pairing**

```bash
podman exec openclaw-vault openclaw pairing list telegram
```

This should show your pending pairing request with a code.

```bash
podman exec openclaw-vault openclaw pairing approve telegram <CODE>
```

**If this fails with device token error:** The CLI needs device auth. Try:
```bash
podman exec openclaw-vault openclaw devices list
```
If device auth blocks all CLI commands, we'll need to explore alternative pairing methods.

- [ ] **Step 5: Test agent interaction**

After pairing is approved, send from Telegram:
```
What is 2 + 2?
```

Expected:
- The agent responds with "4" (or similar)
- The response comes from Haiku (our model config)
- The proxy logs show an ALLOWED request to `api.anthropic.com`
- No tool use is attempted (simple math question doesn't need tools)

- [ ] **Step 6: Test Gear 1 restrictions**

Send from Telegram:
```
List the files in /home
```

Expected in Gear 1 (exec denied, tool profile minimal):
- The agent should NOT be able to execute `ls /home`
- It should either refuse or explain it doesn't have access to tools
- The proxy should NOT show any new requests (no tool execution attempted)

Check proxy logs:
```bash
podman exec vault-proxy cat /var/log/vault-proxy/requests.jsonl | tail -5
```

- [ ] **Step 7: Record results**

Document in notes:
- Did Telegram connect? Y/N
- Did pairing work? Y/N (note if CLI device auth was needed)
- Did the agent respond? Y/N
- Was exec correctly denied? Y/N
- Were proxy logs clean? Y/N

---

### Task 4: Add Gear-Switching Infrastructure

**Files:**
- Create: `scripts/switch-gear.sh`

This is the foundation for all three gears. For now it only implements Gear 1, but the structure supports Gears 2 and 3.

- [ ] **Step 1: Create the gear-switching script**

```bash
#!/usr/bin/env bash
# OpenClaw-Vault: Gear Switching
# Usage: bash scripts/switch-gear.sh <gear>
#   gear: manual | semi | full
set -uo pipefail

GEAR="${1:-}"
VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

case "$GEAR" in
    manual|1)
        echo "[gear] Switching to Gear 1 (Manual)..."
        cp "$VAULT_DIR/config/openclaw-hardening.json5" "$VAULT_DIR/config/active-config.json5"
        cp "$VAULT_DIR/config/gear1-allowlist.txt" "$VAULT_DIR/proxy/allowlist.txt"
        echo "[gear] Config: Gear 1 — exec denied, tool profile minimal, DM pairing"
        ;;
    semi|2)
        echo "[gear] Gear 2 (Semi-Auto) not yet implemented."
        exit 1
        ;;
    full|3)
        echo "[gear] Gear 3 (Full-Auto) not yet implemented."
        exit 1
        ;;
    *)
        echo "Usage: $0 <manual|semi|full>"
        echo "  manual (1): Maximum lockdown — every action requires approval"
        echo "  semi   (2): Selective tool access (not yet implemented)"
        echo "  full   (3): Broad autonomy, driver seat protected (not yet implemented)"
        exit 1
        ;;
esac

echo "[gear] Restarting vault to apply new configuration..."
podman-compose -f "$VAULT_DIR/compose.yml" down
podman-compose -f "$VAULT_DIR/compose.yml" up -d
echo "[gear] Done. Run 'bash scripts/verify.sh' to confirm."
```

- [ ] **Step 2: Create Gear 1 allowlist template**

Create `config/gear1-allowlist.txt`:
```
# Gear 1 (Manual) — LLM APIs + Telegram only
# No raw.githubusercontent.com, no ClawHub, no npm
api.anthropic.com
api.openai.com
api.telegram.org
```

- [ ] **Step 3: Rename current config to clarify it's Gear 1**

The current `openclaw-hardening.json5` IS the Gear 1 config. Keep the name (it's descriptive) but ensure the gear-switching script references it correctly.

- [ ] **Step 4: Make script executable and commit**

```bash
chmod +x scripts/switch-gear.sh
git add scripts/switch-gear.sh config/gear1-allowlist.txt
git commit -m "feat: add gear-switching infrastructure with Gear 1 support

scripts/switch-gear.sh switches between gears by swapping config files
and allowlists, then restarting the container stack. Gear 2 and 3 are
stubbed with clear error messages.

config/gear1-allowlist.txt: LLM APIs + Telegram only."
```

---

### Task 5: Update component.yml for Gear States

**Files:**
- Modify: `component.yml`

The GUI needs to know about gears. Add gear-specific states and the switch-gear command.

- [ ] **Step 1: Add gear states to component.yml status section**

Add new states after the existing ones:
```yaml
    - id: running-manual
      label: "Running (Gear 1: Manual)"
      icon: shield-check
      color: "#22c55e"
    - id: running-semi
      label: "Running (Gear 2: Semi-Auto)"
      icon: shield-half
      color: "#eab308"
    - id: running-full
      label: "Running (Gear 3: Full-Auto)"
      icon: shield-alert
      color: "#f97316"
```

- [ ] **Step 2: Add gear-switching command**

Add to the commands section:
```yaml
  - id: switch-gear-manual
    name: "Switch to Gear 1 (Manual)"
    description: "Maximum lockdown — every action requires your approval"
    group: lifecycle
    type: action
    danger: caution
    command: bash scripts/switch-gear.sh manual
    output:
      format: text
      display: log
    available_when: [running, running-manual, running-semi, running-full, stopped]
    sort_order: 5
    timeout_seconds: 120
```

- [ ] **Step 3: Commit**

```bash
git add component.yml
git commit -m "feat: add gear states and switch-gear-manual command to component.yml"
```

---

### Task 6: Run Gear 1 Verification and Document

**Files:**
- Create: `docs/phase2-test-results.md`

- [ ] **Step 1: Run full verification**

```bash
bash scripts/verify.sh
```
Expected: 15/15 pass.

- [ ] **Step 2: Run gear-switching script**

```bash
bash scripts/switch-gear.sh manual
```
Verify it restarts the stack with Gear 1 config.

- [ ] **Step 3: Test Telegram interaction (if Task 3 succeeded)**

Send a few messages via Telegram:
1. "What is the capital of France?" — should get a direct answer
2. "Read my /etc/passwd file" — should be refused (no file tools)
3. "Run `ls /tmp`" — should be refused (exec denied)

- [ ] **Step 4: Document Phase 2 results**

Create `docs/phase2-test-results.md` with:
- Telegram connection status
- Gear 1 restriction verification
- verify.sh results
- Any issues found
- Screenshot of Telegram conversation (optional)

- [ ] **Step 5: Commit and push everything**

```bash
git add docs/phase2-test-results.md
git commit -m "docs: Phase 2 complete — Gear 1 formalized and tested"
git push origin main
```

Then update parent repo:
```bash
cd lobster-trapp
git add components/openclaw-vault
git commit -m "chore: update openclaw-vault — Phase 2 (Gear 1 formalized)"
git push origin main
```

---

## Exit Criteria

- [ ] Telegram bot created and token stored in .env
- [ ] Telegram bot connects to OpenClaw inside the vault
- [ ] DM pairing works (user approved via CLI or alternative method)
- [ ] Agent responds to simple questions via Telegram
- [ ] Agent correctly refuses file/exec requests in Gear 1
- [ ] Gear-switching script exists and works for Gear 1
- [ ] component.yml has gear states and switch command
- [ ] Old YAML config removed
- [ ] Allowlist cleaned (no raw.githubusercontent.com in Gear 1)
- [ ] All changes committed and pushed
