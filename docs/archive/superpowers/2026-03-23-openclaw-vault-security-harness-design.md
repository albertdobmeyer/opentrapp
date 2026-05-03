# OpenClaw-Vault Security Harness — Design Specification

**Date:** 2026-03-23
**Status:** Draft — Pending Roadmap & Implementation Planning
**Authors:** albertd + Claude
**Revision:** 2 (post-review, all critical/major/minor findings resolved)

---

## Terminology

These terms are used precisely throughout this document:

- **Gear**: One of three vault access levels (Gear 1: Manual, Gear 2: Semi-Auto, Gear 3: Full-Auto). This is a vault concept.
- **Mode**: An OpenClaw-native configuration setting (sandbox mode, exec security mode, approval mode). This is an OpenClaw concept.
- **Driver seat**: The user-facing metaphor for protected resources — the things the agent can never access regardless of gear.
- **Protected resources**: The technical term for driver seat items — root, SSH keys, passwords, keyrings, etc.
- **Layer**: One of the six defense layers (Layers 1-2: infrastructure-enforced; Layers 3-6: OpenClaw-configuration-enforced).

---

## 1. What OpenClaw Is

OpenClaw is a self-hosted Node.js gateway that connects 30+ messaging platforms (Telegram, WhatsApp, Signal, Discord, iMessage, Slack, etc.) to an AI agent that can execute real tasks on a user's computer. It is not a chatbot. It is a local autonomous AI agent with shell access, filesystem access, browser control, messaging capabilities, device control, and the ability to run any program the host user can run.

### 1.1 Architecture

OpenClaw runs as a single Gateway process that binds to `ws://127.0.0.1:18789`. All channels, CLI, web interfaces, and device nodes connect via WebSocket to this Gateway. The Gateway coordinates sessions, tools, events, and device access.

```
Channels (WhatsApp / Telegram / Slack / Discord / Signal / iMessage / 30+ more)
    |
Gateway (control plane, ws://127.0.0.1:18789)
    |-- Pi agent (RPC — the AI reasoning engine)
    |-- CLI (openclaw commands)
    |-- WebChat UI
    |-- macOS app
    |-- iOS / Android nodes
```

The Gateway is the single source of truth for sessions, routing, and channel connections. It runs on the user's machine and has access to everything the user has access to.

### 1.2 Installation

```bash
npm install -g openclaw@latest
openclaw onboard --install-daemon
```

OpenClaw's official docs state Node 22+ is required (Node 24 recommended). Works on macOS, Linux, and Windows (via WSL2). The onboarding wizard guides setup of the Gateway, workspace, channels, and skills.

**NOTE:** The current vault Containerfile uses Node 20-alpine (pinned to `node:20-alpine@sha256:...`). This is a version mismatch that must be resolved during implementation — either upgrade the base image to Node 22+, or verify that the pinned OpenClaw version (`@anthropic-ai/openclaw@2026.2.17`) actually works on Node 20. See Open Question 4.

### 1.3 LLM Integration

OpenClaw connects to external LLM providers for reasoning. It supports multiple providers with model selection and failover:

- OpenAI (ChatGPT/Codex via OAuth)
- Anthropic (Claude via API key)
- Google (Gemini), Groq, Ollama, and 40+ other providers

The main configuration file is `~/.openclaw/openclaw.json` (JSON format). Example:
```json
{
  "agent": { "model": "anthropic/claude-opus-4-6" }
}
```

**NOTE on config format:** OpenClaw's primary config is JSON (`openclaw.json`). Our current vault's hardening config (`config/openclaw-hardening.yml`) uses YAML format and is copied into the container as `config.yml` via the Containerfile. During the redesign, we must determine whether to:
1. Convert our YAML configs to JSON to match OpenClaw's native format, or
2. Verify that OpenClaw accepts YAML config files via the `--config` flag, or
3. Generate JSON from our YAML templates at container build time.

The config key paths referenced in OpenClaw's official docs (e.g., `agents.defaults.sandbox.mode`, `tools.allow`) may differ from the flat YAML structure in our current hardening config (`sandbox.mode`, `exec.approvals.mode`). Alignment must be verified. See Open Question 8.

The LLM provides the intelligence. OpenClaw provides the tools and access. This separation is critical: the security risk is not the LLM itself, but the tools OpenClaw gives the LLM access to.

### 1.4 Communication Channels

Users interact with OpenClaw through messaging apps, not a terminal. Supported channels include:

- WhatsApp (Baileys), Telegram (grammY), Slack (Bolt), Discord (discord.js)
- Google Chat, Signal (signal-cli), BlueBubbles (iMessage)
- Microsoft Teams, Matrix, IRC, LINE, Mattermost, Nostr
- Nextcloud Talk, Synology Chat, Twitch, Zalo, WebChat

This is what makes OpenClaw accessible to non-technical users: you message it from your phone and it does things on your computer.

---

## 2. What OpenClaw Can Do On A User's Computer

This is the threat surface our vault must contain.

### 2.1 Complete Tool Inventory

| Tool | What It Does | System Access Required | Risk Level |
|------|-------------|----------------------|------------|
| `exec` / `process` | Run any shell command, manage background processes | Full shell / command execution | **Critical** — unrestricted code execution equivalent to user account |
| `read` / `write` / `edit` / `apply_patch` | Read, create, modify, delete any file | Full filesystem access | **Critical** — can access all user files, configs, credentials |
| `browser` | Control a Chromium browser (navigate, click, screenshot, fill forms) | Browser control, display, network | **High** — can access websites, fill forms, potentially steal session cookies |
| `web_search` / `web_fetch` | Search the web, fetch page content | Unrestricted internet access | **Medium** — information gathering, potential data exfiltration |
| `message` + channel tools | Send messages across all connected channels as the user | Communication channels (WhatsApp, Telegram, etc.) | **High** — can impersonate the user in conversations |
| `canvas` | Agent-driven visual workspace | Display / presentation | **Low-Medium** |
| `nodes` (iOS/Android) | Camera snap/clip, screen recording, location, contacts, calendar, SMS, photos | Device hardware access | **Critical** — full phone access |
| `cron` / `gateway` | Schedule persistent jobs, restart gateway | System scheduling, gateway control | **High** — persistent execution that survives restarts |
| `image` / `image_generate` | Analyze or generate images | Image processing, potentially network | **Medium** |
| `sessions_spawn` / `sessions_send` | Create sub-agents, delegate tasks | Process management, agent coordination | **High** — can create autonomous sub-agents |
| 1Password skill | Access password vault | Credential storage | **Critical** — "Once authorized, it has access to your entire vault" |

### 2.2 Tool Groups

OpenClaw organizes tools into groups for policy configuration:

- `group:fs` — File system operations (read, write, edit, apply_patch)
- `group:runtime` — Interpreter access (exec, process)
- `group:automation` — Persistent scheduling (cron, webhooks)
- `group:sessions` — Sub-agent management (sessions_spawn, sessions_send)
- `group:web` — Internet access (web_search, web_fetch, browser)

### 2.3 The Default Security Posture

**Sandboxing is OFF by default.** Out of the box, OpenClaw:

- Can execute any shell command with no restrictions
- Has no command allowlist
- Has no approval requirements
- Has the same access as the user who installed it
- Uses tool profile `full` (all tools enabled)

As one security guide puts it: "the AI agent has roughly the same level of access that you have on your machine."

### 2.4 Workspace and Configuration Files

OpenClaw stores its state at `~/.openclaw/`:

```
~/.openclaw/
|-- openclaw.json              # Main configuration (contains tokens/auth)
|-- credentials/               # Channel credentials (WhatsApp, etc.)
|   |-- whatsapp/
|   |-- oauth.json
|-- agents/<agentId>/agent/
|   |-- auth-profiles.json     # API keys
|   |-- agent.json             # Agent config
|-- secrets.json               # Optional encrypted payload
|-- sessions/
|   |-- *.jsonl                # Session transcripts with user data
|-- workspace/                 # Agent working directory
|-- exec-approvals.json        # Approved command patterns
```

### 2.5 Additional Threat Vectors

Beyond direct tool access, these threats must be considered:

**Prompt injection via messaging channels:** In Gear 2 (Semi-Auto) and Gear 3 (Full-Auto), the agent receives messages from external channels (WhatsApp, Telegram, etc.). A malicious message from a third party could manipulate the agent into performing harmful actions. Mitigation is primarily OpenClaw's responsibility (use strong models, strict DM policies), but the vault's monitoring layer must flag unusual behavior patterns.

**Credential leakage through session transcripts:** Session transcripts (`sessions/*.jsonl`) may contain sensitive information the user sends via messaging channels (e.g., sharing a password with the agent via Telegram). These transcripts reside in the container workspace and could theoretically be encoded into LLM API requests, exfiltrating them through allowed domains.

**Supply-chain attacks via ClawHub skills:** 11.9% of ClawHub skills were malware during the ClawHavoc incident. ClawHub registry domains (`clawdhub.com`, `www.clawhub.ai`) must be blocked in ALL gears by default. This is a non-negotiable rule, like the protected resources. If a user needs a specific skill, they must download and review it manually, then transfer it into the container.

---

## 3. OpenClaw's Built-In Security Model

OpenClaw has a three-layer permission system. It is opt-in and designed for technical users who understand what they're configuring. Our vault's job is to configure these layers correctly and add enforcement layers the agent cannot modify.

### 3.1 Sandbox Mode (Where Tools Run)

Controls the execution environment. Set via `agents.defaults.sandbox.mode`:

| Mode | Behavior |
|------|----------|
| `off` | Tools run directly on the host machine (DEFAULT) |
| `non-main` | Only group/channel sessions are sandboxed; main sessions bypass |
| `all` | Every session runs in a containerized sandbox |

Sandbox scope options:
- `agent` — separate container per agent (recommended)
- `session` — separate container per session (stricter)
- `shared` — single container for all agents (risky)

Workspace access within sandbox:
- `none` — no agent workspace access (default when sandboxed)
- `ro` — read-only mount at `/agent`
- `rw` — read-write mount at `/workspace`

Dangerous Docker settings that must NEVER be enabled in the vault:
- `dangerouslyAllowReservedContainerTargets` — privileged container targeting
- `dangerouslyAllowExternalBindSources` — mounting external host volumes
- `dangerouslyAllowContainerNamespaceJoin` — namespace sharing

### 3.2 Tool Policy (What Tools Are Available)

Determines which tools the agent can use. Four built-in profiles:

| Profile | Tools Available |
|---------|----------------|
| `minimal` | Messaging + read-only tools |
| `messaging` | + channel tools (WhatsApp, Telegram, etc.) |
| `standard` | + file read + browser + web tools |
| `full` | All tools enabled (DEFAULT) |

Additional controls:
- `tools.allow` — explicit allowlist (everything else blocked)
- `tools.deny` — explicit denylist (always wins over allow)
- `tools.fs.workspaceOnly` — restricts file operations to workspace directory
- Per-agent overrides via `agents.list[].tools.allow/deny`

**Key rule: "deny always wins."** If a tool is in the deny list, it cannot be used regardless of the allow list or profile.

### 3.3 Exec Security (How Commands Run)

Controls shell command execution specifically:

| Security Level | Behavior |
|---------------|----------|
| `deny` | All gateway/node execution blocked (most restrictive) |
| `allowlist` | Only pre-approved commands work; unknown commands trigger approval |
| `full` | All commands permitted (dangerous) |

Additional exec controls:
- `tools.exec.ask: "always"` — requires approval on each invocation
- `tools.exec.strictInlineEval: true` — forces reapproval for inline code eval
- `tools.exec.safeBins` — trusted binaries that bypass allowlist (cat, grep, ls, etc.)
- `tools.exec.safeBinProfiles` — per-binary restriction profiles
- Interpreters (Python, Node, Ruby, Bash) must NEVER be in safeBins

### 3.4 Elevated Access (The Escape Hatch)

Per-session override that lets exec bypass sandbox to run on the host:

- `/elevated on` — run exec on host (may still need approval)
- `/elevated full` — run exec on host AND skip approval
- Requires `tools.elevated.enabled: true` in config
- Can be restricted to specific users via `tools.elevated.allowFrom`

**This must be disabled in the vault. Period.**

### 3.5 Gateway Authentication

Controls who can connect to the OpenClaw gateway:

- `token` — shared bearer credential (recommended)
- `password` — credential via environment variable
- `trusted-proxy` — identity-aware reverse proxy

Gateway binding:
- `loopback` — localhost only (default, recommended)
- `lan` — local network (risky)
- `tailnet` — Tailscale network
- `custom` — explicit IP

### 3.6 DM Access Control

Controls who can message the agent:

| Policy | Behavior |
|--------|----------|
| `pairing` | Unknown senders get time-limited codes; must be approved (DEFAULT) |
| `allowlist` | Unknown senders blocked entirely |
| `open` | Anyone can message (HIGH RISK) |
| `disabled` | Ignore inbound DMs |

---

## 4. The Identified Problems

### 4.1 The Core Problem

OpenClaw is an extremely powerful agent with full user-level access to a computer, controlled via messaging apps that non-technical users already know how to use. Its default configuration has no sandboxing, no tool restrictions, and no command approval. This means:

1. **A non-technical user** who installs OpenClaw and connects it to Telegram has given an AI agent unrestricted access to their entire computer, their files, their messaging apps, their browser, and potentially their phone.

2. **The built-in security model** (sandbox + tool policy + exec controls) exists but is designed for technical users who understand Linux containers, shell security, and permission models. A non-technical user will never configure these correctly.

3. **The ecosystem itself is hostile.** 11.9% of ClawHub skills were malware. The database was breached. 21,639 instances were exposed on the public internet without authentication. CVE-2026-25253 allowed one-click RCE. This is not a theoretical risk.

### 4.2 What Non-Technical Users Don't Know

A non-technical user who installs OpenClaw doesn't know:

- That the agent can read all their files (photos, documents, tax returns, passwords)
- That the agent can send messages as them on WhatsApp, Signal, Telegram
- That the agent can run any command on their computer
- That the agent can schedule persistent background jobs
- That the agent's API key, if stolen, can run up their bill
- That skills downloaded from ClawHub might be malware
- That the agent's session transcripts contain their personal data
- How to configure sandboxing, tool policies, or exec controls
- How to audit what the agent did after a session

### 4.3 What Existing Hardening Guides Get Wrong

Every existing OpenClaw hardening guide:

1. **Assumes technical users** — "if you can't understand how to run a command line, this is far too dangerous for you"
2. **Puts the API key inside the container** — a compromised process reads it from `/proc/self/environ`
3. **Requires manual configuration** — editing JSON config files, understanding Docker networking, setting up seccomp profiles
4. **Provides no monitoring** — the user has no way to see what the agent did in plain language
5. **Is all-or-nothing** — either fully locked down (unusable) or fully open (dangerous)

### 4.4 What Our Current Vault Gets Right

The existing openclaw-vault implementation (as of 2026-03-23) correctly solves:

- **API key isolation** — proxy-side injection means the key never enters the container
- **Network isolation** — internal-only network with proxy allowlist enforcement
- **Container hardening** — read-only root, all caps dropped, custom seccomp (see `config/vault-seccomp.json` and `config/vault-proxy-seccomp.json`), noexec tmpfs, PID/memory limits, non-root user, `tini` as PID 1 for proper signal forwarding and zombie reaping (ensures kill switches work correctly)
- **Kill switches** — three escalation levels (soft/hard/nuclear) via `scripts/kill.sh`
- **Verification** — 15-point security check (`scripts/verify.sh`) that proves all controls are active
- **Honest documentation** — five residual risks are explicitly listed
- **Proxy CA trust chain** — `scripts/entrypoint.sh` waits for the proxy CA certificate before starting OpenClaw, ensuring the container cannot bypass the proxy on startup

### 4.5 What Our Current Vault Gets Wrong

The existing vault is a **single-mode static sandbox** for security researchers. It fails the non-technical user thesis because:

1. **No granular control** — it's fully locked or nothing. No way to say "allow messaging but not filesystem"
2. **No gear switching** — can't adjust the agent's access level without rebuilding the container
3. **No user-friendly interface** — requires terminal commands, editing config files, understanding Docker
4. **No monitoring in plain language** — proxy logs are JSON lines, not human-readable summaries
5. **Claims to be "not an agentic workstation"** — the README explicitly says the vault prevents the features that make OpenClaw useful (email, files, browser, messaging)
6. **Target audience is wrong** — says "not for you if you've never used a terminal"
7. **Monitoring scripts are stubs** — network-log-parser.py, session-report.sh, skill-scanner.sh print "not yet implemented" (labeled as "Phase 3" in the code)
8. **One broken test** — `tests/test-network-isolation.sh` uses `wget` which was stripped from the image
9. **Proxy container name mismatch** — compose.yml names it `vault-proxy`, but `component.yml` references `openclaw-proxy` in the proxy-logs command. This means the proxy-logs button in the GUI silently fails.
10. **Node version mismatch** — Containerfile uses Node 20-alpine but OpenClaw docs state Node 22+ is required (see section 1.2 NOTE)

### 4.6 Pre-Existing Bugs To Fix

These bugs exist in the current codebase independent of the redesign and should be fixed first:

| Bug | File | Fix |
|-----|------|-----|
| `test-network-isolation.sh` uses `wget` (stripped from image) | `tests/test-network-isolation.sh` | Replace `wget` with Node.js HTTP client or `curl` equivalent |
| Proxy container name mismatch | `component.yml` line 160 | Change `openclaw-proxy` to `vault-proxy` |
| Node.js version mismatch | `Containerfile` | Verify or upgrade to Node 22+ |
| `anthropic-version` header hardcoded to `2023-06-01` | `proxy/vault-proxy.py` line 163 | Make configurable or update; document as a maintenance point |

---

## 5. Our Solution: The Security Harness

### 5.1 The Thesis

**Any non-technical user can safely run OpenClaw on their personal computer, maintaining full control over what the agent can and cannot access, without needing technical knowledge or a second machine.**

### 5.2 The Metaphor

The user's computer is a car. OpenClaw is an AI driver being installed. The OpenClaw-Vault is the security harness that:

- **Keeps the user in the driver seat** — the user always has ultimate control (root, admin, passwords, system resources are never accessible to the agent)
- **Provides a gear selector** — the user can switch between:
  - **Gear 1 — Manual (stick-shift)**: The agent can do nothing without per-action approval. Maximum safety, minimum autonomy.
  - **Gear 2 — Semi-Auto**: The agent has access to specific tools the user has granted (e.g., messaging yes, filesystem no). Moderate safety, moderate autonomy.
  - **Gear 3 — Full-Auto**: The agent has broad autonomy — EXCEPT it can never touch the driver seat (root, admin, passwords, system identity). Lower safety, maximum autonomy.
- **Never lets the AI driver take the steering wheel** — regardless of gear, the user can always:
  - See what the agent is doing (monitoring)
  - Stop the agent immediately (kill switch)
  - Change the agent's access level (gear shifting)
  - Review what the agent did (audit trail)

### 5.3 The Six-Layer Defense Architecture

Our vault wraps OpenClaw in six layers of defense. Even if one layer fails, the others hold. The outer layers (1-2) are enforced at the infrastructure level and cannot be modified by the agent. The inner layers (3-6) configure OpenClaw's own security model at the application level.

```
Layer 1: Container Isolation (kernel-level wall)
  |
  |  The agent runs inside a hardened Podman/Docker container with:
  |  - Read-only root filesystem
  |  - All Linux capabilities dropped
  |  - Custom seccomp profiles (deny-by-default; see config/vault-seccomp.json,
  |    config/vault-proxy-seccomp.json)
  |  - no-new-privileges flag
  |  - Non-root user (uid 1000)
  |  - tini as PID 1 (proper signal forwarding, ensures kill switches work)
  |  - PID limit (256), memory limit (4GB), CPU limit (2 cores)
  |  - noexec on all tmpfs mounts
  |  - No Docker/Podman socket mounted (agent cannot spawn containers)
  |  - No host volume mounts (by default — Gear 2/3 may grant specific mounts)
  |  - entrypoint.sh waits for proxy CA cert before starting OpenClaw
  |
Layer 2: Network Proxy (domain allowlist + API key isolation)
  |
  |  All network traffic routes through a mitmproxy sidecar that:
  |  - Blocks all domains not on the allowlist (returns 403)
  |  - Blocks raw IP addresses (allowlist is domain-only)
  |  - Injects API keys at the network layer (key never enters agent container)
  |  - Injects anthropic-version header (currently hardcoded to 2023-06-01;
  |    must be made configurable — see section 4.6)
  |  - Blocks outbound payloads > 1 MB (exfiltration prevention; threshold may
  |    need per-gear configuration for Gear 3 — see Open Question 9)
  |  - Blocks inbound responses > 10 MB
  |  - Redacts API keys if reflected in responses
  |  - Logs every request/response as structured JSON to
  |    /var/log/vault-proxy/requests.jsonl
  |  - Supports hot-reload of allowlist via SIGHUP (no container restart needed)
  |
Layer 3: OpenClaw Tool Policy (what tools are available — application-level)
  |
  |  Configured via OpenClaw config inside the container:
  |  - Gear 1 (Manual): profile "minimal", deny all exec/fs/automation
  |  - Gear 2 (Semi-Auto): per-capability allow/deny (see section 5.4)
  |  - Gear 3 (Full-Auto): profile "full" with deny list for critical tools
  |  - In ALL gears: deny sessions_spawn, gateway (agent cannot create
  |    sub-agents or modify its own gateway)
  |  - In ALL gears: ClawHub domains blocked in proxy allowlist
  |
Layer 4: OpenClaw Application-Level Restrictions
  |
  |  IMPORTANT: OpenClaw's sandbox mode ("all", "non-main", etc.) works by
  |  spawning Docker/Podman containers for each session. Since our vault
  |  container has no Docker socket (Layer 1), OpenClaw's sandbox container
  |  spawning WILL NOT WORK inside the vault. This is intentional — our
  |  container IS the sandbox.
  |
  |  However, Layer 4 still provides value as application-level restrictions
  |  that OpenClaw enforces in its own runtime:
  |  - sandbox.mode: "non-main" (current default; prevents the agent from
  |    running as the primary/main instance)
  |  - tools.fs.workspaceOnly: true (restricts file ops to workspace directory)
  |  - elevated access: DISABLED permanently (tools.elevated.enabled: false)
  |  - ALL dangerous Docker settings: disabled permanently
  |  - workspaceAccess controlled per gear (none / ro / rw — this controls
  |    what the agent can see within its own container filesystem)
  |
  |  This layer is defense-in-depth: even if Layer 1 were somehow bypassed,
  |  OpenClaw's own application-level restrictions would still limit the agent.
  |  If Layer 1 holds (which it should), Layer 4's container-spawning features
  |  are redundant by design.
  |
Layer 5: OpenClaw Exec Controls (how commands run — application-level)
  |
  |  Configured per gear:
  |  - Gear 1 (Manual): security "deny" (all exec blocked)
  |  - Gear 2 (Semi-Auto): security "deny" by default; if a specific
  |    capability requires exec (e.g., scheduling), it is enabled with
  |    security "allowlist", ask "always", strictInlineEval: true
  |  - Gear 3 (Full-Auto): security "allowlist" with curated safeBins list
  |  - In ALL gears: elevated access disabled, no interpreter in safeBins
  |
Layer 6: Hardening Config (agent behavior lockdown — application-level)
  |
  |  Applied to OpenClaw's own configuration:
  |  - Gear 1 (Manual): approval mode "always" (every action needs approval)
  |  - Gear 2 (Semi-Auto): approval mode configurable per capability
  |  - Gear 3 (Full-Auto): approval mode for destructive actions only
  |  - persistence: controlled per gear (false for Gear 1, configurable for 2/3)
  |  - telemetry: disabled in all gears
  |  - mDNS: disabled in all gears (prevents LAN scanning)
  |  - pairing: allowlist mode (no unauthorized agent-to-agent communication)
  |  - memory: controlled per gear (non-persistent for Gear 1)
  |  - DM policy per gear:
  |    - Gear 1: "pairing" (strictest — each sender must be approved)
  |    - Gear 2: "pairing" (default) or "allowlist" (user configures)
  |    - Gear 3: "allowlist" (only pre-approved senders)
```

### 5.4 The Three Gears — Detailed

#### Gear 1: Manual (Stick-Shift)

*"The agent can think, but it cannot act without your explicit approval for every single action."*

| Layer | Configuration |
|-------|--------------|
| Layer 1: Container | Fully isolated. No host mounts. No network except LLM APIs. |
| Layer 2: Network proxy | Allowlist: LLM API providers only (api.anthropic.com, api.openai.com). `raw.githubusercontent.com` removed (not needed in this gear). |
| Layer 3: Tool policy | Profile: `minimal`. Deny: `group:runtime`, `group:automation`, `group:fs`, `sessions_spawn`, `gateway` |
| Layer 4: App restrictions | sandbox.mode: `non-main`, workspaceAccess: `none`, elevated: disabled |
| Layer 5: Exec | security: `deny` (all shell commands blocked) |
| Layer 6: Hardening | Approval mode: `always`. Persistence: false. DM policy: `pairing`. |

**Use case:** First-time setup, security evaluation, learning what OpenClaw does.

#### Gear 2: Semi-Auto

*"The agent can use specific tools you've granted, within boundaries you've set."*

The user selects which capabilities to enable via the Lobster-TrApp GUI. Each capability maps to a set of tool policy changes, network allowlist additions, and optionally container mount configurations.

| Capability | What It Enables | What It Requires |
|-----------|----------------|-----------------|
| Messaging (Telegram) | `message`, Telegram channel tools | Telegram credentials in persistent volume, Telegram API domains on allowlist |
| Messaging (WhatsApp) | `message`, WhatsApp channel tools | WhatsApp credentials in persistent volume, WhatsApp domains on allowlist |
| Web browsing (sandboxed) | `browser`, `web_search`, `web_fetch` | Broader domain allowlist or user-configurable domains |
| File workspace (sandboxed) | `read`, `write`, `edit` with workspaceOnly: true | Container tmpfs workspace only, user-initiated file transfer in/out via GUI |
| File access (specific folders) | `read`, `write`, `edit` on mounted paths | Specific host directory mounted read-only or read-write into container (requires container restart) |
| Scheduling | `cron` | Persistent container (survives restarts), exec enabled with allowlist |

| Layer | Configuration |
|-------|--------------|
| Layer 1: Container | Selective host mounts (user-chosen directories, read-only by default). Persistent credential volume for messaging channels. |
| Layer 2: Network proxy | Allowlist: LLM APIs + user-selected service domains |
| Layer 3: Tool policy | Explicit allow list for granted capabilities only. Deny: `sessions_spawn`, `gateway`. Each capability adds specific tools to the allow list; everything else stays denied. |
| Layer 4: App restrictions | sandbox.mode: `non-main`, workspaceAccess: per-capability (`none`, `ro`, or `rw`), elevated: disabled |
| Layer 5: Exec | security: `deny` by default. If a capability requires exec (e.g., scheduling), enable with security: `allowlist`, ask: `always`, strictInlineEval: true. |
| Layer 6: Hardening | Approval mode: configurable per capability. Persistence: configurable. DM policy: `pairing` (default) or `allowlist`. |

**Use case:** Everyday AI assistant use — messaging, web research, file management in specific folders, scheduling.

**Note on exec in Gear 2:** The tool policy deny list and exec security settings are complementary, not contradictory. Tool policy controls which tools exist in the agent's toolkit. Exec security controls how the `exec` tool behaves IF it is in the toolkit. In Gear 2, `exec` starts denied in tool policy. Enabling a capability that needs exec (like scheduling) adds `exec` to the allow list AND sets exec security to `allowlist` with `ask: always`. If no capability requires exec, it stays denied at both layers.

#### Gear 3: Full-Auto

*"The agent can operate broadly, but it can never touch the driver seat."*

| Layer | Configuration |
|-------|--------------|
| Layer 1: Container | Broader host mounts (user home minus protected resources). Most domains allowed. |
| Layer 2: Network proxy | Allowlist: broad (but still blocks ClawHub domains, known malicious domains, and internal/private network ranges via mitmproxy's `block_private=true`) |
| Layer 3: Tool policy | Profile: `full`. Deny: `gateway`, `sessions_spawn` (cannot modify itself or create sub-agents) |
| Layer 4: App restrictions | sandbox.mode: `non-main`, workspaceAccess: `rw`, elevated: disabled |
| Layer 5: Exec | security: `allowlist`, safeBins: curated list (cat, grep, ls, etc. — NO interpreters), strictInlineEval: true |
| Layer 6: Hardening | Approval mode: for destructive actions only. Persistence: configurable. DM policy: `allowlist`. |

#### Protected Resources (The Driver Seat) — NEVER Accessible In Any Gear

| Protected Resource | How It's Protected |
|-------------------|-------------------|
| Root / sudo access | Container runs as non-root (uid 1000), sudo stripped from image, no-new-privileges, capabilities dropped |
| System admin (systemd, services) | Seccomp blocks mount, unshare, setns; no host systemd socket mounted |
| User passwords / keyring | `~/.local/share/keyrings` never mounted; GNOME keyring socket not mapped |
| SSH keys | `~/.ssh` never mounted |
| GPG keys | `~/.gnupg` never mounted |
| Browser saved passwords | Real browser profile never used; vault browser is a fresh Chromium profile |
| API keys / tokens | Proxy-side injection; `~/.openclaw/credentials` controlled by vault, not agent |
| Other user accounts | Container user namespace isolation; only uid 1000 mapped |
| Docker / Podman socket | Never mounted (also prevents agent from spawning containers) |
| /etc, /boot, /sys, /proc (host) | Never mounted; container has its own isolated /proc |
| The vault itself | Vault config files are read-only mounts; agent cannot modify its own restrictions |
| ClawHub registry | Domains (`clawdhub.com`, `www.clawhub.ai`) blocked in all gears; never added to allowlist |

### 5.5 Gear Switching

The user switches gears via the Lobster-TrApp GUI. This requires:

1. **Stopping the current session** — active agent session is saved or terminated
2. **Reconfiguring Layers 3-6** — OpenClaw config, allowlist, hardening config updated
3. **Optionally reconfiguring Layers 1-2** — container mounts and network may change (requires container restart)
4. **Restarting the container** — new configuration takes effect
5. **Running verification** — gear-specific security checks confirm new configuration is correct

Switching from a more permissive gear to a less permissive gear (e.g., Gear 3 to Gear 1) always works immediately. Switching to a more permissive gear requires explicit user confirmation via the GUI.

**The user never edits JSON files, YAML configs, or Docker compose files.** The GUI handles all of this.

**TOCTOU safety:** The container must be fully stopped (verified via container runtime status check) before reconfiguration begins. Configuration changes are applied atomically — all config files are written to a staging area, then swapped into place before the container restarts. At no point does the container run with a partially applied configuration.

### 5.6 Monitoring and Audit Trail

Every gear provides the user with visibility into what the agent is doing:

1. **Real-time activity feed** — structured log of agent actions rendered in the GUI as human-readable entries. Implementation approach: parse proxy JSONL logs + OpenClaw session JSONL transcripts, correlate by timestamp, and render as categorized entries (network request, tool use, file access, message sent, command blocked). This is rule-based parsing, not LLM-generated summaries.
2. **Network traffic summary** — which domains were contacted, how much data was sent/received, how many requests were blocked
3. **Tool usage report** — which tools the agent used, what files it accessed, what commands it ran
4. **Session transcript** — full conversation history between user and agent
5. **Security alerts** — flagged when the agent attempts something blocked (domain, tool, command)
6. **Session summary** — when the session ends, a structured summary of actions taken, resources accessed, and any security events

This replaces the current stub monitoring scripts (network-log-parser.py, session-report.sh, skill-scanner.sh — currently labeled "Phase 3" in the code) with real implementations that render in the Lobster-TrApp GUI.

### 5.7 The Kill Switch

Available in every gear, always accessible, cannot be disabled:

| Level | Action | When to Use | Implementation |
|-------|--------|-------------|----------------|
| **Soft stop** | Stop agent session, preserve workspace for review | "Stop, I want to check what you did" | `scripts/kill.sh --soft` (exists) |
| **Hard kill** | Remove containers, volumes, networks | "I don't trust this session, destroy everything" | `scripts/kill.sh --hard` (exists) |
| **Nuclear** | Hard kill + purge all container artifacts + rotate API key reminder | "Something went wrong, clean slate" | `scripts/kill.sh --nuclear` (exists) |

**Future enhancement (not in initial implementation):** A "Pause" level that suspends the agent session while keeping the container running ("Wait, what are you doing?"). This would require either `SIGSTOP` to the OpenClaw process inside the container, or an API call to the OpenClaw Gateway to suspend the session. Implementation will be investigated during Gear 2/3 development when the Gateway API is better understood.

### 5.8 Lobster-TrApp Integration (The Hard Constraint)

The Lobster-TrApp Tauri backend must contain **zero component-specific knowledge**. The gear system, capability toggles, and monitoring must be expressed through the manifest contract (`component.yml`), not through vault-specific Tauri code.

Implementation approach:

1. **Gears as commands:** Each gear maps to a `component.yml` command (e.g., `switch-gear-manual`, `switch-gear-semi`, `switch-gear-full`). These are shell scripts in the vault repo that reconfigure Layers 3-6 and restart the container.
2. **Capabilities as config:** Capability toggles (messaging, web browsing, file access, etc.) are exposed as editable config files in `component.yml`. The GUI's existing config editor renders them; the vault's gear-switching scripts read them.
3. **Monitoring as commands:** Activity feed, session summary, and security alerts are `component.yml` commands with appropriate output formats (`display: log`, `display: report`, etc.). The vault's monitoring scripts parse logs and output human-readable text; the GUI renders it through existing output renderers.
4. **Kill switch as commands:** Already expressed as `component.yml` commands (soft-stop, hard-kill, nuclear-kill).

This may require extending the `component.yml` schema to support:
- Command grouping by gear (e.g., `available_when: [running-manual]` vs `available_when: [running-semi]`)
- State definitions per gear (`running-manual`, `running-semi`, `running-full` in addition to `running`, `stopped`, etc.)

This is a cross-cutting concern that affects both the vault's `component.yml` and the Lobster-TrApp schema. It must be tracked as a dependency.

---

## 6. What Changes From The Current Vault

### 6.1 What We Keep

Everything in the current vault that works:

- Two-container architecture (vault + proxy sidecar)
- Proxy-side API key injection
- Domain allowlist enforcement with SIGHUP hot-reload
- Container hardening (read-only root, caps dropped, seccomp, noexec, PID/mem limits, tini PID 1)
- Kill switch (soft/hard/nuclear — three existing levels)
- Verification suite (currently 15 checks; will be extended per-gear)
- Structured JSON logging (proxy/requests.jsonl)
- Exfiltration detection (payload size limits, key redaction)
- Entrypoint CA cert wait (ensures proxy trust chain)

### 6.2 What We Add

| Addition | Purpose |
|----------|---------|
| **Gear system** (Gear 1 / Gear 2 / Gear 3) | Dynamically adjustable access levels |
| **OpenClaw configuration profiles** | Pre-built configs per gear + per-capability |
| **Gear-switching shell scripts** | Safe reconfiguration exposed as `component.yml` commands |
| **Selective host mounts** | Allow specific directories into container for Gear 2/3 |
| **Protected resources enforcement** | Explicit exclusion list of resources never accessible in any gear |
| **Monitoring implementations** | Real network-log-parser, session-report, activity-feed (replacing stubs) |
| **Audit trail renderer** | human-readable activity feed via `component.yml` commands |
| **Gear-specific verification** | Security checks beyond the current 15, specific to each gear |
| **Persistent credential volume** | For messaging channel credentials (Telegram, WhatsApp) |
| **Gear-specific compose templates** | Different compose.yml variations per gear (different mounts, resources) |

### 6.3 What We Change

| Current State | New State |
|--------------|-----------|
| README says "not an agentic workstation" | README says "a secure agentic workstation with granular control" |
| README says "not for you if you've never used a terminal" | README says "designed for everyone, including non-technical users" |
| Single static configuration | Three gear profiles + per-capability toggles |
| Terminal-only operation | GUI-driven via Lobster-TrApp |
| Hardcoded allowlist (3 domains) | Per-gear allowlist templates with user customization |
| Stub monitoring scripts | Real monitoring with GUI rendering |
| One test suite | Gear-specific test suites |
| Proxy container referenced as `openclaw-proxy` in component.yml | Fixed to `vault-proxy` (matches compose.yml) |
| Node 20 base image | Verified or upgraded to Node 22+ (see Open Question 4) |
| Hardcoded `anthropic-version` header | Made configurable |

### 6.4 What We Remove

| Removal | Reason | Cleanup Required |
|---------|--------|-----------------|
| "Path B: Docker Desktop Sandbox Plugin" | Confusing; weaker security; contradicts thesis | Delete `scripts/docker-sandbox-setup.sh`, remove README section, remove any component.yml references |
| Phase 2 VM isolation stubs | Out of scope; can be added later as a future enhancement | Move `phase2-vm-isolation/` to a clearly-labeled `future/` directory or delete |
| The disclaimer "This tool is not for you" | Our whole thesis is that it IS for everyone | Rewrite README sections 4.5 items 5 and 6 |

---

## 7. Open Questions For Roadmap Planning

These questions need to be answered during implementation planning:

1. **Gear switching: restart or hot-reload?** — Can we reconfigure OpenClaw's tool policy via the Gateway WebSocket API (`ws://127.0.0.1:18789`) without restarting the container? The Gateway is the single control plane for sessions and tools, so runtime reconfiguration may be possible for Layers 3-6. Layer 1-2 changes (mounts, allowlist) can be partially hot-reloaded (SIGHUP for proxy) but mount changes require a restart. This needs investigation.

2. **Host mount granularity** — What is the right default set of mountable directories for Gear 2 and Gear 3? How do we present this to a non-technical user? (e.g., "Documents folder", "Downloads folder" vs raw paths)

3. **Channel credential management** — Telegram (grammY) and WhatsApp (Baileys) require persistent session state (tokens, QR code pairing data). This almost certainly requires a persistent volume that survives container restarts. This volume needs the same security treatment as the proxy-ca volume: controlled externally, not writable by the agent's OpenClaw process in a way that compromises the credentials.

4. **OpenClaw version pinning & Node.js version** — The Containerfile pins `@anthropic-ai/openclaw@2026.2.17` on Node 20-alpine. OpenClaw docs say Node 22+ is required. Must verify: (a) does the pinned version work on Node 20? (b) what is our update strategy? Auto-update is a security risk; never updating leaves known vulnerabilities. Suggest: version bump is a manual, reviewed process with changelog audit.

5. **Testing the thesis** — We plan to install OpenClaw on our own laptop using this repo. Minimum viable test plan: (a) Gear 1: verify all 15 checks pass, verify agent cannot exec/read files/reach non-API domains. (b) Gear 2: enable messaging capability, verify agent can message via Telegram but cannot exec/read host files. (c) Gear 3: verify broad access works, then verify protected resources are inaccessible (attempt to read ~/.ssh, attempt sudo, attempt to reach ClawHub domains).

6. **Scope of monitoring** — Two log sources: proxy JSONL (network traffic) and OpenClaw session JSONL (tool invocations, conversation). Both must be parsed. The monitoring scripts should correlate by timestamp to produce a unified activity feed. Start with proxy logs (already structured); add session log parsing once we understand the format from testing.

7. **Cross-platform support** — The current setup.sh supports Linux and macOS. setup.ps1 supports Windows. Gear-specific compose templates may have platform-specific considerations (e.g., WSL2 path translation for host mounts on Windows, Docker Desktop vs Podman rootless on Linux).

8. **Config format alignment** — Must verify whether OpenClaw's `--config` flag accepts YAML, JSON, or both. Must map our current YAML config keys (e.g., `sandbox.mode`, `exec.approvals.mode`) to OpenClaw's official JSON config key paths (e.g., `agents.defaults.sandbox.mode`, `tools.exec.security`). This mapping is critical for generating correct per-gear configs.

9. **Exfiltration threshold per gear** — The 1 MB outbound payload limit in `vault-proxy.py` may be too restrictive for Gear 3, where legitimate use cases (uploading files, sending large code contexts to LLMs) could exceed it. Should the threshold be configurable per gear? If so, what are safe defaults? (e.g., Gear 1: 1 MB, Gear 2: 5 MB, Gear 3: 25 MB)

10. **Agent self-modification prevention** — In Gear 3 with workspace `rw` access, can the agent write to its own config files? The OpenClaw config directory inside the container must be mounted read-only or placed outside the writable workspace. Verify that no gear allows the agent to modify files that Layer 4-6 depend on.

---

## 8. References

### Normative References (defines behavior the spec relies on)
- [OpenClaw GitHub README](https://github.com/openclaw/openclaw/blob/main/README.md)
- [OpenClaw Official Docs](https://docs.openclaw.ai/)
- [OpenClaw Security Documentation](https://docs.openclaw.ai/gateway/security)
- [OpenClaw Tools & Plugins](https://docs.openclaw.ai/tools)
- [OpenClaw Exec Tool](https://docs.openclaw.ai/tools/exec)
- [Permissions, Sandbox & Security — OpenClaw Help](https://www.getopenclaw.ai/en/help/permissions-sandbox-security)
- [Sandbox vs Tool Policy vs Elevated — OpenClaw Docs](https://open-claw.bot/docs/gateway/sandbox-vs-tool-policy-vs-elevated/)

### Informational References (context, not authoritative)
- [Security-First OpenClaw Setup — Roberto Capodieci](https://capodieci.medium.com/ai-agents-016-security-first-openclaw-setup-sandboxing-dm-pairing-and-what-not-to-share-fb0003f685b4)
- [OpenClaw Security Architecture and Hardening — Nebius](https://nebius.com/blog/posts/openclaw-security)
- [What is OpenClaw — DigitalOcean](https://www.digitalocean.com/resources/articles/what-is-openclaw)
- [OpenClaw Explained — KDnuggets](https://www.kdnuggets.com/openclaw-explained-the-free-ai-agent-tool-going-viral-already-in-2026)
- [OpenClaw Complete Guide — Milvus Blog](https://milvus.io/blog/openclaw-formerly-clawdbot-moltbot-explained-a-complete-guide-to-the-autonomous-ai-agent.md)
- [Don't Run OpenClaw on Your Main Machine — SkyPilot Blog](https://blog.skypilot.co/openclaw-on-skypilot/)
