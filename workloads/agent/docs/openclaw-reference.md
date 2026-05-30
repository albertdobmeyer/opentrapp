# OpenClaw Reference — What We're Securing

**Updated:** 2026-03-27
**OpenClaw version:** 2026.2.26 (pinned in Containerfile)
**Source of truth:** Verified from source code analysis of dist/ bundles inside the container image. See `openclaw-internals.md` for evidence and code citations.

---

## What OpenClaw Is

OpenClaw is an open-source autonomous AI agent runtime. It runs on your machine, connects to LLM providers (Anthropic, OpenAI) via API, and can be controlled through messaging channels (Telegram, WhatsApp). It has tools (file access, shell commands, browser, web search), installable skills (from the ClawHub registry), and optional participation in the Moltbook agent social network.

**Default security posture: NONE.** Out of the box, OpenClaw runs with `tools.profile: "full"`, `sandbox.mode: "off"`, and no exec restrictions. The agent has the same access as the user who installed it. This is what the vault exists to change.

---

## 1. Complete Tool Inventory

These are all the tools OpenClaw can expose to the LLM. The LLM only sees tools that survive the tool policy pipeline — denied tools are removed before the API call.

| Tool | What It Does | Risk Level |
|---|---|---|
| `exec` | Run shell commands on the host | **Critical** — unrestricted code execution |
| `process` | Manage background processes | **Critical** — persistent execution |
| `read` | Read file contents | **High** — access to all user files |
| `write` | Create or overwrite files | **High** — can modify any file |
| `edit` | Modify existing files | **High** — can alter configs, code |
| `apply_patch` | Apply diffs to files | **High** — programmatic file modification |
| `grep` | Search file contents | **Medium** — information gathering |
| `find` | Search for files by name/path | **Medium** — filesystem enumeration |
| `ls` | List directory contents | **Medium** — filesystem enumeration |
| `browser` | Control Chromium (navigate, click, screenshot, fill forms) | **High** — web access, potential cookie theft |
| `web_search` | Search the web | **Medium** — information gathering |
| `web_fetch` | Fetch URL content | **Medium** — potential data exfiltration |
| `message` | Send messages on connected channels | **High** — can impersonate user |
| `canvas` | Agent-driven visual workspace | **Low** |
| `image` / `image_generate` | Analyze or generate images | **Medium** |
| `nodes` | iOS/Android device access (camera, location, contacts, SMS) | **Critical** — full phone access |
| `cron` | Schedule persistent jobs | **High** — survives restarts |
| `gateway` | Control the OpenClaw gateway itself | **High** — self-modification |
| `sessions_spawn` | Create sub-agents | **High** — autonomous delegation |
| `sessions_send` | Send messages between sessions | **Medium** — cross-session communication |

### Tool Groups

OpenClaw organizes tools into groups for bulk policy control:

| Group | Tools Included |
|---|---|
| `group:fs` | read, write, edit, apply_patch |
| `group:runtime` | exec, process |
| `group:automation` | cron, webhooks |
| `group:sessions` | sessions_spawn, sessions_send |
| `group:web` | web_search, web_fetch, browser |

---

## 2. Tool Profiles

Profiles define a base set of available tools. Additional tools can be added/removed via `tools.allow` and `tools.deny` lists.

| Profile | Tools Included (per official docs) | Notes |
|---|---|---|
| `minimal` | `session_status` only | Extremely restricted — no messaging, no file access, no exec |
| `coding` | `group:fs`, `group:runtime`, `group:sessions`, `group:memory`, `image` | Developer profile — filesystem, shell, sessions, memory |
| `messaging` | `group:messaging`, `sessions_list`, `sessions_history`, `sessions_send`, `session_status` | Communication-focused |
| `full` | No restriction (all tools) | **DEFAULT** — everything enabled |

**Source:** Official docs at `docs.openclaw.ai/gateway/configuration-reference`

**Critical discovery:** The `minimal` profile only includes `session_status`. Messaging works in Hard Shell not because of the profile, but because the Telegram channel integration operates independently of the tool profile. The `coding` profile includes `group:sessions` which we deny via `tools.deny` to prevent sub-agent spawning.

**Profile interaction with deny/allow:**
1. Profile sets the base tool set
2. `tools.deny` removes tools from that set (deny always wins)
3. `tools.allow` further restricts to only listed tools (if non-empty)
4. Result: the filtered list sent to the LLM

**Note:** `"standard"` is NOT a valid profile name — Zod schema validation rejects it. Valid profiles are: `minimal`, `coding`, `messaging`, `full`.

---

## 3. Configuration Reference

**Format:** JSON5 (comments and trailing commas allowed)
**Path:** `~/.openclaw/openclaw.json` inside the container
**Validation:** Zod schema at startup — unknown keys cause Gateway to refuse to start
**No `--config` flag:** Config is always at the above path. Use `--profile <name>` to isolate state under `~/.openclaw-<name>/`.

### All Valid Config Keys (verified)

```json5
{
  agents: {
    defaults: {
      model: {
        primary: "provider/model-name",   // e.g., "anthropic/claude-haiku-4-5"
      },
      sandbox: {
        mode: "off" | "non-main" | "all",
        // scope: "agent" | "session" | "shared",  // only relevant when mode != "off"
      },
    },
  },

  tools: {
    profile: "minimal" | "coding" | "messaging" | "full",
    deny: ["tool_name", "group:name", ...],   // deny always wins
    allow: ["tool_name", ...],                 // if non-empty, only these tools survive

    exec: {
      security: "deny" | "allowlist" | "full",
      ask: "always" | "on-miss" | "off",      // official values per docs.openclaw.ai
      askFallback: "deny" | "allowlist" | "full", // fallback when no approval UI reachable
      host: "auto" | "gateway" | "sandbox" | "node", // where commands execute
      safeBins: ["cat", "echo", ...],         // pre-approved binaries (stdin-only per official docs)
      safeBinProfiles: {                       // per-binary argument restrictions
        "bin-name": {
          minPositional: 0,                    // minimum required positional args
          maxPositional: 2,                    // maximum allowed positional args
          allowedValueFlags: ["--format"],     // flags explicitly allowed
          deniedFlags: ["--execute"],          // flags explicitly blocked
        },
      },
    },

    elevated: {
      enabled: false,                         // PERMANENTLY disabled in vault
      // allowFrom: [],                       // who can use elevated (irrelevant when disabled)
    },

    fs: {
      workspaceOnly: true,                    // restrict to ~/.openclaw/workspace/
    },
  },

  gateway: {
    mode: "local",                            // REQUIRED — Gateway refuses to start without this
    bind: "loopback" | "lan" | "tailnet" | "custom",
  },

  session: {
    dmScope: "per-channel-peer",              // session isolation
  },

  channels: {
    telegram: {
      dmPolicy: "pairing" | "allowlist" | "open" | "disabled",
      proxy: "http://vault-proxy:8080",       // route Telegram through vault proxy
    },
    whatsapp: {
      dmPolicy: "pairing",
    },
  },

  logging: {
    redactSensitive: "tools",                 // redact tool output in logs
  },
}
```

### Keys That DON'T Exist (Common Mistakes)

| Wrong Key | Correct Key | Source |
|---|---|---|
| `sandbox.mode` (top-level) | `agents.defaults.sandbox.mode` | phase1-findings.md |
| `exec.approvals.mode` | `tools.exec.security` | phase1-findings.md |
| `tools.elevated: []` | `tools.elevated.enabled: false` | phase1-findings.md |
| `memory.persistent` | Does not exist in schema | Removed from OpenClaw |
| `telemetry.enabled` | `logging.redactSensitive` | Different approach |
| `mdns.enabled` | Does not exist in schema | Removed from OpenClaw |
| `pairing.mode` | `channels.<provider>.dmPolicy` | phase1-findings.md |
| `tools.exec.strictInlineEval` | Exists — forces reapproval for inline eval (`python -c`, `node -e`) | Official docs confirm; earlier testing on v2026.2.17 incorrectly reported it missing |

---

## 4. Telegram Bot Mechanics

### How Pairing Works

1. OpenClaw starts with `channels.telegram.dmPolicy: "pairing"`
2. User sends any message to the bot on Telegram
3. Bot responds with a time-limited pairing code
4. User replies with the code to confirm
5. Bot pairs the user's Telegram ID to the agent session
6. Subsequent messages from that user are routed directly to the agent
7. Pairing persists in `~/.openclaw/agents/main/agent/` — survives restarts on persistent volume

### Message Routing

```
User sends Telegram message
    → grammY polling loop receives update from api.telegram.org
    → Session key: agent:main:telegram:direct:<user_id>
    → Message passed to agent processing pipeline
    → Tool list filtered by applyToolPolicyPipeline()
    → Filtered tools + message sent to LLM (Anthropic API)
    → LLM responds (text or tool_use)
    → If tool_use with ask:"always": approval request shown to user in Telegram
    → User taps Allow or Deny
    → If allowed: tool executes, result fed back to LLM
    → Final response sent back via Telegram
```

### Command Approval Flow

When `tools.exec.ask: "always"` is set:
1. LLM requests a tool execution (e.g., `exec: cat memory/notes.md`)
2. OpenClaw formats the command and sends it to the user via Telegram
3. User sees the exact command in the Telegram chat
4. User responds with "Allow" or "Deny" (via Telegram inline buttons or text)
5. If allowed: command executes inside the container (host: "gateway")
6. If denied: LLM receives a denial and may try a different approach
7. Approval is per-invocation — each command requires fresh approval

### Bot Token Location

The `TELEGRAM_BOT_TOKEN` is passed directly into the vault container via `compose.yml`. This is the one secret that does enter the container (unlike API keys which are proxy-injected). It's a bot token (not a user token), revocable via @BotFather at any time.

---

## 5. Session and Storage

### Directory Structure Inside Container

```
~/.openclaw/
├── openclaw.json                    # Main config (JSON5)
├── agents/
│   └── main/
│       ├── agent/
│       │   ├── auth-profiles.json   # API key profiles (placeholder in vault)
│       │   └── agent.json           # Agent identity and config
│       └── sessions/
│           └── *.jsonl              # Session transcripts (one per session)
├── workspace/                       # Agent working directory (workspaceOnly boundary)
│   ├── MEMORY.md                    # Long-term memory (if created by agent)
│   ├── memory/                      # Daily notes (if created by agent)
│   └── ...                          # Any files the agent creates
└── credentials/                     # Channel credentials (WhatsApp, etc.)
```

### Session Transcripts

Each session produces a `.jsonl` file with one JSON object per line:
- `type: "message"` — user or assistant messages (with role, content, timestamp)
- `type: "tool_use"` — tool invocations by the agent
- `type: "tool_result"` — results from tool execution
- `type: "system"` — heartbeats, session events

These transcripts are what `scripts/read-chat.sh` parses to show conversation history.

### Persistent vs Volatile Storage

| Path | Hard Shell | Split Shell | Notes |
|---|---|---|---|
| `~/.openclaw/` | tmpfs (volatile) | Volume (persistent) | Entire OpenClaw state directory |
| `/tmp` | tmpfs (volatile) | tmpfs (volatile) | Always volatile, noexec |
| `/home/vault/workspace` | tmpfs (volatile) | tmpfs (volatile) | Container workspace (NOT the agent workspace) |

In Split Shell mode, the persistent volume preserves:
- Config (`openclaw.json`)
- Auth profiles (Telegram pairing)
- Session transcripts
- Workspace files (memory, notes)
- Agent identity

---

## 6. What Each Shell Level Allows

### Hard Shell (`config/hard-shell.json5`)

| Setting | Value | Effect |
|---|---|---|
| Profile | `minimal` | `session_status` only (messaging works via Telegram channel, not tool profile) |
| Exec security | `deny` | All shell execution blocked |
| Deny list | group:runtime, group:automation, group:fs, group:sessions, exec, process, browser | Maximum lockdown |
| Storage | tmpfs (volatile) | Nothing persists across restarts |
| Domains | 3 (Anthropic, OpenAI, Telegram) | Minimal network access |

**What Hum can do:** Chat via Telegram. That's it.

### Split Shell (`config/split-shell.json5`)

| Setting | Value | Effect |
|---|---|---|
| Profile | `coding` | Includes exec, read, write, edit, apply_patch, grep, find, ls, process |
| Exec security | `allowlist` | Only safeBins-approved commands execute |
| Exec ask | `always` | Every command requires Telegram approval |
| Exec host | `gateway` | Commands run inside the container (not Docker sandbox) |
| safeBins | 16 commands | cat, echo, mkdir, cp, mv, touch, date, head, tail, wc, sort, uniq, tr, cut, jq, tee (rm removed — agent is constructive only) |
| Deny list | browser, web_search, web_fetch, group:automation, group:sessions, sessions_spawn, sessions_send, gateway, cron, canvas, nodes, process | No web, no cron, no sub-agents |
| Storage | Volume (persistent) | Memory and identity survive restarts |
| Domains | 3 (same as Hard Shell) | No additional network access |

**What Hum can do:** Chat, remember things, take notes, manipulate files in workspace — all with user approval for every command.

### Soft Shell (`config/soft-shell.json5`) — The Safari

| Setting | Value | Effect |
|---|---|---|
| Profile | `coding` | Same base profile as Split Shell |
| Exec security | `allowlist` | Only safeBins-approved commands execute |
| Exec ask | `on-miss` | SafeBins auto-approve, unknowns require Telegram approval |
| Exec host | `gateway` | Commands run inside the container |
| safeBins | 26 commands | All Split Shell safeBins + grep, sed, awk, diff, xargs, basename, dirname, env, test, printf |
| Enabled tools | 17 | All Split Shell tools + web_search, web_fetch, cron, process, canvas, message |
| Deny list | gateway, bash, browser, group:sessions, nodes | Protected resources — agent self-modification, sub-agents, and device access permanently blocked |
| Storage | Volume (persistent) | Memory and identity survive restarts |
| Domains | 4 (base + raw.githubusercontent.com) | Read public GitHub repos |

**What Hum can do:** Search the web, schedule tasks, process files, run text processing commands autonomously (safeBins auto-approve), send messages on authorized channels — all within the workspace boundary.

**What Hum still cannot do:** Browse websites (browser denied by default), spawn sub-agents (sessions denied by default), modify its own security configuration, delete files, run interpreters, or reach the protected resource set (root, SSH keys, credential stores, administrative accounts).

---

## 7. How the Tool Policy Pipeline Works

This is the most important security mechanism. Verified from source code (`reply-Deht_wOB.js` lines 64790-64820).

```
All tools in OpenClaw
    ↓
1. Profile filter: keep only tools in the selected profile
    ↓
2. Deny filter: remove any tool matching tools.deny patterns
    ↓
3. Allow filter: if tools.allow is non-empty, keep only matching tools
    ↓
4. Sandbox filter: if sandboxed, further restrict
    ↓
Result: filtered tool list sent to LLM
```

**The LLM never sees denied tools.** They are removed from the function definitions array before the API call. The agent cannot call a function it doesn't know exists. This is verified from source code — not documentation, not assumptions.

**Deny always wins.** A tool in the deny list is removed regardless of profile, allow list, or any other setting.

---

## 8. Network Architecture

```
OpenClaw process (inside vault container)
    → undici global dispatcher (ProxyAgent)
    → HTTP CONNECT to vault-proxy:8080
    → mitmproxy receives CONNECT, establishes TLS tunnel
    → vault-proxy.py addon intercepts decrypted request
    → Domain check against allowlist (BLOCKED → 403, or ALLOWED)
    → If Anthropic API: replaces x-api-key header with real key
    → Forwards to destination
    → Response flows back through same path
    → vault-proxy.py logs request/response as JSON
    → Response delivered to OpenClaw
```

**Environment variables checked for proxy:**
- `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` (and lowercase variants)
- Set in Containerfile and compose.yml
- The `--import /opt/proxy-bootstrap.mjs` flag ensures undici uses ProxyAgent globally

**Known issue (patched):** OpenClaw's `applyTelegramNetworkWorkarounds()` originally replaced the global ProxyAgent with a plain Agent, bypassing the proxy for Telegram calls. Fixed by `patches/fix-telegram-proxy.sh`. Root cause was `block_private=true` in mitmproxy config (now `false`). Patch provides defense-in-depth.

---

*For source code evidence behind these claims, see `openclaw-internals.md`. For compatibility test results, see `phase1-findings.md`.*
