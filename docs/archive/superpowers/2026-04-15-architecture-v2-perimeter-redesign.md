# Lobster-TrApp Architecture v2 — Perimeter Redesign

**Date:** 2026-04-15
**Supersedes:** Implicit architecture from `docs/trifecta.md` (module separation model)
**Status:** Design — pending implementation planning
**Cross-references:**
- `docs/trifecta.md` (current module relationships — to be updated)
- `docs/product-assessment.md` (USP analysis, user journey requirements)
- `GLOSSARY.md` (Shell system terminology)
- `components/openclaw-vault/docs/specs/2026-03-30-skill-installation-path.md` (current forge→vault flow — to be replaced)
- `components/openclaw-vault/docs/specs/2026-03-30-feed-scanning-deferred.md` (pioneer integration — absorbed by this spec)
- `components/clawhub-forge/docs/forge-identity-and-design.md` (forge identity — to be updated)
- `components/moltbook-pioneer/docs/specs/2026-04-04-vault-integration-design.md` (pioneer integration — absorbed by this spec)

---

## Problem Statement

### The Security Gap

The current architecture runs clawhub-forge and moltbook-pioneer as bare bash scripts on the user's unprotected host machine. Both components handle untrusted content:

- **Forge** downloads SKILL files from ClawHub (11.9% malware rate in the ClawHavoc incident), processes them with bash parsers, and stores them on the host filesystem. A vulnerability in the parser gives the attacker the user's entire machine.
- **Pioneer** fetches content from the Moltbook API (25 known injection patterns) and processes it on the host. Same risk.

The vault carefully air-gaps the OpenClaw agent from the user's system with six layers of defense — but two other components that handle equally untrusted content run with zero isolation. This is one wolf inside the fence and two wolves loose in the garden.

### The UX Gap

The target user is non-technical ("can barely find a download button"). The current architecture requires them to:
- Run bash scripts in a terminal (`make scan-one SKILL=api-dev`)
- Read JSON clearance reports
- Execute `podman cp` commands to transfer files between host and container
- Manually operate three independent CLI tools

This contradicts the documented product vision: "API key + Telegram, everything else automatic."

### The Conceptual Gap

Lobster-TrApp is documented as a "monorepo orchestrator" — a dashboard that discovers components via manifests. But it should be the **security infrastructure** itself. The vault, forge, pioneer, and proxy are not independent products displayed in a GUI. They are parts of one containment system.

---

## The Multi-Agent Trust Chain

Lobster-TrApp exists in a multi-agent orchestration system with three trust tiers:

```
TIER 1: TRUSTED (full host access, makes decisions)
├── human User (interactive — gives instructions, approves escalations)
└── Claude Code / trusted CLI agent (automated — executes on user's behalf)
        │
        │  manages via CLI, API, or GUI
        ▼
TIER 2: INFRASTRUCTURE (enforces the security boundary)
└── Lobster-TrApp (containment system — walls, gate, workshop, monitoring)
        │
        │  contains, monitors, controls
        ▼
TIER 3: CONTAINED (does the work, within boundaries)
└── OpenClaw system (agents, Telegram bot, skills, network interactions)
```

- **Tier 1** operates on the user's host with full trust. The human gives high-level instructions in natural language. Their trusted agent (Claude Code, powered by a large reasoning model like Opus) is the **intelligent warden** — it translates human intent into security decisions, monitors agent behavior, adjusts the dynamic shell in real time, and reports back in plain language. The human doesn't need to understand security; the warden does.
- **Tier 2** is the containerized workshop infrastructure. It enforces all security boundaries mechanically. Everything that touches untrusted content runs here, inside containers. The Tauri GUI is one interface to this infrastructure; Claude Code is another. Tier 2 doesn't make decisions — it enforces what the warden (Tier 1) decides.
- **Tier 3** is the contained workforce. OpenClaw agents do automation work (file management, web browsing, messaging, scheduling) within the boundaries set by Tier 2 and decided by Tier 1. The agents are powerful and resourceful but untrusted — they request capabilities, the warden judges, the infrastructure enforces.

---

## Corrected Architecture

### Container Topology

All components that handle untrusted content run inside containers in one isolated network:

```
USER'S HOST (protected — never touched by untrusted content)
│
├── Claude Code / trusted CLI agent (the warden)
├── Lobster-TrApp Tauri GUI (warden's control panel)
│
└── PERIMETER (Docker/Podman compose network)
    │
    ├── vault-agent (the cell block)
    │   OpenClaw runtime, Telegram bot, CLI agents
    │   Dynamic Shell (Hard/Split/Soft) controls freedom
    │   Can request scanned skills from forge
    │   CANNOT reach host, proxy keys, or other containers directly
    │
    ├── vault-forge (the workshop)
    │   CDR pipeline, 87-pattern scanner, skill verification
    │   Downloads untrusted SKILL files INTO this container
    │   Scans, rebuilds, certifies — never touches the host
    │   Clean output passed to agent via controlled internal path
    │   ISOLATED from agent (agent cannot tamper with inspection)
    │
    ├── vault-pioneer (the monitoring station)
    │   Feed scanner, 25 injection patterns, content analysis
    │   Processes untrusted Moltbook content here
    │   Filters before agent sees it
    │   ISOLATED from agent (agent cannot influence analysis)
    │
    └── vault-proxy (the gate)
        Only connection to outside internet
        Holds real API keys (never in any other container)
        Domain allowlist enforcement
        Routes: agent↔internet, forge downloads, pioneer feeds
        Logs all traffic (requests.jsonl on host via volume mount)
```

### Network Rules

| Source | Destination | Allowed | Purpose |
|--------|------------|---------|---------|
| vault-agent | vault-proxy | Yes | Internet access (filtered, logged, key-injected) |
| vault-agent | vault-forge | No | Agent cannot trigger or influence scanning directly |
| vault-agent | vault-pioneer | No | Agent cannot influence feed analysis |
| vault-agent | host | No | Air-gapped from user's system |
| vault-forge | vault-proxy | Yes | Download skills from ClawHub via proxy |
| vault-forge | vault-agent | Write-only (shared volume) | Deliver scanned, certified skills to agent workspace via shared volume mount — no network connection |
| vault-forge | host | No | Untrusted content never touches host |
| vault-pioneer | vault-proxy | Yes | Fetch Moltbook feeds via proxy |
| vault-pioneer | vault-agent | No | Filtered content delivered via proxy routing |
| vault-proxy | internet | Yes | The only external connection |
| vault-proxy | all containers | Yes | Routes traffic, enforces policy |
| host (GUI/Claude) | vault-proxy | Yes | Management API, monitoring, control |

### How This Differs from Current Architecture

| Aspect | Current (v1) | Corrected (v2) |
|--------|-------------|----------------|
| Forge location | Host (bash scripts) | Container (vault-forge) |
| Pioneer location | Host (bash scripts) | Container (vault-pioneer) |
| SKILL file flow | Downloaded to host → scanned on host → manually copied to vault | Downloaded into forge container → scanned inside → certified output to agent |
| Feed flow | Fetched on host → analyzed on host | Fetched into pioneer container → filtered → clean content to agent via proxy |
| Container count | 2 (agent + proxy) | 4 (agent + forge + pioneer + proxy) |
| compose.yml services | 2 | 4 |
| User interaction with forge | Terminal: `make scan-one` | None — automatic, invisible |
| User interaction with pioneer | Terminal: `make scan-feed` | None — automatic, invisible |
| Attack surface on host | Forge + pioneer scripts process untrusted input with full host access | Zero untrusted content on host |

---

## The Dynamic Shell — Adjusted for Perimeter

The dynamic shell is the product's USP: adjustable restrictions that make the contained agents useful without being dangerous. With all components inside the perimeter, the shell controls everything:

### Hard Shell (the cage — risk score 0.0)

- Agent can only chat via Telegram
- No skills installed. Forge container idle.
- No feeds processed. Pioneer container idle.
- No file access, no exec, no web browsing
- Use case: passive chatbot, maximum safety, proving the containment works

### Split Shell (the arena — risk score 0.18)

- Agent has selective tools, each requiring warden approval
- Skills can be installed: forge auto-scans, warden approves/rejects result
- Limited feed access: pioneer auto-filters, warden sees flagged content
- Workspace access (persistent volume) but no host mounts
- Use case: useful assistant that asks permission for each action

### Soft Shell (the safari — risk score 0.45)

- Agent has broad tool access with auto-approved safe operations
- Skills install with automatic scanning (forge pipeline runs unattended)
- Feeds flow through automatic filtering (pioneer pipeline runs unattended)
- Broad workspace, web access, cron jobs, memory persistence
- The wall is absolute: SSH keys, passwords, system accounts, Docker socket — NEVER accessible
- Use case: autonomous assistant that gets things done within hard boundaries

Shell transitions:
- **Shell Up** (more restrictive): instant, no confirmation needed
- **Shell Down** (more permissive): requires warden confirmation (human or Claude Code)

### Intelligent Meta-Orchestration — The True USP

The dynamic shell is not just adjustable — it is **intelligent.** A large reasoning model (Claude Opus via Claude Code, or another trusted CLI agent) acts as the warden: the intelligent middleman between the non-technical human user and the powerful but dangerous clawbot system.

**Why this matters:** The non-technical user cannot make security decisions. They don't know what "allow exec with allowlist" means. They don't know whether a SKILL file scan result is acceptable. They don't know when the agent's behavior is suspicious. But Claude Opus does. The meta-orchestration layer is:

```
human: "I want my assistant to manage my calendar"
    ↓
Claude Opus (the warden): understands the request, determines
    that calendar management requires Split Shell with specific
    tools (cron, web_fetch for calendar API), adjusts the shell
    accordingly, monitors agent behavior for anything beyond
    calendar operations, and reports back to the human in plain
    language: "Your assistant now has calendar access. It asked
    to also read your email — I blocked that. Want me to allow it?"
    ↓
Lobster-TrApp (infrastructure): enforces the shell configuration
    the warden chose, runs the containers, blocks unauthorized
    actions at the container/proxy level
    ↓
OpenClaw (the worker): operates within the boundaries, does
    the calendar work, cannot escape the restrictions even if
    it tries
```

**The warden makes intelligent decisions:**
- Reads scan results from forge and decides whether a skill is safe to install (not the user, not the agent — the warden)
- Monitors agent behavior and detects anomalies (unexpected network requests, excessive file access, attempts to escalate)
- Adjusts the shell dynamically based on what the agent is doing (shell up automatically if suspicious activity detected)
- Translates security events into plain language for the human ("Your assistant tried to access a website not on its approved list. I blocked it.")
- Approves or denies agent requests in Split Shell mode based on the task context (not blanket allow/deny — contextual judgment)

**This solves the "impossible" problem:** Critics say AI agent security is impossible because either you restrict the agent into uselessness OR you let it loose and accept the risk. The dynamic + intelligent shell breaks this dichotomy: a powerful reasoning model (the warden) makes per-action, per-context security decisions in real time, giving the agent exactly the freedom it needs for the current task and no more. The human doesn't need to understand security. The agent can't bypass the infrastructure. The warden bridges both worlds.

---

## Manifest Evolution

### Current Schema Sections (preserved)

1. **identity** — id, name, version, role, icon, color
2. **status** — declared states + probe commands
3. **commands** — individual operations
4. **configs** — editable configuration files
5. **health** — periodic probes
6. **prerequisites** — setup requirements

### New Schema Section: Workflows

Components declare multi-step automated sequences that the GUI (or Claude Code) can execute as single user actions:

```yaml
workflows:
  - id: vet-and-deliver
    name: Install Skill Safely
    description: Download, scan, verify, and deliver a skill to the agent
    user_description: >
      We'll download this skill into a secure sandbox, scan it for
      87 known malware patterns, rebuild it from scratch to remove
      any hidden content, and deliver the clean version to your
      assistant.
    trigger: manual           # or: on-download, on-request, scheduled
    steps:
      - id: download
        command: cdr-download
        args:
          url: "{{input.skill_url}}"
      - id: scan
        command: scan-one
        depends_on: download
        abort_on_failure: true
      - id: rebuild
        command: cdr-rebuild
        depends_on: scan
        abort_on_failure: true
      - id: deliver
        command: deliver-to-agent
        depends_on: rebuild
    inputs:
      - id: skill_url
        type: url
        label: Skill URL
        description: ClawHub URL or direct download link
```

### New Schema Section: Orchestrator Workflows

Cross-component sequences defined at the orchestrator level (not in individual component manifests):

```yaml
# In orchestrator config, not component.yml
orchestrator_workflows:
  - id: full-skill-install
    name: Install a Skill
    user_description: >
      Downloads a skill, scans it for security threats,
      rebuilds it from scratch, and installs it for your assistant.
    steps:
      - component: clawhub-forge
        workflow: vet-and-deliver
      - component: openclaw-vault
        command: install-skill
        args:
          skill_path: "{{previous.output_path}}"
    shell_requirement: split  # minimum shell level to run this workflow
```

---

## User Journey (Post-Redesign)

### First Run

1. User downloads Lobster-TrApp installer (one file, one app)
2. App opens → setup wizard:
   - **Container runtime**: "This app needs Docker Desktop to keep your AI safe. [Download Docker Desktop] — click Continue when installed." (or auto-detects if already present)
   - **API key**: "Enter your Anthropic API key. [Where do I find this?]"
   - **Telegram**: "Create a Telegram bot for your assistant. [Step-by-step guide with screenshots]"
3. App builds all four containers, starts them, runs verification
4. "Your assistant is ready. Open Telegram and send it a message."

### Daily Use

- User talks to their assistant via Telegram
- If they want to install a skill: they open the GUI → "Install Skill" → paste URL → app handles everything (forge scans inside container, shows pass/fail, installs on approval)
- If they want to change security level: GUI shows a slider or toggle, not "Hard Shell / Split Shell" terminology. Plain language: "Restrict to chat only" / "Allow file management" / "Full autonomy"
- Security dashboard shows: agent status, recent activity, blocked threats, shell level

### Warden Use (Claude Code)

Claude Code can manage the same infrastructure programmatically:
- `lobster-trapp status` — show all container states, agent activity
- `lobster-trapp shell soft` — change to Soft Shell
- `lobster-trapp install-skill <url>` — run the full scan→install pipeline
- `lobster-trapp logs` — stream monitoring data
- Or via MCP tools exposed by the Tauri app

---

## What Changes in Each Repo

### lobster-trapp (this repo)

- `CLAUDE.md`: Reframe from "monorepo orchestrator" to "security infrastructure for AI agents"
- `schemas/component.schema.json`: Add `workflows` section
- `app/src-tauri/`: Add workflow executor (runs multi-step sequences across containers)
- `app/src/`: Workflow-driven UI (buttons trigger workflows, not individual commands)
- `docker-compose.yml`: 4 services (agent + forge + pioneer + proxy) in one network
- New: orchestrator workflow definitions (cross-component sequences)
- New: CLI interface for Claude Code / programmatic management
- `docs/trifecta.md`: Update to reflect perimeter model
- `README.md`: Reframe for the corrected architecture and target audience

### openclaw-vault

- `compose.yml`: Evolves from 2 services to part of the parent's 4-service compose
- Network config: `vault-internal` network now shared with forge and pioneer containers
- Skill installation: receives certified skills from forge container via internal network, not host `podman cp`
- Monitoring APIs: exposed via watchtower endpoints for GUI and Claude Code
- Shell configs: unchanged (Hard/Split/Soft still control agent restrictions)
- Security model: unchanged (six layers preserved, extended to cover forge/pioneer)

### clawhub-forge

- New: `Dockerfile` / `Containerfile` — containerize the scanning pipeline
- CDR pipeline: runs inside container, downloads untrusted files INTO the container
- Skill delivery: outputs clean skills to a shared volume or internal network endpoint
- CLI tools (`make` targets): still work for development, but production use is containerized
- Pattern database (87 MITRE patterns): included in container image
- `component.yml`: Add `workflows` section, update `prerequisites.container_runtime: true`
- `forge-identity-and-design.md`: Update to reflect containerized deployment

### moltbook-pioneer

- New: `Dockerfile` / `Containerfile` — containerize the scanning tools
- Feed scanner: runs inside container, processes untrusted content there
- Pattern export: patterns baked into container image, used by proxy routing
- CLI tools: still work for development, production use is containerized
- `component.yml`: Add `workflows` section, update `prerequisites.container_runtime: true`
- `vault-integration-design.md`: Replaced by this spec (integration is now container-native)

---

## What Does NOT Change

- **The six-layer defense model** — extended to cover forge/pioneer, not replaced
- **The dynamic shell concept** — Hard/Split/Soft with the same security properties
- **The proxy key injection** — API keys still only in the proxy container
- **The 24-point verification** — still validates the agent container's hardening
- **The manifest-driven discovery** — components still self-describe via component.yml
- **The scanning patterns** — 87 MITRE + 25 injection patterns, same databases
- **The CDR pipeline** — same quarantine → parse → filter → extract → rebuild flow
- **The test infrastructure** — orchestrator checks, integration tests, unit tests
- **The repo structure** — four repos, submodule model preserved
- **The Tauri app framework** — React 18 + Rust + Tauri 2

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| 4 containers require more RAM than 2 | Forge and pioneer are lightweight (bash + patterns), not GPU/LLM workloads. Estimated +200MB total. | Keep forge/pioneer containers minimal. Allow stopping them when idle (Hard Shell). |
| Container-to-container networking adds complexity | More compose config, more network rules to get right | Well-defined network topology. Integration tests validate connectivity. |
| CDR pipeline needs Ollama (LLM) for intent extraction | Ollama is heavy (1-5GB). Running inside forge container is impractical. Running on host means sending untrusted content outside the perimeter. | CDR intent extraction is optional — scan + structural rebuild work without LLM. If LLM is needed: (a) Ollama runs as a 5th container inside the perimeter (vault-ollama), or (b) intent extraction is deferred until the skill is already cleaned by structural rebuild. Option (a) is correct but adds RAM cost. Option (b) trades capability for simplicity. Decision deferred to implementation. |
| Forge container needs internet to download skills | Must go through proxy (domain allowlist) | Only ClawHub domains allowed for forge. All downloads logged. |
| Increased build time (4 container images) | Longer first-run setup | Parallel builds. Cache layers. Pre-built images on registry (future). |
| Claude Code integration (MCP/CLI) is new scope | Adds engineering work | Phase it: GUI-only first, CLI/MCP in a later release. |

---

## Verification Plan

### Architecture Verification

1. No untrusted content touches the host filesystem (audit all download/fetch paths)
2. Agent container cannot reach forge or pioneer containers directly (network rule test)
3. Forge container can only reach proxy (not internet directly, not agent)
4. Pioneer container can only reach proxy (same)
5. Proxy routes correctly between all containers and enforces allowlist
6. API keys present only in proxy container (env audit across all containers)

### Functional Verification

7. Skill installation workflow: URL → forge scan → rebuild → deliver to agent (end-to-end)
8. Feed scanning: Moltbook request → pioneer filter → clean content to agent (when enabled)
9. Shell transitions: Hard → Split → Soft and back, all constraints enforced
10. GUI: workflow buttons trigger correct multi-step sequences
11. Monitoring: watchtower APIs return correct data to GUI and CLI

### Regression Verification

12. All 24-point vault verification checks still pass
13. All 40 orchestrator checks pass (updated for new topology)
14. All 28 cross-module integration checks pass (updated for container networking)
15. Frontend unit tests pass
16. Rust backend tests pass
17. Playwright E2E tests pass (updated for workflow UI)

---

## Implementation Sequence (High Level)

This section outlines the dependency order. Detailed implementation planning follows in a separate plan document.

1. **Containerize forge** — Dockerfile, test it runs standalone, verify scanning works inside container
2. **Containerize pioneer** — same treatment
3. **Compose network** — 4-service compose.yml with correct network rules
4. **Internal routing** — forge→agent skill delivery, proxy→pioneer feed routing
5. **Schema evolution** — add `workflows` section to component.schema.json
6. **Manifest updates** — all three component.yml files get workflows
7. **Backend workflow executor** — Rust code to run multi-step workflows
8. **Frontend workflow UI** — React components for workflow execution
9. **Setup wizard rebuild** — driven by orchestrator workflow, not hardcoded steps
10. **Documentation reframe** — CLAUDE.md, trifecta.md, README, landing page
11. **Test updates** — orchestrator checks, integration tests, E2E tests for new topology
12. **CLI/MCP interface** — for Claude Code integration (can be phased separately)

---

*This spec supersedes the implicit architecture described in `docs/trifecta.md` (separate host-side components model). Per-component roadmaps and integration specs referenced in the cross-references section are absorbed by this redesign.*
