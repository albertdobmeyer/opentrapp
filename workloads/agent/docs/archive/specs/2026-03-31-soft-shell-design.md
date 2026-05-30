# Spec: Soft Shell Design — The Safari

**Date:** 2026-03-31
**Phase:** 7
**Security implications:** This is the highest-autonomy shell level. Every design decision either strengthens or weakens the moat between the agent and the user's machine.

---

## The Safari Analogy

**Hard Shell** = the cage. Lion is locked in, secure, can't do anything. Safe but useless.

**Split Shell** = the arena. Lion has certain tools, but the tamer (Claude/user) controls and approves every single action. Useful but slow.

**Soft Shell** = the safari. Lion roams freely within a vast fenced territory. It can hunt, explore, and act autonomously. But the moat is absolute — the lion can never reach the village (the user's identity, keys, system). The tamer watches from the observation tower (monitoring) and can pull the kill switch at any time.

## What Soft Shell Enables

The agent becomes genuinely useful — not just a chatbot, but an autonomous assistant that eliminates categories of work:

| Use Case | What Hum Does | Tools Used |
|---|---|---|
| Personal assistant | Morning briefings, calendar reminders, scheduled via Telegram | cron, web_search, read, write, message |
| Research assistant | Searches web, reads articles, summarizes, caches in workspace | web_search, web_fetch, read, write |
| Content automation | Generates posts, reads sources, writes drafts | web_search, web_fetch, read, write |
| Document processing | Reads files, extracts data, categorizes | read, write, exec (safeBins) |
| Competitive intelligence | Monitors feeds on schedule, alerts on keywords | web_search, web_fetch, cron |
| File management | Organizes workspace, creates directories, copies files | exec (safeBins), read, write, edit |

## The Moat — What the Agent Can NEVER Do

These restrictions hold in ALL shell levels, including Soft Shell. They are enforced by the container exoskeleton (Layer 1) and can never be weakened by tool policy changes.

### Identity (never mounted)
- `~/.ssh` — SSH keys
- `~/.gnupg` — GPG keys
- `~/.local/share/keyrings` — GNOME keyring / saved passwords
- Browser saved passwords — Soft Shell browser uses fresh Chromium profile

### System Integrity (never mounted, read-only root)
- `/etc`, `/sys`, `/boot`, `/proc` (host) — container has its own isolated versions
- Root / sudo — non-root user, sudo stripped, no-new-privileges
- systemd / services — seccomp blocks mount, unshare, setns
- Docker / Podman socket — never mounted, can't spawn containers

### Self-Modification (config locked, tool denied)
- `gateway` tool — permanently denied, agent can't modify its own security
- Vault config — chmod 444 + integrity hash detection (check #24)
- `nodes` tool — permanently denied, no device access
- `bash` tool — permanently denied (distinct from exec, always blocked)

### Destructive Operations (agent is constructive only)
- `rm`, `rmdir` — stripped from container image
- `chown`, `chgrp` — stripped from container image
- Interpreters (`python`, `bash`, `ruby`, `perl`) — stripped from image, not in safeBins

### Supply Chain (domains blocked)
- `clawdhub.com`, `www.clawhub.ai` — 11.9% malware rate, permanently blocked
- Private/internal network ranges — blocked by proxy

## Soft Shell Configuration

### Profile and Tool Policy

```
profile: "coding"
```

NOT `"full"` — the `coding` profile includes all the tools we need (fs, runtime, sessions, memory, image) and we explicitly enable/deny from there. Using `"full"` would include `nodes` and other tools we never want.

### Enabled Tools (the safari territory)

| Tool | Risk | Why Enabled |
|---|---|---|
| read | high | File access within workspace |
| write | high | Create files, notes, drafts |
| edit | high | Modify existing files |
| apply_patch | high | Programmatic file changes |
| exec | critical | Shell commands via safeBins (no interpreters, no rm) |
| grep | medium | Search file contents |
| find | medium | Locate files |
| ls | medium | List directories |
| memory_search | low | Search agent memory |
| memory_get | low | Retrieve memory entries |
| image | medium | Analyze images |
| web_search | medium | Search the web |
| web_fetch | medium | Fetch URL content |
| cron | high | Schedule recurring tasks |
| process | critical | Background task management |
| canvas | low | Visual workspace |
| message | high | Send messages on authorized channels |

**17 tools enabled.** Risk score: ~0.45 on the 0.0-0.9 scale.

### Denied Tools (the moat)

| Tool | Why Denied |
|---|---|
| gateway | NEVER — agent can't modify itself |
| bash | NEVER — distinct exec path, always blocked |
| nodes | NEVER — no device access |
| browser | Denied by default. User can enable via tool-control as a per-capability toggle. |
| sessions_spawn | Denied by default. User can enable for multi-agent workflows. |
| sessions_send | Denied unless sessions_spawn enabled |
| sessions_list | Denied unless sessions_spawn enabled |
| sessions_history | Denied unless sessions_spawn enabled |
| session_status | Denied unless sessions_spawn enabled |

### Exec Configuration

```json5
exec: {
  security: "allowlist",
  ask: "on-miss",           // Auto-approve safeBins, prompt for unknowns
  host: "gateway",
  safeBins: [
    // All Split Shell safeBins (16) PLUS:
    "cat", "echo", "mkdir", "cp", "mv", "touch", "date",
    "head", "tail", "wc", "sort", "uniq", "tr", "cut", "jq", "tee",
    // New in Soft Shell:
    "grep", "sed", "awk", "diff", "xargs", "basename", "dirname",
    "env", "true", "false", "test", "printf"
  ],
  safeBinProfiles: {
    // Empty for now — all args allowed
    // Future: add deniedFlags per binary for tighter control
  }
}
```

**Key change from Split Shell:** `ask: "on-miss"` instead of `ask: "always"`. SafeBins commands execute without Telegram approval. Unknown commands still require approval.

This is the safari: the lion runs freely within the fence (safeBins), but hitting the fence (unknown command) triggers the tamer (user approval).

### Proxy Allowlist

Soft Shell broadens the domain allowlist beyond the 3 base domains:

```
# Base (always)
api.anthropic.com
api.openai.com
api.telegram.org

# Soft Shell additions
raw.githubusercontent.com    # Read public GitHub repos (skill source review)
```

Additional domains are user-configurable via `tool-control.sh` or the GUI. Each added domain is logged and shown in `make tools-status`.

The allowlist is still DENY-BY-DEFAULT. Only listed domains are reachable. The proxy logs every request.

### Approval Model

| Action | Split Shell | Soft Shell |
|---|---|---|
| SafeBins command (cat, grep, etc.) | Requires Telegram approval | Auto-approved (on-miss) |
| Unknown command | Requires Telegram approval | Requires Telegram approval |
| File read/write in workspace | Auto (workspaceOnly) | Auto (workspaceOnly) |
| Web search/fetch | N/A (denied) | Auto (on allowlisted domains) |
| Cron job creation | N/A (denied) | Requires Telegram approval |
| Background process | N/A (denied) | Requires Telegram approval |

### Risk Score

Soft Shell preset: **0.45** on the 0.0-0.9 scale.

| Level | Score | Description |
|---|---|---|
| Hard Shell | 0.0 | Cage — no tools |
| Split Shell | 0.18 | Arena — 11 tools, every action approved |
| **Soft Shell** | **0.45** | Safari — 17 tools, safeBins auto-approved |
| Soft Shell + browser | ~0.55 | Safari + browsing |
| Soft Shell + sessions | ~0.65 | Safari + multi-agent |

## Implementation Plan

### Step 1: Add soft preset to tool-manifest.yml
Define the `soft` preset with 17 enabled tools, extended safeBins, `on-miss` approval.

### Step 2: Create config/soft-shell.json5
Reference config file (like hard-shell.json5 and split-shell.json5).

### Step 3: Update tool-control-core.py
Add `soft` preset handling. Extend safeBins list. Handle `ask: "on-miss"`.

### Step 4: Update verify.sh
Add Soft Shell detection (profile=coding, exec.security=allowlist, exec.ask=on-miss). Set risk score expected range (0.35-0.55).

### Step 5: Test
- `make tools-dry-run PRESET=soft` — preview config
- `echo "y" | bash scripts/tool-control.sh --preset soft --apply` — apply and verify
- `make verify` — 24/24 with Soft Shell detected
- `make test` — all tests pass
- Attack surface probes — all pass (moat intact)

### Step 6: Document
Update CLAUDE.md, README.md, openclaw-reference.md shell level tables.

## Verification Plan

1. Apply Soft Shell → 24/24 verify PASS
2. Verify all NEVER-enabled tools are denied (gateway, bash, nodes)
3. Verify rm not in safeBins, no interpreters in safeBins
4. Verify proxy allowlist contains only expected domains
5. Verify risk score in 0.35-0.55 range
6. Verify config integrity hash matches
7. Run attack surface tests — all 21 probes pass
8. Round-trip: Hard → Split → Soft → Split → Hard — all verify at each step
9. Live test: message Hum, verify web_search works, cron works, safeBins auto-approve

## What Soft Shell Does NOT Include

- `browser` tool — denied by default, user can enable as per-capability toggle
- `sessions_spawn` — denied by default, user can enable for multi-agent workflows
- Host directory mounts — workspace only (no host ~/Documents etc.)
- ClawHub domains — permanently blocked
- Full exec (`security: "full"`) — never, always `"allowlist"`

These are Soft Shell + capability expansions on the sliding scale (0.55-0.7), not part of the base Soft Shell preset.

---

*The safari: vast territory, real autonomy, absolute moat. The lion is useful. The village is safe.*
