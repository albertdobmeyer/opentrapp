# Lobster-TrApp Glossary

Official terminology for the Lobster-TrApp ecosystem. Use these terms consistently across all repos, documentation, UI, and conversation.

---

## User-Facing Terms (Frontend Only)

The parent repo (lobster-trapp) is the only thing non-technical users see. These mappings define how developer concepts appear in the GUI. **Never show the developer term in user-facing UI.**

| Developer Term | User-Facing Term | Where Used |
|---|---|---|
| openclaw-vault | **My Assistant** | Sidebar, dashboard, component detail |
| clawhub-forge | **Skills** / **Skill Store** | Sidebar, dashboard |
| moltbook-pioneer | **Agent Network** | Sidebar, dashboard |
| Hard Shell | **Chat Only** | Mode descriptions (post-v0.1.0) |
| Split Shell | **Supervised** | Mode descriptions (post-v0.1.0) |
| Soft Shell | *(default — no name shown)* | Hidden, this is the default experience |
| Component | *(invisible)* | Never shown to users |
| Manifest / component.yml | *(invisible)* | Never shown to users |
| Container / Podman / Docker | **Secure sandbox** | Wizard, status messages |
| Perimeter | *(invisible)* | Never shown to users |
| Proxy / vault-proxy | *(invisible)* | Never shown to users |
| Workflow | **Action** | Button labels |
| Command (user tier) | **Quick action** | Button labels |
| Command (advanced tier) | *(behind "Developer Tools" toggle)* | Hidden by default |
| compose.yml | *(invisible)* | Never shown to users |
| Health probe | **Status badge** | Dashboard indicators |
| 24-point verification | **"Safe" / "Warning"** badge | Single badge, details expandable |

---

## Shell System (Security Levels)

The shell is the security boundary around the OpenClaw agent. Inspired by lobster biology — a harder shell means more protection but less flexibility. The **intelligent warden** (Claude Code / Opus) adjusts the shell based on task context, so the user doesn't need to understand security.

| Term | What It Means | Analogy | Agent Can... | Agent Cannot... |
|------|--------------|---------|-------------|----------------|
| **Hard Shell** | Maximum protection. Conversation only. | The cage | Chat via Telegram. That's it. | Run commands, read files, browse web, schedule tasks, install skills. |
| **Split Shell** | Selective openings. Every action requires approval. | The arena | Read/write files, exec with safeBins (warden or user approves each command). | Access anything not explicitly approved. Protected resources always blocked. |
| **Soft Shell** | Broad autonomy. SafeBins auto-approve. Core protections enforced. | The safari | Search web, schedule tasks, process files, run safeBins commands autonomously. | Touch the driver seat: root, SSH keys, passwords, keyrings, admin accounts. Ever. |

**Shell Up** — Increase protection (e.g., Soft Shell → Hard Shell). Always instant, no confirmation needed. You can always pull the reins.

**Shell Down** — Increase capability (e.g., Hard Shell → Split Shell). Requires explicit confirmation from user or warden. You're granting more freedom — be sure.

**Molt** — Reconfigure the shell. Internally, this means swapping config files, adjusting the proxy allowlist, and restarting the container with new permissions. Named after the lobster molting process where the old shell is shed and a new one forms.

---

## Architecture Terms

| Term | What It Means |
|------|--------------|
| **Perimeter** | The complete 4-container security boundary. All untrusted content (agent runtime, skill files, social feeds) stays inside. Nothing untrusted touches the host. Defined in `compose.yml`. |
| **Exoskeleton** | The container itself — the outer wall that always exists regardless of shell level. Read-only filesystem, dropped capabilities, seccomp profile, noexec mounts. The exoskeleton never comes off. |
| **Vault** | The complete security harness: exoskeleton (container) + proxy (network filter) + tool policy (OpenClaw config) + hardening. "The vault" = the whole package. |
| **Proxy Sidecar** | The mitmproxy container (vault-proxy) that sits between ALL containers and the internet. Filters domains, injects API keys, logs everything. The only door in the fence. |
| **Driver Seat** | The resources that are NEVER accessible to the agent in any shell level: root access, SSH keys, GPG keys, passwords, keyrings, admin accounts, Docker socket, the vault's own config. The user always keeps the steering wheel. |
| **Protected Resources** | Technical term for the driver seat. The list of files, sockets, and capabilities that are excluded from every shell level. |
| **Allowlist** | The list of domains the proxy permits. Everything not on the list is blocked and logged. Each shell level has its own allowlist template. |
| **Placeholder Key** | The dummy API key inside the agent container. The agent uses this to construct API requests, but the proxy replaces it with the real key before forwarding. The agent never sees the real key. |

---

## Trust and Orchestration Terms

| Term | What It Means |
|------|--------------|
| **Trust Tier** | One of three levels in the multi-agent trust chain. Tier 1 (trusted): human + warden with full host access. Tier 2 (infrastructure): the container perimeter that enforces boundaries. Tier 3 (contained): the OpenClaw agents that do the work. |
| **Warden** | The intelligent middleman between the human user and the contained agents. Currently Claude Code powered by Opus. Makes contextual, per-action security decisions in real time — reads scan results, monitors behavior, adjusts the shell, reports in plain language. |
| **Workflow** | A multi-step automated sequence declared in `component.yml` or `config/orchestrator-workflows.yml`. Chains individual commands into a single user-facing action (e.g., "Install Skill" = scan → verify → certify → deliver). The GUI and Claude Code execute workflows as atomic operations. |
| **Orchestrator Workflow** | A workflow that spans multiple components. Defined at the parent orchestrator level in `config/orchestrator-workflows.yml`. References component IDs + command/workflow IDs. |
| **Component Workflow** | A workflow that operates within a single component. Defined in that component's `component.yml`. References the component's own command IDs. |

---

## Product Terms

| Term | What It Means |
|------|--------------|
| **Lobster-TrApp** | The complete product — desktop app + container perimeter + security scanning + ecosystem tools. Pronounced "lobster trap." |
| **NewLobsterTrappBot** | The agent's persona during development and testing. The current test bot (`@NewLobsterTrappBot`) replaced the earlier `Hum` handle on 2026-04-24 when the project moved to a secondary Telegram account. Name derives from "Hummer" (lobster, in German). Not a product name — each user's agent can develop its own identity. |
| **The Trifecta** | The three component repos working together inside the perimeter: openclaw-vault (containment) + clawhub-forge (skill security) + moltbook-pioneer (feed monitoring). |
| **OpenClaw** | The upstream open-source AI agent runtime we're securing. Not our project — we wrap it. Think of it as the engine; we build the safety cage around it. |
| **ClawHub** | The upstream skill registry for OpenClaw. 11.9% malware rate during the ClawHavoc incident. Skills from here are scanned by forge inside the perimeter before the agent can use them. |
| **Moltbook** | The upstream agent social network where AI agents post, follow, and interact. Content from here is scanned by pioneer inside the perimeter before the agent sees it. |
| **The containerized workshop Allegory** | The mental model for the architecture. OpenClaw agents are inmates (powerful but dangerous). The perimeter is the containerized workshop fence. Forge is the workshop. Pioneer is the monitoring station. The proxy is the gate. Claude Code is the warden. The GUI is the control panel. All untrusted content stays inside the fence. |

---

## Component Repos

| Term | Repo | Role | Container |
|------|------|------|-----------|
| **openclaw-vault** | `albertdobmeyer/openclaw-vault` | Runtime containment | vault-agent + vault-proxy |
| **clawhub-forge** | `albertdobmeyer/clawhub-forge` | Supply chain defense (scanner, CDR) | vault-forge |
| **moltbook-pioneer** | `albertdobmeyer/moltbook-pioneer` | Social intelligence (feed scanner) | vault-pioneer |
| **lobster-trapp** | `albertdobmeyer/lobster-trapp` | Parent app — GUI + perimeter orchestration | n/a (runs on host) |

---

## Security Terms

| Term | What It Means |
|------|--------------|
| **Six-Layer Defense** | The vault's defense-in-depth: (1) Container isolation, (2) Network proxy, (3) Tool policy, (4) Application restrictions, (5) Exec controls, (6) Hardening config. Each layer works independently. |
| **Tool Policy** | OpenClaw's built-in mechanism that filters which tools the LLM can see. Denied tools are removed BEFORE the LLM receives them. The agent literally cannot call a tool it doesn't know exists. |
| **Proxy Key Injection** | The core security innovation: the real API key lives only in vault-proxy. The agent has a placeholder. The proxy swaps the placeholder for the real key at the network layer. |
| **Kill Switch** | Three-level emergency stop: Soft (stop, preserve data), Hard (destroy containers and volumes), Nuclear (purge everything + remind to rotate API key). |
| **Pairing** | The process where a Telegram user proves their identity to the bot. Required after restarts in Hard Shell. |
| **ClawHavoc** | The incident where 11.9% of ClawHub skills (341/2,857) were malware. This is why ClawHub domains are blocked and why forge exists. |
| **CDR (Content Disarm & Reconstruction)** | Forge's core innovation: downloaded skills are quarantined, pre-filtered, semantically understood, then rebuilt from scratch. The original is never used. Binary: clean rebuild or discard. |
| **Quarantine** | The temporary directory inside the forge container where downloaded skills are held during CDR. Never touches the host. |
| **Clearance Report** | A JSON certificate generated after a skill passes the full pipeline (lint + scan + verify + test). Required by the vault for skill acceptance. |
| **Network Isolation** | The compose topology uses separate `internal: true` networks so containers cannot reach each other directly. Agent can't reach forge. Forge can't reach pioneer. Only the proxy bridges all networks. |

---

## Development Terms

| Term | What It Means |
|------|--------------|
| **Phase** | A stage in the implementation plan. Current plan: `.claude/plans/quirky-mixing-sunbeam.md` |
| **Spec** | A design specification. The v2 architecture spec: `docs/superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md` |
| **Verified Knowledge** | Information confirmed by reading actual source code. See `docs/openclaw-internals.md`. |
| **The Hallucination Problem** | When the LLM fabricates tool results instead of admitting it can't use denied tools. A trust problem for non-technical users. |

---

## Mapping: Old Terms → New Terms

| Old Term | New Term | Notes |
|----------|----------|-------|
| Gear 1: Manual | **Hard Shell** | Maximum lockdown |
| Gear 2: Semi-Auto | **Split Shell** | Selective capabilities |
| Gear 3: Full-Auto | **Soft Shell** | Broad autonomy |
| Gear switching | **Molting** | Reconfiguring the shell |
| Container isolation | **Exoskeleton** | The outer wall |
| Protected resources | **Driver Seat** | What the agent can never touch |
| Monorepo orchestrator | **Security infrastructure** | Lobster-TrApp's true identity |
| Dashboard | **Control panel** | The warden's interface |
| Host-side tools | **Perimeter containers** | Forge/pioneer now run inside containers |

**Important:** Update all specs, plans, code comments, and documentation to use current terminology. Old terms may appear in documents predating 2026-04-15.

---

*Last updated: 2026-04-15*
