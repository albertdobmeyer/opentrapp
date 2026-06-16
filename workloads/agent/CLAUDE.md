# vault-agent — Hardened Agent Runtime Container

> **This is the `agent` workload of the [opentrapp](https://github.com/albertdobmeyer/opentrapp) monorepo** — `workloads/agent/`, container `vault-agent`. It is **not** a standalone repo or submodule (see "This is a OpenTrApp workload" below). Older docs call it "OpenCli-Container"; that name is retired.

## What This Is

`vault-agent` is the **hardened runtime container** that safely runs an autonomous CLI agent inside a containerized, proxy-gated environment. The default (and only currently-built) agent is **OpenClaw**; the container is a hardened, agent-agnostic **base + a pluggable recipe** (`recipes/<name>/`, default `openclaw`) so it can wrap other agents (e.g. opencode) without weakening the perimeter — see [`recipes/README.md`](recipes/README.md). Its core innovations:

1. **API keys never enter the agent container** — a mitmproxy sidecar injects credentials at the network layer
2. **All network traffic is logged and filtered** — domain allowlist enforced by the proxy
3. **Tool policy works with container isolation** — OpenClaw's own tool filtering prevents the LLM from seeing denied tools, while container hardening limits blast radius if any software layer has a bug

**Role in ecosystem**: `runtime` — the innermost execution environment where AI agents actually run.

**For detailed source code analysis:** See `docs/openclaw-internals.md`

## This Is a OpenTrApp Workload

Post [ADR-0013](../../docs/adr/0013-monorepo-consolidation.md) this lives **in the opentrapp monorepo** at `workloads/agent/` — **not** a git submodule. The old `components/opencli-container/` submodule layout was consolidated 2026-05-30; `components/` no longer exists. The `component.yml` in this directory is the **manifest contract** that tells the OpenTrApp daemon/GUI how to discover, display, and control this workload.

### Manifest Contract Rules
- `component.yml` must always parse as valid YAML
- `identity.id` must be `agent` (the daemon/GUI uses this as a stable key)
- `identity.role` must be `runtime`
- All `available_when` values must reference states declared in `status.states`
- All `restart_command` values in configs must reference command IDs in `commands`
- Command IDs must be unique
- Health probe IDs must be unique

### Validating the Manifest
From the opentrapp root:
```bash
bash tests/orchestrator-check.sh        # Validates all manifests including this one
(cd app/src-tauri && cargo test --lib)  # Rust tests parse the manifests
```

## Architecture

```
Core agent ⇄ proxy view — the full perimeter is five containers (see below):

┌─────────────────────────────────────────────┐
│  vault-proxy (mitmproxy sidecar)            │
│  - Holds real API keys (in env vars)        │
│  - Enforces domain allowlist                │
│  - Injects API key into request headers     │
│  - Logs every request as structured JSON    │
│  - No internet (reaches via vault-egress)   │
│  - SETUID/SETGID caps for gosu entrypoint   │
└──────────────┬──────────────────────────────┘
               │ HTTP proxy (agent-net, internal)
┌──────────────┴──────────────────────────────┐
│  vault-agent (agent container)               │
│  - OpenClaw gateway running with Haiku      │
│  - Read-only root filesystem                │
│  - All Linux capabilities dropped           │
│  - Custom seccomp (deny-by-default)         │
│  - noexec on all tmpfs mounts               │
│  - no-new-privileges flag set               │
│  - tini PID 1 (signal forwarding)           │
│  - Non-root user (vault, uid 1000)          │
│  - PID limit, 4GB RAM, 2 CPUs              │
│  - NO internet (vault-internal only)        │
│  - NO real API keys (placeholder only)      │
│  - sandbox.mode="off" (container IS sandbox)│
│  - Tool profile: coding (see tool-manifest)  │
│  - Telegram bot for user interaction        │
└─────────────────────────────────────────────┘
```

### The Full Perimeter

`vault-agent` + `vault-proxy` are the core of the **five-container perimeter** defined in `compose.yml` at the opentrapp root (ADR-0009). The headline guarantee — **USP-1: privilege separation** — is that **no single container holds both the API keys and internet access**:

- **vault-proxy** — holds the real API keys, enforces the L7 (application-layer) allowlist, injects credentials. **No internet** — reaches the network only through vault-egress.
- **vault-egress** — has `NET_ADMIN` + internet and enforces the L3 (network-layer) filter. **No keys.**
- **vault-skills** (the Skill Firewall) — downloads + pattern-scans + CDR-rebuilds SKILL files in isolation the agent **cannot tamper with**, delivering certified-clean output to the agent via the read-only `skills-deliveries` volume (**USP-2: anti-tamper supply-chain defense**). On skills-net.
- **vault-social** — generalized agent-social-feed vetting; a live AT Protocol (Bluesky) adapter shipped (ADR-0017). **Opt-in / on-demand**; full build-out as the third concern is **deferred** until Vault/Skill/GUI are closed (ADR-0024).

The skill and social workloads reach the internet only through vault-proxy → vault-egress and cannot reach `vault-agent` or each other directly.

```
five-container perimeter (opentrapp compose.yml):

  internet ◄── vault-egress ◄──(egress-net)── vault-proxy ──┬─ agent-net ─── vault-agent
               (NET_ADMIN, no keys)    (holds keys, no net)  ├─ skills-net ── vault-skills
                                                             └─ social-net ── vault-social

  vault-skills ──(skills-deliveries, read-only volume)──► vault-agent   (certified-clean skills)
```

### How Our Vault Synergizes With OpenClaw

| OpenClaw Layer | What It Does | Our Vault Layer | What It Adds |
|---------------|-------------|-----------------|-------------|
| Tool policy (deny/allow) | Filters tools before LLM sees them | Container isolation | Limits blast radius if tool policy has a bug |
| Exec security (deny/allowlist) | Blocks or gates shell commands | Read-only root + noexec tmpfs | Even if exec runs, can't write/execute files |
| Auth profiles (API keys) | Stores and sends credentials | Proxy key injection | Real key never enters the container |
| Sandbox mode (Docker) | Isolates tool execution | Not used — container IS the sandbox | Our container is stronger than OpenClaw's Docker sandbox |
| DM policy (pairing) | Controls who can message the agent | Network proxy + allowlist | Controls what the agent can reach |

## Directory Structure

```
workloads/agent/                    (the `agent` workload of the opentrapp monorepo)
├── Containerfile                   Hardened multi-stage base (Node 22-alpine) + recipe COPY
├── recipes/                        Pluggable agent recipes — base+recipe (see recipes/README.md)
│   ├── openclaw/install.sh         OpenClaw build install (default recipe)
│   └── opencode/install.sh         opencode recipe — fail-closed scaffold (part 2b)
├── compose.yml                     Container + proxy orchestration
├── .env.example                    API key + bot token template (gitignored)
├── component.yml                   MANIFEST — OpenTrApp contract
├── config/
│   ├── tool-manifest.yml           Source of truth — all tools, risk, injection vectors
│   ├── openclaw-hardening.json5    Agent config (JSON5, baked into image)
│   ├── hard-shell.json5            Hard Shell preset config
│   ├── split-shell.json5           Split Shell preset config
│   ├── soft-shell.json5            Soft Shell preset config
│   ├── hard-shell-allowlist.txt    Hard Shell domain template
│   ├── vault-seccomp.json          Syscall filter (vault container)
│   └── vault-proxy-seccomp.json    Syscall filter (proxy container)
├── patches/
│   └── fix-telegram-proxy.sh       Source patch: Telegram proxy bypass fix
├── proxy/
│   ├── vault-proxy.py              mitmproxy addon (key injection + allowlist)
│   └── allowlist.txt               Active domain allowlist
├── scripts/
│   ├── entrypoint.sh               Container startup (config + CA cert + auth)
│   ├── proxy-bootstrap.mjs         Global undici proxy dispatcher
│   ├── tool-control.sh              Per-tool whitelisting/blacklisting + shell-level switching
│   ├── tool-control-core.py        Config generator core (python3, called by tool-control.sh)
│   ├── setup.sh / setup.ps1        One-command setup
│   ├── kill.sh / kill.ps1          Three-level kill switch
│   └── verify.sh                   24-point security verification
├── monitoring/                     [Stubs] Skill scanner, log parser
├── tests/                          Isolation verification tests
└── docs/
    ├── openclaw-internals.md       Source code analysis (verified knowledge)
    ├── phase1-findings.md          Phase 1 compatibility test results
    ├── openclaw-reference.md       How OpenClaw works (tools, config, Telegram, sessions)
    ├── roadmap.md                  Phased development plan
    └── setup-guide.md              Non-technical user setup guide
```

## OpenClaw Configuration

**Format:** JSON5 (NOT YAML). File: `~/.openclaw/openclaw.json` inside the container.
**Validation:** Zod schema at startup — unknown keys cause Gateway to refuse to start.
**Config placed by:** `scripts/entrypoint.sh` copies `config/openclaw-hardening.json5` to the correct path.

### Key Config Decisions

| Setting | Hard Shell | Split Shell | Soft Shell |
|---------|------------|-------------|------------|
| `tools.profile` | `minimal` | `coding` | `coding` |
| `tools.exec.security` | `deny` | `allowlist` | `allowlist` |
| `tools.exec.ask` | `always` | `always` | `on-miss` (safeBins auto-approve) |
| `tools.exec.safeBins` | none | 16 | 26 |
| Enabled tools | 0 | 11 | 17 |
| Risk score | 0.0 | 0.18 | 0.34 |
| Proxy domains | 3 | 3 | 4 |

**Invariants (all shell levels):** `sandbox.mode: "off"`, `elevated.enabled: false`, `gateway.mode: "local"`, `channels.telegram.dmPolicy: "pairing"`, `fs.workspaceOnly: true`

## Commands

| Command ID | Shell | Danger | Description |
|-----------|-------|--------|-------------|
| `setup` | `make setup` | caution | Build container environment |
| `start` | `make start` | safe | Start vault + proxy |
| `soft-stop` | `make stop` | safe | Graceful container stop |
| `hard-kill` | `make kill` | destructive | Force stop, remove containers + volumes |
| `nuclear-kill` | `make nuclear` | destructive | Remove everything |
| `verify` | `make verify` | safe | 24-point security check |
| `test` | `make test` | safe | Run all test scripts |
| `tools-status` | `make tools-status` | safe | Show per-tool enabled/disabled status |
| `hard-shell` | `make hard-shell` | caution | Switch to Hard Shell preset |
| `split-shell` | `make split-shell` | caution | Switch to Split Shell preset |
| `soft-shell` | `make soft-shell` | caution | Switch to Soft Shell preset |
| `network-report` | `make network-report` | safe | Analyze proxy logs for anomalies |
| `session-report` | `make session-report` | safe | Post-session activity summary |
| `log-rotate` | `make log-rotate` | safe | Rotate proxy logs, check transcript size |
| `logs` | `podman logs -f vault-agent` | safe | Stream vault logs |
| `proxy-logs` | `podman logs -f vault-proxy` | safe | Stream proxy logs |

## Editable Configs (via GUI)

| Path | Format | Danger | Notes |
|------|--------|--------|-------|
| `.env` | env | caution | API keys + bot tokens. Restart required. |
| `proxy/allowlist.txt` | line-list | safe | Domain allowlist. Hot-reloadable (SIGHUP). |
| `config/openclaw-hardening.json5` | json5 | destructive | Agent security policy. Rebuild required. |

## Security Patches Applied

| Patch | Target | Purpose | Removable When |
|-------|--------|---------|----------------|
| `patches/fix-telegram-proxy.sh` | `dist/send-DslMV0Oj.js` | Preserves ProxyAgent in Telegram global dispatcher | OpenClaw ships upstream fix (PR #30367) |

## Security Model

### Six Defense Layers
1. **Container isolation** — read-only root, caps dropped, seccomp, noexec, non-root, PID/mem limits
2. **Network proxy** — domain allowlist, API key injection, request logging, payload size limits
3. **Tool policy** — deny list filters tools BEFORE LLM sees them (verified in source code)
4. **Application restrictions** — sandbox.mode, workspaceOnly, elevated disabled
5. **Exec controls** — security: allowlist, ask: always, safeBins whitelist (Split Shell)
6. **Hardening config** — DM pairing, no persistence, telemetry disabled

### 24-point verification (verify.sh)
- **Checks 1-14:** Universal container hardening — proxy DNS, TCP, read-only root, caps dropped, no host mounts, no interop, API-key isolation, no Docker socket, no sudo, non-root, seccomp, noexec, no-new-privileges, PID limit
- **Checks 15-18:** Shell-specific config — adapts to detected shell level (Hard: profile=minimal, exec=deny; Split: profile=coding, exec=allowlist+always; Soft: profile=coding, exec=allowlist+on-miss; safeBins match profiles)
- **Checks 19-24:** Per-tool security — NEVER-enable tools denied, rm not in safeBins, no interpreters in safeBins, proxy allowlist verified, risk score within expected range, config integrity hash (tamper detection)

## Development Principles

**This is a zero-trust security project.** Our public claim is that anybody can use a clawbot securely on their private machine without ever risking leaks of private or sensitive data. Every line of code must uphold this promise.

### Process
- Work slowly and methodically — one task at a time
- Always validate a change before moving to the next task
- Read and understand existing code before modifying it
- No batching or rushing — security-critical work deserves patience
- When in doubt, stop and verify rather than assume

### Research First
- **Always consult OpenClaw's official documentation** before making assumptions about how OpenClaw works. We are wrapping their system — we must understand it accurately.
- Do not guess at OpenClaw config syntax, tool behavior, or API formats — look it up or verify from source code
- When official docs are insufficient, verify from the actual dist/ source bundles inside the container (see `docs/openclaw-internals.md` for methodology)

### Security Model
- **Each tool use is a potential injection attack vector.** Every tool added to the agent's capabilities expands the attack surface. Evaluate accordingly.
- **No trust jumps.** Complete and test the current shell level fully before designing the next one. Each new capability must be individually verified.
- **Shell levels are completed in order.** Hard Shell → Split Shell → Soft Shell. Never skip levels, never bundle multiple capability expansions.

### Spec-Driven Development
- **Every new feature requires a written spec before implementation.** No exceptions.
- Specs live in `docs/specs/` and must cover: purpose, security implications, implementation approach, and verification plan
- The spec must be reviewed and approved before code is written
- This applies to all changes that affect the security boundary: tool policy, safeBins, allowlist, container config, exec controls

## What NOT to Do

- Do not change `identity.id` or `identity.role` in component.yml without coordinating with opentrapp
- Do not remove or rename command IDs that the GUI depends on — add new ones instead
- Do not put real API keys anywhere except `.env` (which is gitignored) — they belong in the proxy only
- Do not disable seccomp profiles or add capabilities — the security model is defense-in-depth
- Do not modify allowlist.txt to include ClawHub domains (11.9% malware rate)
- Do not set `tools.elevated.enabled: true` — permanently disabled
- Do not add interpreters (node, sh, python) to `tools.exec.safeBins`
- Do not add destructive commands (rm, rmdir) to `tools.exec.safeBins` — deletion is user-side only
- Do not add permission tools (chmod, chown) to `tools.exec.safeBins`
- Do not change `sandbox.mode` from `"off"` — the container IS the sandbox
- Do not give the agent any destructive capabilities — the agent is constructive only (read, write, create, search); all destructive operations are handled by the user or Claude from the host side

---
*Last updated: 2026-06-16 — monorepo workload (id `agent`, container `vault-agent`); five-container perimeter; pluggable agent-recipe base (default `openclaw`). All three shell levels verified.*
