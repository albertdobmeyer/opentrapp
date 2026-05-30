# OpenClaw Internals — Source Code Analysis

**Date:** 2026-03-24
**OpenClaw version:** 2026.2.26 (current pinned version — see `Containerfile` line 19)
**Analysis method:** Reading dist/ bundle files inside the container image
**Purpose:** We cannot secure what we don't understand. This document captures verified knowledge about OpenClaw's architecture from the actual source code — not documentation, not blog posts, not assumptions.

*Note: This analysis supersedes the Phase 1 findings (`phase1-findings.md`), which tested against `openclaw@2026.2.17`. The upgrade to 2026.2.26 added Telegram proxy support. The Containerfile is the single source of truth for the pinned version.*

---

## 1. Message Processing Pipeline

When a user sends a Telegram message, this is the actual execution path:

```
User sends "Hello" via Telegram
    ↓
grammY polling loop receives update from api.telegram.org
    (File: extensions/telegram/src/channel.ts)
    ↓
Message routed to agent session
    Session key: agent:main:telegram:direct:<user_id>
    ↓
Tool list filtered by applyToolPolicyPipeline()
    (File: reply-Deht_wOB.js:65102)
    ↓
    Pipeline steps (in order):
    1. Global tools.deny list → removes denied tools
    2. Global tools.allow list → if non-empty, keeps only listed tools
    3. Profile expansion → "minimal" adds messaging + read-only tools
    4. Sandbox tool policy → if sandboxed, further restricts
    ↓
Filtered tool list sent to LLM via Anthropic API
    - API key from auth-profiles.json (placeholder in our case)
    - Anthropic SDK adds Authorization header
    - Request goes through undici → HTTP_PROXY → vault-proxy → internet
    (File: @mariozechner/pi-ai/dist/providers/anthropic.js:540-600)
    ↓
LLM responds (text or tool_use)
    ↓
If tool_use: approval check (per exec-approvals config)
    (File: pi-embedded-CQnl8oWA.js:19789-19900)
    ↓
Response sent back to Telegram via grammY
    → through undici → HTTP_PROXY → vault-proxy → api.telegram.org
```

### Critical Security Property

**The LLM never sees denied tools.** The tool list is filtered BEFORE being sent to the LLM. This is the strongest possible enforcement — the agent cannot call a function it doesn't know exists. The filtering happens at `applyToolPolicyPipeline()` which removes tools from the function definitions array.

Source evidence: `reply-Deht_wOB.js` lines 64790-64820:
```javascript
function filterToolsByPolicy(tools, policy) {
    const matcher = makeToolPolicyMatcher(policy);
    return tools.filter(t => matcher(t.name));
}
```

And the matcher (`pi-tools.policy-C8K-rNTV.js` lines 225-244):
```javascript
function makeToolPolicyMatcher(policy) {
    const deny = compileGlobPatterns({ raw: expandToolGroups(policy.deny ?? []) });
    const allow = compileGlobPatterns({ raw: expandToolGroups(policy.allow ?? []) });
    return (name) => {
        const normalized = normalizeToolName(name);
        if (matchesAnyGlobPattern(normalized, deny)) return false;   // DENY FIRST
        if (allow.length === 0) return true;
        if (matchesAnyGlobPattern(normalized, allow)) return true;
        return false;
    };
}
```

---

## 2. Sandbox Mode — What It Actually Does

**File:** `sandbox-DY8nmmZL.js`

The `shouldSandboxSession()` function (line 2004-2010):
```javascript
if (cfg.mode === "off") return false;
if (cfg.mode === "all") return true;
return sessionKey.trim() !== mainSessionKey.trim();  // "non-main" mode
```

| Mode | Behavior | Docker Required? |
|------|----------|-----------------|
| `"off"` | No sandboxing. Tools run directly in the OpenClaw process. | No |
| `"non-main"` | Only non-main sessions spawn Docker containers. Main session runs directly. | Yes (for non-main) |
| `"all"` | Every session spawns a Docker container. | Yes |

**Why `"off"` is correct for the vault:**
- The vault container IS the sandbox (Layer 1: read-only, caps dropped, seccomp, noexec)
- Docker is not available inside the vault (no socket, by design)
- `"non-main"` causes "spawn docker ENOENT" for non-main sessions
- `"off"` avoids the Docker dependency while tool policy enforcement still applies

**Sandbox does NOT affect tool policy.** Even with `mode="off"`, the `applyToolPolicyPipeline()` still filters tools based on deny/allow lists. Sandbox mode controls WHERE tools execute (Docker vs process), not WHETHER they execute.

---

## 3. Authentication — API Key Flow

**File:** `auth-profiles-6WJHPoy1.js`

### How OpenClaw loads API keys:

1. **Profile store loaded** from `~/.openclaw/agents/<agentId>/agent/auth-profiles.json`
2. **Profile lookup** via `resolveApiKeyForProfile()` (line ~14200):
   - Checks `store.profiles[profileId]`
   - Validates `type === "api_key"`
   - Resolves secret via `resolveProfileSecretString()` (supports env, file, or exec sources)
3. **Key passed to Anthropic SDK** via `new Anthropic({ apiKey })` (anthropic.js line ~540)
4. **SDK adds header** `x-api-key: <the key>` to the HTTP request automatically

### Correct auth-profiles.json format:
```json
{
  "profiles": {
    "anthropic:api": {
      "provider": "anthropic",
      "type": "api_key",
      "key": "<the key value>"
    }
  },
  "order": {
    "anthropic": ["anthropic:api"]
  }
}
```

**Fields that caused confusion:**
- `type` (NOT `mode`) — must be `"api_key"`
- `key` (NOT `apiKey`) — the actual key value or secret reference
- `order` (NOT `default`) — specifies profile resolution order per provider

### Our placeholder approach:
We put a dummy key (`sk-ant-api03-placeholder-vault-proxy-will-inject-real-key-placeholder`) in auth-profiles.json. OpenClaw's SDK includes it in the `x-api-key` header. Our vault-proxy.py intercepts the request and REPLACES it with the real key from its own environment. The agent only ever sees the placeholder.

---

## 4. Network Layer — How Requests Are Routed

### HTTP Client: undici

OpenClaw uses Node.js's built-in `undici` library for all HTTP requests.

**Global proxy setup** (`send-DslMV0Oj.js` line ~730):
```javascript
const proxyUrl = process.env.HTTPS_PROXY || process.env.HTTP_PROXY;
setGlobalDispatcher(proxyUrl
    ? new ProxyAgent({ uri: proxyUrl, connect: { autoSelectFamily: ... } })
    : new Agent({ connect: { autoSelectFamily: ... } })
);
```

**Environment variables checked** (`fetch-guard-BhYFHZ2H.js`):
```javascript
const ENV_PROXY_KEYS = [
    "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY",
    "http_proxy", "https_proxy", "all_proxy"
];
```

### The `applyTelegramNetworkWorkarounds` issue

`send-DslMV0Oj.js` calls `applyTelegramNetworkWorkarounds()` which sets a new global dispatcher. In the unpatched code, this overwrites any ProxyAgent with a plain Agent. Our patch (in `patches/fix-telegram-proxy.sh`) makes it check for proxy env vars and use ProxyAgent when they're set.

**However:** The root cause of ALL proxy failures was `block_private=true` in mitmproxy, which killed connections FROM private IPs (the vault container). With `block_private=false`, the standard proxy path may work. The patch provides defense-in-depth.

### Request flow through the vault:

```
OpenClaw process → undici global dispatcher (ProxyAgent)
    → HTTP CONNECT to vault-proxy:8080
    → mitmproxy receives CONNECT, establishes TLS tunnel
    → vault-proxy.py addon intercepts decrypted request
    → Checks domain against allowlist (BLOCKED or ALLOWED)
    → If Anthropic: replaces x-api-key header with real key
    → Forwards to api.anthropic.com
    → Response flows back through the same path
    → vault-proxy.py logs the request/response as JSON
```

---

## 5. Exec Tool — Execution and Guards

**File:** `pi-embedded-CQnl8oWA.js` lines 19789-19900

The `executeNodeHostCommand()` function:

1. Resolves exec approvals via `resolveExecApprovals()`
2. Checks security policy: `if (hostSecurity === "deny") throw new Error("exec denied")`
3. If security is "allowlist": checks command against allowed patterns
4. If approval required: waits for human approval via gateway
5. Calls `system.run` to spawn the process

**Guard layers for exec:**
1. Tool policy: `exec` must be in the tool list (filtered by `applyToolPolicyPipeline`)
2. Exec security: `tools.exec.security` must not be "deny"
3. Allowlist: command must match an allowed pattern
4. Approval: `tools.exec.ask` can require human approval per command
5. safeBins: pre-approved binaries that skip the allowlist

**In Hard Shell:** exec is denied at BOTH the tool policy level (tools.deny includes "exec") AND the exec security level (tools.exec.security = "deny"). Two independent guards, both blocking.

**In Split Shell:** exec is enabled with `security: "allowlist"` and `ask: "always"`. Only safeBins-approved commands execute, and each requires Telegram approval.

---

## 6. Elevated Access

**There is no global "elevated mode" in the code.** What the docs call "elevated access" is actually:
- Per-tool `ownerOnly` flags (checked via `wrapOwnerOnlyToolExecution()`)
- Exec approval configuration (`security`, `ask`, `safeBins`)
- These are already disabled in our config (`tools.elevated.enabled: false`)

---

## 7. What We Got Wrong (And Why)

| Assumption | Reality | Source |
|-----------|---------|--------|
| Config format is YAML | JSON5 with Zod validation | Config crashes on invalid keys |
| `--config` flag exists | Does not exist; config at `~/.openclaw/openclaw.json` | CLI help output |
| `sandbox.mode` provides security | It controls Docker spawning, not tool policy | `sandbox-DY8nmmZL.js:2005` |
| Telegram bypasses proxy due to grammY bug | Root cause was `block_private=true` in mitmproxy | Proxy logs: "killed by block_private" |
| `apiKey` field in auth profile | Correct field is `key` (not `apiKey`) | `auth-profiles-6WJHPoy1.js:14200` |
| Agent can bypass tool policy | Tool list filtered before LLM sees it — impossible to call unseen tool | `reply-Deht_wOB.js:64790` |

---

## 8. How Our Vault Synergizes With OpenClaw

Instead of fighting OpenClaw's architecture, our vault layers complement it:

| OpenClaw Layer | What It Does | Our Vault Layer | What It Adds |
|---------------|-------------|-----------------|-------------|
| Tool policy (deny/allow) | Filters tools before LLM sees them | Container isolation | Even if tool policy has a bug, container limits blast radius |
| Exec security (deny/allowlist) | Blocks or gates shell commands | Read-only root + noexec tmpfs | Even if exec runs, can't write/execute files |
| Auth profiles (API keys) | Stores and sends credentials | Proxy key injection | Real key never enters the container |
| Sandbox mode (Docker) | Isolates tool execution | Not needed — container IS the sandbox | Our container is stronger than OpenClaw's Docker sandbox |
| DM policy (pairing) | Controls who can message the agent | Network proxy + allowlist | Controls what the agent can reach |

The vault doesn't replace OpenClaw's security — it wraps it in a hardware-enforced boundary that OpenClaw's software-level controls cannot escape.
