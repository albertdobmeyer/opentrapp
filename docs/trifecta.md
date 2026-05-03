# The Trifecta — Perimeter Defense for the OpenClaw Ecosystem

**Updated:** 2026-04-15
**Supersedes:** Previous version (2026-04-06, separate host-side components model)
**Design spec:** `docs/superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md`

---

## The Problem

OpenClaw is a powerful autonomous AI agent with unrestricted access to your machine by default. It can execute shell commands, read your files, control a browser, send messages as you, and install skills from a registry where 11.9% of packages were malware (ClawHavoc, 341/2,857 skills). Running it raw is like giving a stranger your laptop password.

Everything in the OpenClaw ecosystem is a potential injection attack: the agent itself, SKILL files from the registry, social content from other agents on Moltbook, even the URLs the agent visits. A single defense layer is not enough. A container can be misconfigured. A scanner can miss a pattern. A feed filter can be bypassed by encoding.

The only reliable defense is a **perimeter** — all untrusted content stays inside hardened containers, never touching the user's host machine. And the restrictions must be **intelligent** — a reasoning model decides how much freedom the agents get, based on context.

## The containerized workshop Allegory

The OpenClaw agents are like contained robots. They're powerful and resourceful — they can code, browse the web, manage files, send messages. But they're potentially dangerous: they might steal credentials, exfiltrate data, or corrupt the user's system. Many people want to use their labor because it's cheap and effective.

The question: how do you let inmates do useful work without giving them access to steal from or damage your house?

| Concept | containerized workshop Analogy | Lobster-TrApp |
|---------|---------------|---------------|
| **The Fence** | containerized workshop perimeter wall | Container network — all untrusted content stays inside |
| **The Cell Block** | Where inmates live | vault-agent — where OpenClaw runs, heavily restricted |
| **The Workshop** | Where inmates work under inspection | vault-forge — where SKILL files are scanned and rebuilt |
| **The Monitoring Station** | Wiretap room / visitor area | vault-pioneer — where social feeds are analyzed |
| **The Gate** | The only door in/out, with guards | vault-proxy — API key injection, domain allowlist, logging |
| **The Warden** | containerized workshop director, makes judgment calls | Claude Code / Opus — intelligent security decisions |
| **The Control Panel** | Warden's security screens | Lobster-TrApp GUI — visual interface to the perimeter |
| **The Leash** | Adjustable restrictions per inmate | Dynamic Shell — Hard/Split/Soft security levels |
| **The human** | containerized workshop owner / taxpayer | The user — gives high-level instructions, approves escalations |

Key insight: **the workshop and monitoring station are INSIDE the containerized workshop, not outside.** You don't bring untrusted materials out of the containerized workshop and inspect them in your kitchen. Everything that touches untrusted content stays behind the fence.

## Multi-Agent Trust Chain

```
TIER 1: TRUSTED (full host access, makes decisions)
├── human User (interactive — gives instructions in plain language)
└── Claude Code / trusted CLI agent (the warden — intelligent middleman)
        │
        │  manages via CLI, API, or GUI
        ▼
TIER 2: INFRASTRUCTURE (enforces boundaries mechanically)
└── Lobster-TrApp perimeter (4 containers + GUI)
        │
        │  contains, monitors, controls
        ▼
TIER 3: CONTAINED (does the work, within boundaries)
└── OpenClaw agents (bots, Telegram gateway, skills, network interactions)
```

- **Tier 1** operates on the user's host. The human gives high-level instructions. Their trusted agent (Claude Code, powered by Opus) is the intelligent warden — translates human intent into security decisions, monitors agent behavior, adjusts the shell in real time, reports back in plain language.
- **Tier 2** enforces all boundaries mechanically. Everything that touches untrusted content runs here inside containers. The GUI is one interface; Claude Code is another. Tier 2 doesn't make decisions — it enforces what the warden decides.
- **Tier 3** is the contained workforce. OpenClaw agents do automation work within boundaries set by Tier 2 and decided by Tier 1. Powerful but untrusted.

## Container Topology

```
USER'S HOST (protected — zero untrusted content here)
│
├── Claude Code / trusted agent (the warden)
├── Lobster-TrApp GUI (warden's control panel)
│
└── PERIMETER (Docker/Podman compose network)
    │
    ├── vault-agent (the cell block)
    │   OpenClaw runtime, Telegram bot, CLI agents
    │   Dynamic Shell (Hard/Split/Soft) controls freedom
    │   CANNOT reach host, proxy keys, forge, or pioneer
    │
    ├── vault-forge (the workshop)
    │   CDR pipeline, 87-pattern scanner, skill verification
    │   Downloads untrusted SKILL files INTO this container
    │   Clean output delivered to agent via shared volume
    │   ISOLATED from agent (agent cannot tamper with inspection)
    │
    ├── vault-pioneer (the monitoring station)
    │   Feed scanner, 25 injection patterns, content analysis
    │   Processes untrusted Moltbook content here
    │   ISOLATED from agent (agent cannot influence analysis)
    │
    └── vault-proxy (the gate)
        Only connection to outside internet
        Holds real API keys (never in any other container)
        Domain allowlist, payload size limits, traffic logging
        Bridges all internal networks
```

### Network Isolation

Each component gets its own internal network. Only the proxy bridges them.

| Source | Destination | Allowed | Purpose |
|--------|------------|---------|---------|
| vault-agent | vault-proxy | Yes | Internet access (filtered, logged, key-injected) |
| vault-agent | vault-forge | No | Agent cannot influence scanning |
| vault-agent | vault-pioneer | No | Agent cannot influence feed analysis |
| vault-agent | host | No | Air-gapped from user's system |
| vault-forge | vault-proxy | Yes | Download skills from registry via proxy |
| vault-forge | vault-agent | Write-only (shared volume) | Deliver certified skills |
| vault-pioneer | vault-proxy | Yes | Fetch feeds via proxy |
| vault-proxy | internet | Yes | The only external connection |
| host (GUI/Claude) | vault-proxy | Yes | Management, monitoring, control |

## The Three Modules

### openclaw-vault — The Cell Block (Runtime Containment)

**Role:** `runtime` — the hardened container that runs the agent.

Wraps OpenClaw in a six-layer defense-in-depth container. The agent runs inside; secrets stay outside. All traffic is logged and filtered. The user controls everything from Telegram or the GUI.

**Key innovations:**
- API keys never enter the agent container (proxy-side injection)
- Domain allowlist enforced at the network layer
- Three shell levels (Hard/Split/Soft) with intelligent switching
- 24-point security verification
- Three-level kill switch (soft/hard/nuclear)

### clawhub-forge — The Workshop (Supply Chain Defense)

**Role:** `toolchain` — the security gatekeeper for skills.

Runs **inside the perimeter** as its own container. Downloads untrusted SKILL files into the container, scans them with 87 MITRE ATT&CK patterns, rebuilds them from scratch via CDR, and delivers certified clean output to the agent workspace via shared volume. Untrusted content never touches the host.

**Key innovations:**
- Content Disarm & Reconstruction (CDR) — downloaded skills rebuilt from semantic intent
- Offline scanner (87 patterns, zero-trust line-by-line verification)
- Security certificates (clearance reports with SHA-256)
- All scanning happens inside the container — a compromised host cannot bypass the scanner

### moltbook-pioneer — The Monitoring Station (Social Intelligence)

**Role:** `network` — safe reconnaissance and participation on Moltbook.

Runs **inside the perimeter** as its own container. Processes untrusted social content here — the agent never sees unfiltered feed content. 25 injection patterns tuned for social engineering attacks.

**Key innovations:**
- Feed scanning isolated from the agent (agent cannot manipulate analysis)
- Pattern export for proxy-level auto-filtering
- Three engagement levels (Observer/Researcher/Participant)

**Current status:** Moltbook API is down (since 2026-04-05). Pioneer is containerized and ready but feed integration is deferred.

---

## The Dynamic Shell — The USP

The product's reason to exist. Critics say AI agent security is impossible: either you restrict the agent into uselessness, or you let it loose and accept the risk.

The dynamic shell breaks this dichotomy. And it's not just adjustable — it's **intelligent.**

### Shell Levels

| Level | Analogy | Agent Can... | Agent Cannot... |
|-------|---------|-------------|----------------|
| **Hard Shell** | The cage | Chat via Telegram | Run commands, read files, browse web, install skills |
| **Split Shell** | The arena | Read/write files, run safe commands (each needs approval) | Anything not explicitly approved |
| **Soft Shell** | The safari | Browse web, schedule tasks, run commands autonomously | Touch the driver seat: SSH keys, passwords, root, admin accounts. Ever. |

### Intelligent Meta-Orchestration

A large reasoning model (Claude Opus via Claude Code) acts as the warden. It doesn't just flip shell levels — it makes **contextual, per-action security decisions in real time:**

- Reads scan results from forge and decides whether a skill is safe to install
- Monitors agent behavior and detects anomalies
- Adjusts the shell dynamically based on what the agent is doing
- Translates security events into plain language for the human
- Approves or denies agent requests based on task context

The human doesn't need to understand security. The agent can't bypass the infrastructure. The warden bridges both worlds.

---

## Ownership Matrix

Each capability has exactly one owner. No duplication, no ambiguity.

| Capability | Owner | Location |
|---|---|---|
| Container isolation (read-only, caps, seccomp) | vault | vault-agent container |
| API key injection via proxy | vault | vault-proxy container |
| Domain allowlist enforcement | vault | vault-proxy container |
| Shell level switching (Hard/Split/Soft) | vault | vault-agent container |
| Kill switch (soft/hard/nuclear) | vault | host → container management |
| Runtime monitoring (proxy logs, session audit) | vault | vault-proxy + host volume |
| 24-point security verification | vault | vault-agent container |
| Skill scanning (87 MITRE patterns) | forge | vault-forge container |
| Skill linting and structure validation | forge | vault-forge container |
| Zero-trust skill verification | forge | vault-forge container |
| CDR pipeline | forge | vault-forge container |
| Feed injection scanning (25 patterns) | pioneer | vault-pioneer container |
| Platform census and trends | pioneer | vault-pioneer container |
| Agent identity safety checklist | pioneer | vault-pioneer container |
| Workflow orchestration | lobster-trapp | GUI / CLI |
| Cross-component workflows | lobster-trapp | `config/orchestrator-workflows.yml` |

---

## Manifest-Driven Workflows

Components self-describe via `component.yml` with 6 sections:

1. **identity** — who is this component
2. **status** — how to determine its state
3. **commands** — individual operations
4. **configs** — editable configuration files
5. **health** — dashboard badges
6. **workflows** — multi-step automated sequences (new in v2)

Workflows chain declared commands into single user-facing actions. The GUI executes them with progress tracking. Claude Code executes them programmatically.

**Component workflows** handle sequences within one component (e.g., forge: scan → verify → certify).
**Orchestrator workflows** handle sequences across components (e.g., forge.scan → vault.install). These live in `config/orchestrator-workflows.yml`.

---

## Defense-in-Depth: How the Layers Stack

### Against a compromised agent (runtime threat):

| Layer | Module | Container | What stops it |
|---|---|---|---|
| 1. Exoskeleton | vault | vault-agent | Read-only root, caps dropped, seccomp, noexec, PID/mem limits |
| 2. Network proxy | vault | vault-proxy | Domain allowlist, payload size limits, request logging |
| 3. Tool policy | vault | vault-agent | Denied tools never sent to LLM |
| 4. Exec controls | vault | vault-agent | safeBins allowlist + human approval |
| 5. Workspace restriction | vault | vault-agent | workspaceOnly: true |
| 6. Kill switch | vault | host | Soft stop, hard kill, nuclear eject |

### Against a malicious skill (supply chain threat):

| Layer | Module | Container | What stops it |
|---|---|---|---|
| 1. Skill scanner | forge | vault-forge | 87 patterns detect malicious constructs |
| 2. Zero-trust verifier | forge | vault-forge | Every line classified |
| 3. CDR rebuild | forge | vault-forge | Original discarded, rebuilt from intent |
| 4. Domain allowlist | vault | vault-proxy | ClawHub domains blocked by default |
| 5. Network isolation | perimeter | compose network | Forge isolated from agent |
| 6. Exoskeleton | vault | vault-agent | Container limits blast radius |

### Against hostile feed content (social threat):

| Layer | Module | Container | What stops it |
|---|---|---|---|
| 1. Feed scanner | pioneer | vault-pioneer | 25 injection patterns |
| 2. Network isolation | perimeter | compose network | Pioneer isolated from agent |
| 3. DM pairing policy | vault | vault-agent | Each Telegram user approved |
| 4. Tool policy | vault | vault-agent | Denied tools stay invisible |
| 5. human/warden approval | Tier 1 | host | User sees every action |

---

## Current Status

| Module | Maturity | Container | Key Achievement |
|---|---|---|---|
| **openclaw-vault** | 100% | vault-agent + vault-proxy | 24-point verify, 3 shell levels, 6-layer defense |
| **clawhub-forge** | 100% | vault-forge | 87-pattern scanner, CDR pipeline, 25 skills certified |
| **moltbook-pioneer** | 100% | vault-pioneer | 48 tests, 25 injection patterns, 3 engagement presets |
| **lobster-trapp** | ~85% | n/a (host) | GUI functional, workflow executor + UI implemented, perimeter orchestrated |

### What's Implemented

- 4-container compose with network isolation (verified)
- Manifest schema with workflows section
- 10 component workflows + 4 orchestrator workflows defined
- 41-check validation suite (all passing)
- Containerfiles for forge and pioneer
- Rust workflow executor with interpolation, sequencing, and success conditions (Phase 4, c670e9a)
- React workflow UI with progress tracking, input forms, and danger-level styling (Phase 5, 9a5cd78)

### What Remains

- E2E user journey blockers: Podman install guidance in wizard, API key entry in config step (see `docs/handoff.md`)
- UX polish: dashboard onboarding, error messages, landing page language simplification
- Claude Code integration / MCP server (Phase 7, post-v0.1.0)

---

*This document is the single source of truth for how the three modules relate. Per-module details live in each module's own docs. The full architecture spec is at `docs/superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md`.*
