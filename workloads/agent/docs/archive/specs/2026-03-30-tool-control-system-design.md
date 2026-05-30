# Spec: Tool Control System — Per-Tool Whitelisting/Blacklisting

**Date:** 2026-03-30
**Phase:** 4 (was "Soft Shell Design", reframed as "Tool Control System")
**Security implications:** This system controls which tools the OpenClaw agent can use. Every tool enabled is a potential injection attack vector.

---

## Purpose

Build a granular tool control system that lets the user whitelist and blacklist every single tool in OpenClaw's portfolio. Shell levels (Hard/Split/Soft) become presets on a sliding zero-trust scale from 0.0 to 0.9, not the only options.

The user must always:
- Know exactly which tools are enabled
- Be able to enable/disable any individual tool
- See the security implications of each change
- Have the change verified before it takes effect

## Architecture

### How It Works

```
User chooses tools (via GUI or CLI)
    → tool-control.sh generates openclaw.json
    → Config includes: profile, deny list, allow list, exec settings, safeBins
    → Per-tool allowlist changes applied to proxy/allowlist.txt
    → Container restarted (or hot-reloaded for Layer 3-6 changes)
    → verify.sh confirms new config matches expectations
```

The tool control script is the single point where tool policy is defined. It reads a **tool manifest** (what tools exist, their risk levels, their requirements) and a **user selection** (what the user wants enabled), then generates the correct OpenClaw config.

### Tool Manifest

A static file that documents every tool, its risk level, what it requires, and what enabling it means for each defense layer. This is the source of truth.

Based on our `openclaw-reference.md` tool inventory and the original design spec (lines 483-505):

```yaml
# config/tool-manifest.yml — what tools exist and what they cost to enable
tools:
  # --- File Operations ---
  read:
    risk: high
    group: fs
    description: Read file contents
    injection_vectors:
      - "Agent reads sensitive files outside workspace"
      - "Path traversal if workspaceOnly fails"
    requires:
      profile: coding
    layers:
      proxy: []  # no domain changes needed
      exec: false  # not an exec operation

  write:
    risk: high
    group: fs
    description: Create or overwrite files
    injection_vectors:
      - "Agent writes malicious content to workspace"
      - "Overwrite agent config if workspaceOnly fails"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  edit:
    risk: high
    group: fs
    description: Modify existing files
    injection_vectors:
      - "Agent modifies files to inject content"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  apply_patch:
    risk: high
    group: fs
    description: Apply diffs to files
    injection_vectors:
      - "Programmatic file modification at scale"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  grep:
    risk: medium
    group: none
    description: Search file contents
    injection_vectors:
      - "Information gathering from workspace files"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  find:
    risk: medium
    group: none
    description: Search for files by name/path
    injection_vectors:
      - "Filesystem enumeration"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  ls:
    risk: medium
    group: none
    description: List directory contents
    injection_vectors:
      - "Filesystem enumeration"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  # --- Execution ---
  exec:
    risk: critical
    group: runtime
    description: Run shell commands
    injection_vectors:
      - "Arbitrary code execution"
      - "Command injection via crafted arguments"
      - "Pipeline chaining to bypass safeBins"
    requires:
      profile: coding
      exec_security: allowlist
      exec_ask: always
      safeBins: true
    layers:
      proxy: []
      exec: true

  process:
    risk: critical
    group: runtime
    description: Manage background processes
    injection_vectors:
      - "Persistent execution that outlives the session"
      - "Resource exhaustion via fork"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  # --- Web ---
  browser:
    risk: high
    group: ui
    description: Control Chromium browser
    injection_vectors:
      - "Access to web content with potential cookie theft"
      - "Navigation to malicious sites"
      - "Form filling with user credentials"
    requires:
      profile: coding
    layers:
      proxy: ["user-configured domains"]
      exec: false

  web_search:
    risk: medium
    group: web
    description: Search the web
    injection_vectors:
      - "Information gathering"
      - "Search queries that leak context"
    requires:
      profile: coding
    layers:
      proxy: ["search API domains"]
      exec: false

  web_fetch:
    risk: medium
    group: web
    description: Fetch URL content
    injection_vectors:
      - "Data exfiltration via URL parameters"
      - "SSRF if internal URLs reachable"
    requires:
      profile: coding
    layers:
      proxy: ["fetched domains must be on allowlist"]
      exec: false

  # --- Communication ---
  message:
    risk: high
    group: messaging
    description: Send messages on connected channels
    injection_vectors:
      - "Impersonation — agent sends messages as user"
      - "Social engineering via automated messaging"
    requires:
      profile: messaging
    layers:
      proxy: ["channel API domains"]
      exec: false

  # --- Automation ---
  cron:
    risk: high
    group: automation
    description: Schedule persistent jobs
    injection_vectors:
      - "Persistent execution that survives restarts"
      - "Delayed payload delivery"
    requires:
      profile: coding
      exec_security: allowlist
      persistent_container: true
    layers:
      proxy: []
      exec: true

  gateway:
    risk: high
    group: automation
    description: Control the OpenClaw gateway
    injection_vectors:
      - "Self-modification — agent changes its own config"
      - "Disable security controls from inside"
    requires: never  # NEVER enabled — agent cannot modify itself
    layers: {}

  # --- Sessions ---
  sessions_spawn:
    risk: high
    group: sessions
    description: Create sub-agents
    injection_vectors:
      - "Autonomous delegation to unmonitored sub-agents"
      - "Resource exhaustion via agent spawning"
    requires: never  # NEVER enabled in Split Shell
    layers: {}

  sessions_send:
    risk: medium
    group: sessions
    description: Send messages between sessions
    injection_vectors:
      - "Cross-session data leakage"
    requires: never  # NEVER enabled in Split Shell
    layers: {}

  # --- Device ---
  nodes:
    risk: critical
    group: nodes
    description: iOS/Android device access
    injection_vectors:
      - "Full phone access — camera, contacts, SMS, location"
    requires: never  # NEVER enabled — no device access
    layers: {}

  # --- Memory ---
  memory_search:
    risk: low
    group: memory
    description: Search agent memory
    injection_vectors:
      - "Memory poisoning via crafted search results"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  memory_get:
    risk: low
    group: memory
    description: Retrieve memory entries
    injection_vectors:
      - "Access to persisted sensitive information"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  # --- Visual ---
  canvas:
    risk: low
    group: ui
    description: Agent visual workspace
    injection_vectors:
      - "Low — display only"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false

  image:
    risk: medium
    group: none
    description: Analyze images
    injection_vectors:
      - "Steganographic data in images"
    requires:
      profile: coding
    layers:
      proxy: []
      exec: false
```

### Shell Presets

Shell levels are pre-defined tool selections — convenience shortcuts on the sliding scale.

**Hard Shell (0.0):** `session_status` only. Profile: `minimal`. Everything denied.

**Split Shell — Current (0.2):** Profile: `coding`. Enabled: `read`, `write`, `edit`, `apply_patch`, `exec` (with safeBins + approval), `grep`, `find`, `ls`, `memory_search`, `memory_get`, `image`. Denied: everything else.

**Split Shell — Full (0.4):** All of current Split Shell plus per-capability toggles for: `web_search`, `web_fetch`, `browser` (sandboxed), `cron`, `process`. Each requires its own allowlist and layer changes.

**Soft Shell (0.7-0.9):** Profile: `full` minus driver seat. Broad exec with curated safeBins. Broad domain allowlist. Approval only for destructive actions.

### The Air-Gap Principle: Constructive vs Destructive

The vault air-gaps the OpenClaw agent from the user's computer. The division:

**Agent side (constructive only):** read, write, create, search, analyze. The agent can build and propose, but never destroy.

**User side (destructive + admin):** delete, modify system, change permissions, manage tool access, review audits. Claude Code and the OpenTrApp GUI handle these from outside the container.

If Hum needs something deleted, it asks the user via Telegram. The user or Claude handles it from the host.

### NEVER Enabled (In Any Shell Level)

These tools/operations are permanently denied, regardless of user choice:
- `gateway` — agent cannot modify itself or its own security config
- `sessions_spawn` — no autonomous sub-agents (Split Shell and below)
- `nodes` — no device access
- `rm` — agent cannot delete files (destructive operation — user-side only)
- Root/system modification — read-only root filesystem, no caps, no sudo
- `chmod` / `chown` — agent cannot change file permissions (never in safeBins)

### Protected Resources (Driver Seat)

These are NEVER accessible, enforced by the container exoskeleton (Layer 1):
- Root / sudo access
- SSH keys (`~/.ssh`)
- GPG keys (`~/.gnupg`)
- Passwords / keyrings
- Docker/Podman socket
- Host /etc, /boot, /sys, /proc
- The vault's own config
- ClawHub registry domains

## Implementation Plan

### Step 1: Tool Manifest File
Create `config/tool-manifest.yml` documenting every tool with risk levels, injection vectors, and layer requirements. This is the single source of truth for what tools exist and what enabling them costs.

### Step 2: Config Generator Script
Create `scripts/tool-control.sh` that:
- Reads the tool manifest
- Accepts user tool selections (CLI args or config file)
- Generates the correct `openclaw.json` with proper profile, deny/allow lists, exec settings
- Updates `proxy/allowlist.txt` if needed
- Validates the generated config before applying
- Runs verification after applying

### Step 3: Per-Capability Expansion (One at a Time)
Add each new capability individually with its own test:
1. Web search (`web_search`) — needs proxy allowlist for search API
2. Web fetch (`web_fetch`) — needs proxy allowlist per-domain
3. Browser — needs Chromium in container, broader allowlist
4. Scheduling (`cron`) — needs persistent container, exec enabled
5. Background processes (`process`) — needs PID limit awareness

Each capability gets:
- A spec documenting its injection vectors
- An addition to the tool manifest
- A test verifying it works correctly AND the rest stays locked
- A test verifying it can be disabled

### Step 4: Verification Expansion
Update `verify.sh` to validate the current tool selection:
- Check that enabled tools match what the user chose
- Check that disabled tools are actually denied
- Check that the proxy allowlist matches the tool selection
- Report the current position on the 0.0-0.9 scale

## Security Implications

- Every tool enabled expands the attack surface
- The tool manifest documents injection vectors per tool — the user should see these
- The config generator is a security-critical component — it must be tested thoroughly
- Defense-in-depth: even if the config generator has a bug, the proxy allowlist and container exoskeleton still enforce boundaries independently
- The sliding scale is informational — it helps the user understand their risk posture, not replace the per-tool control

## Verification Plan

1. Generate configs for each shell preset and verify they match expected tool selections
2. Test each capability individually: enable it, verify it works, disable it, verify it's blocked
3. Test that protected resources are inaccessible at every point on the scale
4. Test that the NEVER-enabled tools cannot be enabled regardless of user input
5. Run full `make test` and `make verify` after every change

---

*This spec replaces "Phase 4: Soft Shell Design" in the roadmap. The tool control system is the infrastructure that enables both Split Shell completion and eventual Soft Shell implementation.*
