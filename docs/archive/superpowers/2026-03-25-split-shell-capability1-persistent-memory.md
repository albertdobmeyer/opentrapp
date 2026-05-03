# Split Shell — Capability 1: Persistent Memory

**Date:** 2026-03-25
**Status:** Draft
**Shell transition:** Hard Shell → Split Shell (first capability)
**Prerequisite:** Hard Shell verified and working (Phase 2 complete)

---

## What This Enables

A user messages NewLobsterTrappBot on Telegram:
> "Remember that my dentist appointment is April 3rd at 2pm"

NewLobsterTrappBot writes this to `memory/2026-03-25.md` in its workspace. Tomorrow, when the user asks "When's my dentist appointment?", NewLobsterTrappBot reads its memory files and answers correctly — even if the container was restarted overnight.

Without this capability (Hard Shell): NewLobsterTrappBot has amnesia. Every restart is a blank slate. Pairing resets. Conversations vanish. This is the #1 reason NewLobsterTrappBot is useless in Hard Shell.

With this capability (Split Shell): NewLobsterTrappBot remembers things, maintains context across sessions, and the Telegram pairing survives restarts.

---

## What Changes

### 1. Tool Policy: Enable file operations in workspace

**File:** `config/split-shell.json5` (new config for Split Shell)

Change from Hard Shell:
```
// Hard Shell: group:fs is denied
tools: {
    deny: ["group:runtime", "group:automation", "group:fs", ...]
}

// Split Shell: group:fs removed from deny, workspaceOnly stays true
tools: {
    deny: ["group:runtime", "group:automation", "group:sessions",
           "sessions_spawn", "sessions_send", "gateway",
           "cron", "exec", "process", "browser"],
    fs: { workspaceOnly: true }
}
```

**What this unlocks:** `read`, `write`, `edit`, `apply_patch` — but ONLY within `/home/vault/.openclaw/workspace/`. The `workspaceOnly: true` flag is OpenClaw's built-in restriction that prevents file operations outside the workspace directory.

**What stays denied:** exec, process, browser, cron, sessions, gateway. The agent can read/write files but cannot run commands, browse the web, or schedule tasks.

### 2. Container: Persistent volume for ~/.openclaw/

**File:** `compose.yml`

Change from Hard Shell:
```yaml
# Hard Shell: tmpfs (volatile — wiped on restart)
tmpfs:
    - /home/vault/.openclaw:size=64m,noexec,nosuid,mode=1777

# Split Shell: named volume (persists across restarts)
volumes:
    - vault-data:/home/vault/.openclaw
```

Add to volumes section:
```yaml
volumes:
    vault-proxy-logs:
        driver: local
    proxy-ca:
        driver: local
    vault-data:            # NEW — persistent agent data
        driver: local
```

**What persists:**
- `workspace/memory/*.md` — daily notes and long-term memory
- `workspace/IDENTITY.md`, `USER.md` — agent and user identity
- `credentials/telegram-default-allowFrom.json` — approved Telegram users (no re-pairing!)
- `agents/main/sessions/*.jsonl` — conversation history
- `openclaw.json` — running config (includes auto-generated gateway token)

**What this means for the user:** Start the container, NewLobsterTrappBot remembers everything. No re-pairing. No amnesia.

### 3. Allowlist: No changes needed

Persistent memory is purely local file I/O within the container. No new domains required. The allowlist stays the same as Hard Shell:
- `api.anthropic.com`
- `api.openai.com`
- `api.telegram.org`

### 4. Entrypoint: Adapt for persistent volume

**File:** `scripts/entrypoint.sh`

The current entrypoint copies `openclaw-hardening.json5` to `~/.openclaw/openclaw.json` on every startup. With a persistent volume, this would overwrite any config changes OpenClaw made at runtime (like the auto-generated gateway auth token).

Change: Only copy the hardening config if `openclaw.json` doesn't already exist (first run). On subsequent starts, preserve the existing config.

```sh
if [ ! -f "$CONFIG_DST" ]; then
    # First run: install hardening config
    cp "$CONFIG_SRC" "$CONFIG_DST"
    echo "[vault] Hardening config installed (first run)"
else
    echo "[vault] Existing config preserved (persistent volume)"
fi
```

Same logic for auth-profiles.json — only create if it doesn't exist.

### 5. Shell switching script

**File:** `scripts/switch-shell.sh` (rename from `switch-gear.sh`)

```
Usage: bash scripts/switch-shell.sh <hard|split|soft>
```

When switching from Hard Shell to Split Shell:
1. Copy `config/split-shell.json5` to the persistent volume
2. Update the proxy allowlist (no changes for Capability 1)
3. Restart the container
4. Run Split Shell verification tests

When switching from Split Shell to Hard Shell:
1. Copy `config/hard-shell.json5` (current `openclaw-hardening.json5`)
2. Optionally wipe the persistent volume (`--clean` flag)
3. Restart the container

---

## What Does NOT Change

| Property | Still True in Split Shell? |
|----------|--------------------------|
| Exoskeleton (container hardening) | Yes — read-only root, caps dropped, seccomp, noexec, non-root |
| Proxy key injection | Yes — agent never sees real API key |
| Domain allowlist | Yes — same 3 domains |
| Exec denied | Yes — no shell commands |
| Browser denied | Yes — no web access |
| Cron denied | Yes — no scheduled tasks |
| Driver seat protected | Yes — no SSH keys, passwords, root, Docker socket |
| Kill switch | Yes — always available |
| 15-point verification | Yes — all checks still pass |

---

## Security Analysis

### What the agent CAN do with file access:

1. **Read its own workspace files** — AGENTS.md, SOUL.md, USER.md, memory/
2. **Write new files in workspace** — memory notes, identity updates
3. **Read session transcripts** — conversation history (already in the container)
4. **Modify its own personality** — SOUL.md, IDENTITY.md (OpenClaw's design allows this)

### What the agent CANNOT do:

1. **Read host files** — no host mounts, workspaceOnly enforced
2. **Read outside workspace** — `tools.fs.workspaceOnly: true` prevents it
3. **Execute files it writes** — noexec on all mounts (the exoskeleton)
4. **Reach the internet** — proxy allowlist unchanged
5. **Run commands** — exec still denied

### Risk assessment:

**Data in the persistent volume:** The volume contains conversation transcripts, memory files, and the Telegram pairing data. If someone gains access to the host filesystem, they could read these. But this is true of ANY local application — your browser cache, your chat apps, your email client all store data on disk.

**Agent self-modification:** The agent can modify SOUL.md and IDENTITY.md, changing its own personality. This is by OpenClaw's design (AGENTS.md tells it to). In Hard Shell this was prevented (no file writes). In Split Shell, we allow it because:
- The personality files only affect LLM behavior, not security controls
- The security config (`openclaw.json`) is managed by the entrypoint, not by the agent
- The tool policy, exec controls, and allowlist are NOT in the workspace

**Volume size:** The persistent volume should have a size limit to prevent the agent from filling the disk. Podman volume size limits can be set via the `--opt size=` flag.

### MANDATORY: Workspace Audit System

**No shell level beyond Hard Shell is permitted without complete workspace visibility.** This is a non-negotiable requirement — the user must be able to see every file the agent creates, modifies, or deletes, and every network request the agent makes. Zero blind spots.

#### Audit Command: `vault-audit.sh`

A new script (`scripts/vault-audit.sh`) that provides a complete picture of what NewLobsterTrappBot did. Runs from the host, does not require the container to be running (reads the persistent volume directly).

```
Usage: bash scripts/vault-audit.sh [options]
  --full          Full workspace listing with sizes and timestamps
  --changes       Files created or modified since last audit
  --diff <file>   Show content of a specific workspace file
  --memory        Show all memory files (memory/*.md + MEMORY.md)
  --sessions      Show session transcript summaries (message count, last active)
  --network       Parse proxy logs (domains contacted, requests blocked, payload sizes)
  --config        Show running OpenClaw config (verify shell level is correct)
  --all           Run all of the above
```

Output example for `--changes`:
```
=== Workspace Changes Since Last Audit (2026-03-25 08:00) ===

  NEW     memory/2026-03-25.md         (342 bytes)   13:45:02
  MODIFY  USER.md                      (891 bytes)   13:40:15
  MODIFY  IDENTITY.md                  (636 bytes)   13:40:10

  No files deleted.
  No files created outside workspace/.

  Total workspace size: 18.2 KB / 64 MB limit
```

#### Audit Triggers

The audit must be runnable:
1. **On demand** — user runs `vault-audit.sh` from terminal or GUI button
2. **On shell switch** — `switch-shell.sh` runs a full audit before AND after switching
3. **On container stop** — `kill.sh --soft` runs `--changes` and displays summary
4. **Periodically** — future: cron on the HOST (not inside the container) to audit daily

#### What the Audit Checks

| Check | What It Catches |
|-------|----------------|
| File listing with timestamps | New or modified files since last audit |
| File content hash tracking | Detects if a file was silently modified between audits |
| Workspace size | Prevents the agent from filling the disk |
| Files outside workspace boundary | Detects if workspaceOnly was bypassed (should never happen) |
| Proxy log summary | Domains contacted, requests blocked, unusual payload sizes |
| Config verification | Confirms the running config matches the expected shell level |
| Session transcript count | How many conversations, total size |
| Personality file changes | Detects if SOUL.md, IDENTITY.md, or AGENTS.md were modified |

#### Prompt Injection Detection

The agent could write **prompt injection** into its own memory files — text designed to manipulate its future self into behaving differently (e.g., "IGNORE ALL PREVIOUS INSTRUCTIONS" hidden in a memory note). This is an LLM-level risk, not a system-level risk, but the audit should flag it.

The audit script should scan workspace files for known injection patterns:
- "ignore previous instructions"
- "you are now"
- "system prompt override"
- "new instructions:"
- Base64-encoded blocks (potential hidden payloads)
- Unusually large files (could contain embedded data)

These are heuristic checks, not guarantees. But they catch obvious attempts.

#### Integration with Lobster-TrApp GUI (Future)

The audit output should render in the GUI dashboard as:
- A "Workspace" tab showing all files with preview
- A "Network" tab showing proxy log timeline
- A "Changes" badge showing count of files modified since last session
- A "Security" tab showing shell level, config verification, and any flags

#### component.yml Commands

```yaml
  - id: audit-workspace
    name: Workspace Audit
    description: Show all files the agent has created or modified
    group: monitoring
    type: query
    danger: safe
    command: bash scripts/vault-audit.sh --all
    output:
      format: text
      display: report
    available_when: [running-hard, running-split, running-soft, stopped]
    sort_order: 5
    timeout_seconds: 30

  - id: audit-changes
    name: Recent Changes
    description: Files changed since last audit
    group: monitoring
    type: query
    danger: safe
    command: bash scripts/vault-audit.sh --changes
    output:
      format: text
      display: log
    available_when: [running-hard, running-split, running-soft, stopped]
    sort_order: 6
    timeout_seconds: 15
```

### Residual risk: Exfiltration via LLM API

The agent could encode workspace file contents (including conversation transcripts and memory) into LLM API requests. This was already a residual risk in Hard Shell (conversation content goes to the API). Split Shell doesn't materially change this — the same conversations that were sent to the API as context are now also saved to disk.

**Mitigation:** The 1 MB payload limit on the proxy prevents bulk exfiltration. Individual API calls contain conversation context by design.

---

## Verification Plan

### Split Shell verification tests (new, beyond the 15-point check):

1. **File write works in workspace:** Agent can create `memory/test.md`
2. **File read works in workspace:** Agent can read `SOUL.md`
3. **workspaceOnly enforced:** Agent cannot read `/etc/passwd` or `/home/vault/.openclaw/openclaw.json`
4. **Persistence verified:** Write a file, restart container, verify file still exists
5. **Pairing survives restart:** Approve Telegram pairing, restart, verify user is still approved
6. **Exec still denied:** Agent cannot run `ls`, `cat`, or any shell command
7. **Browser still denied:** Agent cannot browse websites
8. **Cron still denied:** Agent cannot schedule tasks
9. **Proxy allowlist unchanged:** Only 3 domains allowed, all others blocked
10. **Protected resources still blocked:** No SSH keys, no Docker socket, no host mounts

### End-to-end test:

1. Switch to Split Shell: `bash scripts/switch-shell.sh split`
2. Message NewLobsterTrappBot: "Remember that my dentist is Dr. Smith, appointment April 3rd at 2pm"
3. Verify: NewLobsterTrappBot writes to `memory/2026-03-25.md`
4. Restart: `bash scripts/kill.sh --soft && podman-compose up -d`
5. Message NewLobsterTrappBot: "When is my dentist appointment?"
6. Verify: NewLobsterTrappBot reads from memory and answers correctly
7. Verify: No re-pairing needed (Telegram user still approved)

---

## Files To Create/Modify

| Action | File | What Changes |
|--------|------|-------------|
| Create | `config/split-shell.json5` | Split Shell config (fs enabled, exec/browser/cron denied) |
| Create | `config/hard-shell.json5` | Rename current `openclaw-hardening.json5` for clarity |
| Modify | `compose.yml` | Add `vault-data` persistent volume option |
| Create | `compose.split-shell.yml` | Override file for Split Shell volume mounts |
| Modify | `scripts/entrypoint.sh` | Conditional config copy (first run only) |
| Create | `scripts/switch-shell.sh` | Shell switching mechanism |
| Create | `scripts/vault-audit.sh` | **MANDATORY** — workspace audit, network audit, config verification, injection detection |
| Modify | `scripts/verify.sh` | Add Split Shell verification tests (checks 16-25) |
| Modify | `component.yml` | Add shell states, switch commands, and audit commands |
| Create | `docs/split-shell-test-results.md` | Test documentation |

---

## Open Questions

1. **Volume size limit:** What's a reasonable size for the persistent volume? 256 MB? 1 GB? Depends on how much conversation history accumulates.

2. **workspaceOnly bypass:** Is `tools.fs.workspaceOnly: true` enforced at the OpenClaw application level (software, could have bugs) or at the container level (hardware, trustworthy)? We should verify in the source code before relying on it as a security boundary.

3. **Config conflict:** When the entrypoint preserves existing config (persistent volume), and we want to change security settings (e.g., shell down from Split to Hard), how do we force-overwrite? The `switch-shell.sh` script should handle this, but the entrypoint needs to not undo it on restart.

4. **Agent modifying security config:** Can the agent write to `~/.openclaw/openclaw.json` (the security config) since it's inside `~/.openclaw/`? If `workspaceOnly` restricts to `~/.openclaw/workspace/` (not all of `~/.openclaw/`), we're fine. Must verify.
