# Phase 1: Verify OpenClaw Compatibility — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run OpenClaw inside the vault for the first time, verify all security controls work live, and answer the open questions that block all future phases.

**Architecture:** No new code unless bugs are found. This is an investigation phase — we build, start, observe, test, and document. Every finding either confirms or challenges our spec's assumptions.

**Tech Stack:** Podman 4.9.3, podman-compose 1.0.6, existing vault codebase, Anthropic API (Haiku-only, $5 cap)

**Spec reference:** Open Questions 1, 4, 8, and Layer 4 validation from `docs/superpowers/specs/2026-03-23-openclaw-vault-security-harness-design.md`

**Working directory:** `components/openclaw-vault`

**SAFETY:** The API key is Haiku-only with a $5 spending cap. No web tools enabled. ClawHub domains blocked. SSH key backed up to `/tmp/ssh_backup_20260323`. If anything unexpected happens, run `bash scripts/kill.sh --hard` immediately.

---

## What We Need To Learn

| Open Question | What We Need To Do | Blocks |
|--------------|-------------------|--------|
| OQ4: Does OpenClaw @2026.2.17 work on Node 20? | Build the container. If `npm install` or `openclaw` fails, we know. | Everything |
| OQ8: What config format does OpenClaw accept? | Try our YAML config. Check `openclaw --help` for format docs. Try JSON. | Gear configs |
| OQ8b: Do our YAML keys match OpenClaw's actual config schema? | Start OpenClaw, check logs for unknown/ignored keys, compare behavior. | Security claims |
| OQ1: Can we reconfigure via Gateway WebSocket? | Check if gateway starts, inspect its API. | Gear switching |
| Layer 4: What happens when sandbox mode is set but no Docker socket? | Start OpenClaw with `sandbox.mode: "non-main"`, observe behavior. | Spec accuracy |
| NEW: Does OpenClaw need domains beyond our allowlist to start? | Watch proxy logs during startup for blocked requests. | Basic functionality |

---

## File Map

No new files created. Potential modifications based on findings:

| Action | File | Condition |
|--------|------|-----------|
| May modify | `config/openclaw-hardening.yml` | If config keys don't match OpenClaw's schema |
| May modify | `Containerfile` (lines 7-8, 15) | If Node 20 doesn't work and we need Node 22 |
| May modify | `proxy/allowlist.txt` | If OpenClaw needs additional domains to start |
| Create | `docs/phase1-findings.md` | Document all answers to open questions |

---

### Task 1: Build The Container Image

**Files:**
- Read: `Containerfile`
- Read: `compose.yml`

This is the first real test. If the build fails on `npm install -g @anthropic-ai/openclaw@2026.2.17` with Node 20, we know OQ4's answer immediately.

- [ ] **Step 1: Build the vault container image**

Run:
```bash
cd components/openclaw-vault
podman build -t openclaw-vault -f Containerfile . 2>&1 | tee /tmp/vault-build.log
```

**If SUCCEEDS:** Node 20 works with OpenClaw @2026.2.17. OQ4 partially answered. Proceed.

**If FAILS with Node version error:** Node 20 is incompatible. Fix:
1. Update Containerfile lines 7-8 and 15 to `node:22-alpine` with a fresh digest pin
2. Rebuild: `podman build -t openclaw-vault -f Containerfile .`
3. Document the Node 20 incompatibility in findings

**If FAILS with npm/network error:** The build stage needs internet to pull npm packages. This is expected — the builder stage is not behind the proxy. Check network connectivity and retry.

- [ ] **Step 2: Verify the built image**

Run:
```bash
podman images openclaw-vault
```
Expected: Image exists with recent timestamp.

Run:
```bash
podman run --rm openclaw-vault openclaw --version 2>/dev/null || podman run --rm --entrypoint "" openclaw-vault node -e "console.log(process.version)"
```
Expected: OpenClaw version OR Node version confirms what's installed.

- [ ] **Step 3: Record findings**

Document in notes:
- Did the build succeed on Node 20? Y/N
- What OpenClaw version is installed?
- What Node version is in the image?
- Any warnings during build?

---

### Task 2: Start The Compose Stack

**Files:**
- Read: `compose.yml`
- Read: `.env` (exists, has API key)

- [ ] **Step 1: Verify .env file exists and has the key**

Run:
```bash
test -f .env && echo ".env exists" && grep -c 'ANTHROPIC_API_KEY' .env
```
Expected: `.env exists` and `1` (one key entry).

- [ ] **Step 2: Start the stack**

Run:
```bash
cd components/openclaw-vault
podman-compose up -d 2>&1 | tee /tmp/vault-start.log
```

- [ ] **Step 3: Check both containers are running**

Run:
```bash
podman ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
```
Expected: Both `openclaw-vault` and `vault-proxy` show as "Up".

**If openclaw-vault exits immediately:** Check logs:
```bash
podman logs openclaw-vault 2>&1 | tee /tmp/vault-container.log
```
Common failure causes:
- Entrypoint can't find hardening config → check `/opt/openclaw-hardening.yml` in image
- Proxy CA cert timeout → check if vault-proxy started first
- OpenClaw crashes on startup → Node version or config issue
- Config format not recognized → OQ8 answered (negative)

**If vault-proxy exits:** Check logs:
```bash
podman logs vault-proxy 2>&1 | tee /tmp/proxy-container.log
```
Common failure causes:
- vault-proxy.py syntax error → check Python version in mitmproxy image
- Allowlist not found → check volume mount

- [ ] **Step 4: Watch startup logs for both containers**

Run:
```bash
podman logs openclaw-vault 2>&1 | head -50
echo "---"
podman logs vault-proxy 2>&1 | head -50
```

Look for:
- OpenClaw startup messages (version, gateway binding, config loaded)
- Any "unknown config key" or "ignoring" warnings → OQ8b answer
- Any network requests that fail → domains OpenClaw needs
- Proxy messages about loaded allowlist and blocked requests

- [ ] **Step 5: Check proxy logs for blocked requests during startup**

Run:
```bash
podman exec vault-proxy cat /var/log/vault-proxy/requests.jsonl 2>/dev/null | head -20
```

This shows what OpenClaw tried to reach during startup:
- If there are BLOCKED entries for unknown domains → OpenClaw needs those domains
- If there are ALLOWED entries → we see the first API calls
- If empty → OpenClaw made no network requests during startup

- [ ] **Step 6: Record findings**

Document:
- Did both containers start? Y/N
- Any startup errors or warnings?
- What config keys did OpenClaw recognize/ignore?
- What domains did OpenClaw try to reach?
- Did the proxy correctly block non-allowlisted domains?

---

### Task 3: Run Security Verification

**Files:**
- Run: `scripts/verify.sh`
- Run: `tests/test-network-isolation.sh`

This is the first live validation of our security claims. Every check in verify.sh has been code-reviewed but never run against a live container.

- [ ] **Step 1: Run the 15-point security check**

Run:
```bash
bash scripts/verify.sh 2>&1 | tee /tmp/vault-verify.log
```
Expected: All 15 checks PASS.

**If any check FAILS:** This is a critical finding. Document exactly which check failed and why. Do NOT proceed until all 15 pass — the security thesis depends on these.

- [ ] **Step 2: Run network isolation tests**

Run:
```bash
bash tests/test-network-isolation.sh 2>&1 | tee /tmp/vault-network-tests.log
```
Expected: All 7 tests pass (6 pass + 1 informational).

- [ ] **Step 3: Run the existing unit test suites**

Run all test scripts in the tests/ directory:
```bash
for t in tests/test-*.sh; do
    echo "=== Running: $t ==="
    bash "$t" 2>&1
    echo ""
done 2>&1 | tee /tmp/vault-all-tests.log
```

Record which tests pass and fail. Some tests (like test-kill-switch.sh) may need the container to be running in a specific state.

- [ ] **Step 4: Record findings**

Document:
- verify.sh results: X/15 passed
- Network isolation: X/7 passed
- Other test results
- Any unexpected failures and root causes

---

### Task 4: Explore OpenClaw Inside The Container

**Files:**
- None modified. Pure exploration.

- [ ] **Step 1: Get OpenClaw version and help**

Run:
```bash
podman exec openclaw-vault openclaw --version 2>&1
podman exec openclaw-vault openclaw --help 2>&1 | tee /tmp/openclaw-help.log
```

Look for: version number, available subcommands, config-related flags (--config, --format, --json, etc.)

- [ ] **Step 2: Check what config formats are accepted**

Run:
```bash
podman exec openclaw-vault openclaw --help 2>&1 | grep -iE 'config|yaml|json|format'
```

Also check if there's built-in config validation:
```bash
podman exec openclaw-vault openclaw config --help 2>&1 || echo "No config subcommand"
podman exec openclaw-vault openclaw validate-config --help 2>&1 || echo "No validate-config"
```

- [ ] **Step 3: Check what OpenClaw actually loaded from our config**

Run:
```bash
podman exec openclaw-vault cat /home/vault/.config/openclaw/config.yml
```
Verify: This matches our `config/openclaw-hardening.yml` (copied by entrypoint.sh).

If OpenClaw has a way to dump its running config:
```bash
podman exec openclaw-vault openclaw config show 2>&1 || echo "No config show command"
podman exec openclaw-vault openclaw status 2>&1 || echo "No status command"
```

- [ ] **Step 4: Check if the Gateway is running and what port it uses**

Run:
```bash
podman exec openclaw-vault sh -c "ss -tlnp 2>/dev/null || netstat -tlnp 2>/dev/null" || echo "No network tools"
```

Look for: A listener on port 18789 (ws://127.0.0.1:18789 per OpenClaw docs) or similar.

Alternative: check if the OpenClaw process is running:
```bash
podman exec openclaw-vault ps aux 2>/dev/null || podman exec openclaw-vault sh -c "ls /proc/*/cmdline 2>/dev/null | head -10"
```

- [ ] **Step 5: Check OpenClaw's behavior with sandbox mode and no Docker socket**

The hardening config sets `sandbox.mode: "non-main"`. Inside the container, there is no Docker/Podman socket. Check:
```bash
podman logs openclaw-vault 2>&1 | grep -iE 'sandbox|docker|podman|container|socket'
```

Look for: warnings about missing Docker socket, sandbox mode falling back, or sandbox being ignored.

- [ ] **Step 6: Check if OpenClaw has a CLI interaction mode**

Run:
```bash
podman exec -it openclaw-vault openclaw chat --help 2>&1 || echo "No chat subcommand"
podman exec -it openclaw-vault openclaw cli --help 2>&1 || echo "No cli subcommand"
podman exec -it openclaw-vault openclaw session --help 2>&1 || echo "No session subcommand"
```

We need to know if there's a way to interact with OpenClaw that doesn't require Telegram/WhatsApp. The CLI or WebChat might work.

- [ ] **Step 7: Record findings**

Document:
- OpenClaw version and available commands
- Config format(s) supported (YAML, JSON, both?)
- Whether our config keys are recognized
- Gateway status and port
- Sandbox behavior without Docker socket
- Available interaction methods (CLI, WebChat, etc.)

---

### Task 5: Test A Simple OpenClaw Interaction (If Possible)

**SAFETY:** This task uses the Anthropic API. The key is Haiku-only with $5 cap. Each Haiku request costs fractions of a cent. We will make at most 2-3 test requests.

**Files:**
- May modify: `config/openclaw-hardening.yml` (if model config needed)

- [ ] **Step 1: Determine if OpenClaw needs model configuration**

Check if our hardening config needs an `agent.model` setting. Check OpenClaw logs:
```bash
podman logs openclaw-vault 2>&1 | grep -iE 'model|agent|provider|anthropic|haiku'
```

If OpenClaw is complaining about no model being configured, we need to add it to the hardening config:
```yaml
agent:
  model: "anthropic/claude-haiku-4-5"
```

If we need to modify the config, update `config/openclaw-hardening.yml`, rebuild or restart:
```bash
podman-compose down
podman-compose up -d
```

- [ ] **Step 2: Attempt a simple interaction**

Based on Task 4 Step 6 findings, try ONE of these approaches (whichever is available):

**Option A: CLI interaction**
```bash
podman exec -it openclaw-vault openclaw chat "What is 2 + 2?"
```

**Option B: Gateway API (if running)**
```bash
podman exec openclaw-vault node -e "
const ws = require('ws');
const c = new ws.WebSocket('ws://127.0.0.1:18789');
c.on('open', () => { c.send(JSON.stringify({type:'message',content:'What is 2+2?'})); });
c.on('message', d => { console.log(d.toString()); c.close(); });
c.on('error', e => { console.error(e.message); process.exit(1); });
setTimeout(() => process.exit(0), 10000);
"
```

**Option C: If no interaction method works**
Skip this step. Document that OpenClaw requires a messaging channel (Telegram/WhatsApp) to interact and cannot be tested via CLI alone. This itself is an important finding for the spec.

- [ ] **Step 3: Check proxy logs for the API call**

Run:
```bash
podman exec vault-proxy cat /var/log/vault-proxy/requests.jsonl 2>/dev/null | tail -5
```

Look for:
- An ALLOWED request to `api.anthropic.com`
- The request method and path (should be POST /v1/messages)
- No API key visible in the log (it should NOT be logged)

- [ ] **Step 4: Check approval mode behavior**

Our config sets `exec.approvals.mode: "always"`. If we sent a message and OpenClaw tries to use a tool (exec, read, etc.), it should request approval. Check:
```bash
podman logs openclaw-vault 2>&1 | grep -iE 'approval|confirm|deny|tool|exec'
```

A simple "What is 2+2?" should NOT trigger any tool use — the LLM should answer directly.

- [ ] **Step 5: Record findings**

Document:
- Did the interaction work? Y/N
- What model did OpenClaw use? (Check proxy logs for the model parameter)
- Did the proxy correctly inject the API key? (Check if the request succeeded)
- Did the proxy log the request without exposing the key?
- What approval behavior was observed?

---

### Task 6: Teardown and Document

**Files:**
- Create: `docs/phase1-findings.md`

- [ ] **Step 1: Stop the stack cleanly**

Run:
```bash
bash scripts/kill.sh --soft
```
Verify containers are stopped:
```bash
podman ps -a --format "table {{.Names}}\t{{.Status}}"
```

- [ ] **Step 2: Check memory impact**

Run:
```bash
free -h
```
Verify system recovered memory after container teardown.

- [ ] **Step 3: Verify SSH key is intact**

Run:
```bash
diff ~/.ssh/hetzner_linuxlaptop /tmp/ssh_backup_20260323 && echo "SSH key unchanged — vault did not touch it"
```

- [ ] **Step 4: Write Phase 1 findings document**

Create `docs/phase1-findings.md` with all answers to open questions, organized as:

```markdown
# Phase 1 Findings — OpenClaw Compatibility Verification

**Date:** 2026-03-23
**Vault version:** openclaw-vault @ phase0-bug-fixes
**OpenClaw version:** @anthropic-ai/openclaw@2026.2.17
**Test key:** Anthropic Haiku-only, $5 cap

## Open Question Answers

### OQ4: Does OpenClaw work on Node 20?
[Answer with evidence]

### OQ8: Config format — YAML, JSON, or both?
[Answer with evidence]

### OQ8b: Do our config keys match OpenClaw's schema?
[Answer with evidence — list any keys that were ignored or unrecognized]

### OQ1: Gateway WebSocket API availability
[Answer with evidence]

### Layer 4: Sandbox behavior without Docker socket
[Answer with evidence]

### NEW: Domains OpenClaw needs beyond our allowlist
[List any blocked domains from proxy logs]

## Security Verification Results
- verify.sh: X/15 passed
- Network isolation: X/7 passed
- Other tests: [results]

## Key Discoveries
[Anything unexpected that affects the spec or roadmap]

## Impact On Later Phases
[How these findings change Phase 2-6 plans]
```

- [ ] **Step 5: Commit findings**

```bash
cd components/openclaw-vault
git add docs/phase1-findings.md
git commit -m "docs: add Phase 1 findings — OpenClaw compatibility verification"
```

If any code changes were made (Containerfile upgrade, config fixes):
```bash
git add -A
git commit -m "fix: [description of what was fixed based on Phase 1 findings]"
```

- [ ] **Step 6: Delete SSH backup**

```bash
rm /tmp/ssh_backup_20260323
echo "SSH backup removed"
```

---

## Decision Points

This plan has branching paths based on findings:

```
Build succeeds on Node 20?
  ├─ YES → proceed
  └─ NO → upgrade to Node 22, rebuild, then proceed

Both containers start?
  ├─ YES → proceed
  └─ NO → diagnose from logs, fix, then proceed

verify.sh passes all 15 checks?
  ├─ YES → proceed (security thesis holds live!)
  └─ NO → STOP. Fix failures before ANY further work.

OpenClaw recognizes our config keys?
  ├─ YES → spec Layer 4-6 assumptions are valid
  └─ NO → must remap config keys before Phase 2
       (this is the most likely failure mode)

OpenClaw can be interacted with from CLI?
  ├─ YES → we can test all gears without Telegram setup
  └─ NO → Phase 2+ requires messaging channel setup first

OpenClaw needs domains we haven't allowlisted?
  ├─ NO → current allowlist is sufficient
  └─ YES → evaluate each domain, add if safe, document if blocked
```

## Exit Criteria

- [ ] Container image builds successfully (Node version confirmed)
- [ ] Both containers start and stay running
- [ ] All 15 verify.sh checks pass
- [ ] All 7 network isolation tests pass
- [ ] Open Questions 1, 4, and 8 answered with evidence
- [ ] Layer 4 sandbox behavior documented
- [ ] Phase 1 findings document written and committed
- [ ] System resources (RAM, disk) confirmed recovered after teardown
- [ ] SSH key verified unchanged
- [ ] All code changes (if any) committed and pushed
