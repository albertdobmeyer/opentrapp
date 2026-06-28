# Glossary

Definitions of terms used in the OpenTrApp source, manifests, and documentation. Use these terms consistently across all repositories, UI text, and contributor discussions.

**Updated:** 2026-06-27

---

## 1. User-facing terms (frontend only)

The GUI — the React frontend served on-demand by the loopback `viewer-server` (de-Tauri, [ADR-0022](docs/adr/0022-daemon-control-surface.md)) — is the only surface non-technical users see. The mappings below define how internal developer concepts are presented in the GUI. **Developer terms must never appear in user-facing UI.**

| Developer term | User-facing term | Where used |
|---|---|---|
| `agent` (workload) | **My Assistant** | Sidebar, dashboard, component detail |
| `skills` (workload) | **Skills** / **Skill Store** | Sidebar, dashboard |
| `social` (workload) | **Agent Network** | Sidebar, dashboard (opt-in; build-out deferred) |
| Hard Shell | **Chat Only** | Mode descriptions |
| Split Shell | **Supervised** | Mode descriptions |
| Soft Shell | *(default — no mode label shown)* | Default user experience |
| Component | *(invisible)* | Never shown to users |
| Manifest / `component.yml` | *(invisible)* | Never shown to users |
| Container / Podman / Docker | **Secure sandbox** | Wizard, status messages |
| Perimeter | *(invisible)* | Never shown to users |
| Proxy / `vault-proxy` | *(invisible)* | Never shown to users |
| Workflow | **Action** | Button labels |
| Command (user tier) | **Quick action** | Button labels |
| Command (advanced tier) | *(behind "Developer Tools" toggle)* | Hidden by default |
| `compose.yml` | *(invisible)* | Never shown to users |
| Health probe | **Status badge** | Dashboard indicators |
| 24-point verification | **"Safe" / "Warning"** badge | Single badge with expandable detail |

---

## 2. Shell levels

Three privilege levels for the agent. Each level defines an allowed tool surface and a network allowlist. The default is Split Shell.

| Term | Definition | Allowed | Denied |
|---|---|---|---|
| **Hard Shell** | Maximum restriction; conversational mode only. | Telegram chat. | Command execution, file I/O, web access, skill loading. |
| **Split Shell** | Selective access with per-action approval. | Workspace file read/write, safelisted commands (each requires explicit approval). | Anything not on the safelist; arbitrary network fetches. |
| **Soft Shell** | Broad autonomy within fixed protections. | Web browsing, scheduled tasks, autonomous execution of safelisted commands, the broader OpenClaw tool surface. | Protected resources (root, SSH keys, credential stores, administrative accounts) — denied at every level, no exception. |

**Shell up** — increase restriction (e.g. Soft → Hard). Always permitted; takes effect immediately.

**Shell down** — reduce restriction (e.g. Hard → Split). Requires explicit user or coordinator approval; never automatic.

**Shell switch** — reconfiguring the active shell. Implementation: swaps tool-policy and proxy-allowlist configuration files and restarts the agent container with the new permission set.

---

## 3. Architecture terms

| Term | Definition |
|---|---|
| **Perimeter** | The five-container security boundary defined in `compose.yml`. All untrusted content (the agent process, skill files, fetched network content) stays inside the perimeter; nothing untrusted reaches the host filesystem. L7 (application-layer) policy lives in `vault-proxy`; L3 (network-layer) policy lives in `vault-egress` — see [ADR-0009](docs/adr/0009-five-container-perimeter.md). |
| **Container hardening** | The fixed set of OS-level restrictions applied to every perimeter container regardless of shell level: read-only root filesystem, all Linux capabilities dropped, custom seccomp profile, `noexec` mounts. Independent of shell level. The single exception is `vault-egress`, which holds `NET_ADMIN` (only) because it owns the L3 policy ruleset; it holds no secrets and runs no application code. |
| **Vault** | The complete runtime-containment package: container hardening + proxy egress filter + tool policy + shell configuration. The agent runtime lives in `workloads/agent/`; the L7+L3 egress chain lives in `infra/proxy/` + `infra/egress/`. |
| **Proxy** (`vault-proxy`) | The mitmproxy-based L7 egress gateway. Enforces the domain allowlist, injects API keys per request, performs a post-resolve destination-IP check (ADR-0009 Tier 2), and logs every transaction. Chains upstream to `vault-egress`; has no direct internet attachment. |
| **Egress** (`vault-egress`) | The L3 egress gateway. Drops outbound packets destined for RFC1918 / loopback / link-local / multicast / reserved ranges at the kernel level (nftables) and runs a pinned DoT resolver (Quad9 + Cloudflare) with a minimum-TTL cache. The only container with public-internet attachment. Holds `NET_ADMIN` but no API keys. Defined in [ADR-0009](docs/adr/0009-five-container-perimeter.md) and [ADR-0010](docs/adr/0010-pinned-resolver-dns.md). |
| **Protected resources** | Host-level resources that are denied at every shell level without exception: root, SSH keys, GPG keys, password stores and keyrings, administrative accounts, the Docker / Podman socket, and the perimeter's own configuration files. |
| **Allowlist** | The list of domains the proxy permits. Requests to any other host are rejected and logged. Each shell level has its own allowlist template. |
| **Placeholder key** | The dummy API-key string the agent container sees. The agent constructs API requests using the placeholder; `vault-proxy` substitutes the real key before forwarding. The agent never reads the literal value of any production credential. |

---

## 4. Trust tiers

| Term | Definition |
|---|---|
| **Tier 1** (trusted) | Components running on the user's host with full filesystem and network access: the user, the trusted CLI coordinator (Claude Code or equivalent), and the OpenTrApp desktop GUI. Tier 1 makes decisions and issues commands. |
| **Tier 2** (infrastructure) | The container perimeter. Enforces boundaries mechanically; does not make security decisions. Implemented by OpenTrApp's compose orchestration plus the five `vault-*` containers (`vault-agent`, `vault-skills`, `vault-social`, `vault-proxy`, `vault-egress`). |
| **Tier 3** (contained) | The OpenClaw agent process, Telegram gateway, loaded skills, and any fetched network content. Performs the work the user wants done, within the boundaries Tier 2 enforces. |
| **CLI coordinator** | The reasoning model running on the host (Claude Code, Anthropic's Opus, or an equivalent CLI agent) that translates user intent into perimeter operations, reads scanner results, adjusts shell level by context, and surfaces security events to the user in plain language. The coordinator decides; the perimeter enforces. |

---

## 5. Workflow terms

| Term | Definition |
|---|---|
| **Workflow** | A multi-step automated sequence declared in `component.yml` or `config/orchestrator-workflows.yml`. Chains individual commands into a single user-facing action. |
| **Component workflow** | A workflow that operates within a single component. Defined in that component's `component.yml`; references the component's own command IDs only. |
| **Orchestrator workflow** | A workflow that spans multiple components. Defined in `config/orchestrator-workflows.yml`; references component IDs plus command or workflow IDs. |
| **Manifest** | The `component.yml` file in each workload directory. Declares the workload's identity, status states, commands, configs, health probes, and workflows. |
| **Command** | A single declared operation a component exposes (e.g., forge's `scan`). Has an ID, an executable, an argument schema, and a danger level. |
| **Health probe** | A lightweight, repeatable command declared in the manifest that returns a status badge for the GUI dashboard. |

---

## 6. Workloads and infrastructure (monorepo since ADR-0013)

| Directory | Role | Container | Status |
|---|---|---|---|
| `app/`             | Desktop application + perimeter orchestrator | none (host) | Active |
| `workloads/agent/` | Runtime containment | `vault-agent` | Active |
| `workloads/skills/` | Supply-chain defense (scanner, CDR) | `vault-skills` | Active |
| `workloads/social/` | Agent-social-feed analysis | `vault-social` | **Opt-in / on-demand** — live AT Protocol adapter shipped (ADR-0017); full build-out deferred (MISSION Thread C / ADR-0024) |
| `infra/proxy/`     | L7 egress policy | `vault-proxy` | Active |
| `infra/egress/`    | L3 egress policy | `vault-egress` | Active |

---

## 7. Upstream terms

| Term | Definition |
|---|---|
| **OpenClaw** | The third-party autonomous AI agent runtime that OpenTrApp is designed to contain. Not a project of this repository; this software wraps it. |
| **ClawHub** | The third-party skill registry for OpenClaw. Skills downloaded from ClawHub are scanned by `vault-skills` before reaching the agent. |
| **Moltbook** | A third-party AI-agent social network. Acquired by Meta on 2026-03-10. Originally the data source for `vault-social`. |
| **ClawHavoc** | The 2026-Q1 study that classified 11.9 % of published ClawHub skills (341 of 2,857) as malicious. Cited as the empirical motivation for the supply-chain scanning layer. |

---

## 8. Security terms

| Term | Definition |
|---|---|
| **Defense-in-depth** | The layered-mitigation strategy used throughout the perimeter: each threat category (runtime, supply chain, network/social) is mitigated by multiple independent layers, so a single layer's failure does not produce an end-to-end compromise. Detailed tables in [`docs/trifecta.md`](docs/trifecta.md). |
| **Tool policy** | OpenClaw's built-in mechanism for filtering which tools the LLM is told about. Denied tools are removed from the catalog before the model receives it; the agent has no representation of a tool that has been denied. |
| **Proxy key injection** | The mechanism by which `vault-proxy` substitutes a placeholder string in outbound requests with the real API-key value. Implemented at the network layer. |
| **Kill switch** | The three-level emergency stop: graceful stop (preserves data), hard kill (destroys containers and volumes), and full perimeter teardown (purges all state and prompts the user to rotate the API key). |
| **Pairing** | The Telegram identity-verification step in which a chat counterpart proves their identity to the bot. Required after restart in Hard Shell. |
| **Content Disarm & Reconstruction (CDR)** | The supply-chain defense pattern used by `openagent-skills`: the original downloaded artifact is held in a quarantine volume, parsed for its semantic intent, and rebuilt from scratch. The original file is discarded; only the rebuilt artifact reaches the agent. |
| **Quarantine** | The temporary directory inside `vault-skills` where downloaded skills are held during scanning and reconstruction. Bound to the container; never reaches the host filesystem. |
| **Clearance report** | A signed JSON certificate generated after a skill passes the full pipeline (lint, scan, line verification, rebuild). Required by `vault-agent` before a skill is loaded. |
| **Network isolation** | The compose topology's use of separate `internal: true` Docker networks. Each `vault-*` container has its own internal network; only `vault-proxy` bridges them. `vault-agent` cannot reach `vault-skills` or `vault-social` directly. |

---

## 9. Historical term mapping

These older terms appear in pre-2026-04-15 documents and commit messages. Replace with the current term in any new work.

| Older term | Current term | Notes |
|---|---|---|
| Gear 1 (Manual) | Hard Shell | Maximum-restriction level |
| Gear 2 (Semi-Auto) | Split Shell | Selective-access level |
| Gear 3 (Full-Auto) | Soft Shell | Broad-autonomy level |
| Gear switching | Shell switching | The reconfiguration action |
| "Container isolation" (used as a layer name) | Container hardening | The fixed set of OS-level restrictions |
| Driver Seat | Protected resources | Resources denied at every shell level |
| Exoskeleton | Container hardening | Same concept, plain term |
| Monorepo orchestrator | Perimeter orchestrator | The role of the parent `opentrapp` repository |
| Dashboard | GUI control surface | The browser-viewer GUI projection (loopback `viewer-server`) |
| Warden | CLI coordinator | The trusted reasoning model on the host |
| The Trifecta | The three modules | Used informally to refer to vault + skills + social (the three concerns) collectively |
