# OpenTrApp — Contributor Guide

Project documentation for AI coding assistants and human contributors working in this repository.

For end-user documentation, see [`README.md`](README.md). For the full architecture, see [`docs/trifecta.md`](docs/trifecta.md). For terminology, see [`GLOSSARY.md`](GLOSSARY.md).

---

## 1. What this project is

**Product identity — north star ([ADR-0020](docs/adr/0020-product-identity-and-distribution.md)):**
OpenTrApp is a **registry-installable CLI/daemon orchestrator + signed container images** that runs an autonomous CLI agent inside a **five-container security perimeter** on the user's own machine — **operable by humans *and* by the user's host agent** (e.g. Claude Code) — with a web GUI and an optional MCP adapter as **thin, on-demand projections of the same manifest-driven daemon**. The daemon owns the perimeter; the projections come and go. The *external* operator orchestrates the *contained* agent, and **boundary-weakening operations stay human-gated regardless of who asks** (danger-gated control plane; ADR-0021, forthcoming). It is **not** a desktop app and **not** an MCP server for the *contained* agent.

**Current state (honest — the identity above is the TARGET, not yet fully built):** OpenTrApp ships *today* as a **Tauri 2 desktop application**. The perimeter-owning `opentrapp-core` / `opentrapp-daemon` are already Tauri-free ([ADR-0019](docs/adr/0019-headless-daemon-gui-viewer-split.md)), but the GUI is still a Tauri/GTK3 viewer and distribution is still native installers via GitHub Releases. The CLI-first / registry / MCP / de-Tauri direction is staged across ADR-0020 (identity), ADR-0021 (danger-gated control plane), and ADR-0022 (control surface / de-Tauri) — all forthcoming except 0020. **Do not document the CLI / registry / MCP / browser-viewer surfaces as existing until they land.** Concretely, **de-Tauri is NOT done**: the GUI is still a Tauri/GTK3 desktop bundle today; only `opentrapp-core` / `opentrapp-daemon` are Tauri-free.

**The mission (the *why*).** Enable as many people as possible to run **open-source CLI agents and agentic systems safely** on their own machines — whether they operate the agent themselves *or* delegate to a trusted vendor AI (e.g. Claude Code) — at the **highest standard** of autonomous-agent containment. This is bleeding-edge agentic security, built in the open, agent-agnostic, no lock-in. The bar is best-in-class, never good-enough. Stars and forks are *earned* by being the perimeter people actually trust: substance first, visibility follows (§12.4).

**The three areas of concern — composable, CLI-first ([ADR-0024](docs/adr/0024-product-structure-three-concerns.md)).** OpenTrApp is one brand decomposed into three architecturally-standalone concerns, each independently CLI-operable and independently distributable, plus an optional GUI:
- **Vault** — containerization / the perimeter. USP-1: privilege separation, no container holds both the API keys *and* the internet. This is the **virtual air-gap**: the contained agent can never reach the credentials, the host, or the network except through the proxy allowlist.
- **Skill** — toolset forging / the Skill Firewall. USP-2: anti-tamper scan + Content-Disarm-Reconstruction of skills, in isolation the agent cannot touch.
- **Social** — agent-to-agent social-feed vetting. Opt-in / on-demand; a live AT Protocol (Bluesky) adapter shipped (ADR-0017); full build-out deferred (ADR-0024).

The **GUI is optional and a pure projection of the CLI systems underneath** — never the entry point. You run what you need: the Skill Firewall alone vets a skill, the Vault alone contains an agent. One monorepo for *development* ([ADR-0013](docs/adr/0013-monorepo-consolidation.md)); three independently-*distributable* modules; one optional GUI *projection*. If a module ever earns standalone adoption it is extracted via an OCI image / package, never back into a git submodule.

**Three non-negotiable bars — catch drift early; push back on any change that erodes one:**
1. **Zero-trust air-gap.** It must stay *impossible* for the contained agent to read the credentials or reach the network off-allowlist, *even if the agent is fully compromised* — enforced fail-closed by `verify.sh` check 7 and boundary self-test B3. The contained agent is untrusted-by-design (Tier 3); we *contain* it, never trust it (see `docs/known-advisories.md`, "Trust-tier triage").
2. **Lean background app, the 7.2 GB floor.** The full perimeter and the T0 self-test must run on a 7.2 GB Linux laptop (verified 2026-06-22, cleaned of co-tenants); idle auto-pause collapses resting RAM toward zero. **If what we build does not run on this machine, we are failing.** Any change that breaks this floor is a regression, not a trade-off.
3. **Agentic, two-position, never an amplifier.** Operable by the user directly *or* by a trusted external operator (Claude Code) outside the cage, orchestrating the untrusted agent inside. Boundary-weakening always needs **out-of-band human confirmation regardless of who asks** ([ADR-0021](docs/adr/0021-danger-gated-agentic-control-plane.md)) — so a prompt-injected external agent cannot disarm the cage.

The application is the perimeter orchestrator: it composes the five containers, owns their lifetime, and exposes a manifest-driven GUI for user-facing operations. The L7 (application-layer) policy lives in `vault-proxy`; the L3 (network-layer) policy lives in `vault-egress`; see [ADR-0009](docs/adr/0009-five-container-perimeter.md).

The agent's reasoning is delegated to the agent's vendor API (Anthropic's, for OpenClaw). The agent's execution layer (file work, tool calls, skill loading) runs locally inside `vault-agent`. The application can never claim to make running an autonomous agent absolutely safe; it raises the cost of compromise via defense-in-depth and is open about the residual risks.

## 2. Repository layout

Post [ADR-0013](docs/adr/0013-monorepo-consolidation.md): single monorepo. 3 workloads
+ 2 infra + 1 orchestrator. Directory name matches container name 1:1.

```
opentrapp/                              (this repository — public, monorepo)
├── app/                                Tauri 2 + React 18 desktop GUI — OPTIONAL projection (the daemon owns the perimeter, ADR-0019); not the product
│   ├── src/                            React frontend
│   └── src-tauri/                      Rust backend (opentrapp-core / -daemon are Tauri-free)
├── workloads/                          one directory per workload container
│   ├── agent/                          → vault-agent       (Vault concern: runtime containment)
│   ├── skills/                         → vault-skills      (Skill concern: skill scanner + CDR)
│   └── social/                         → vault-social      (Social concern: opt-in / on-demand)
├── infra/                              shared infrastructure containers
│   ├── proxy/                          → vault-proxy       (L7 egress policy)
│   └── egress/                         → vault-egress      (L3 egress policy)
├── compose.yml                         five-service perimeter compose definition
├── schemas/
│   └── component.schema.json           manifest contract schema
├── config/
│   └── orchestrator-workflows.yml      cross-workload workflow definitions
├── tests/
│   └── orchestrator-check.sh           120-check validation suite
└── docs/
    ├── perimeter-explained.md          one-page elevator architecture
    ├── trifecta.md                     full architecture, threat model, defense layers
    ├── handoff.md                      current session-state documentation
    ├── adr/                            architecture decisions (current numbering: 0001–0015)
    └── archive/                        historical planning artifacts
```

### Workload status

| Workload | Directory | Container | Role | Status |
|----------|-----------|-----------|------|--------|
| Agent (Vault)  | `workloads/agent/`  | `vault-agent`  | Runtime containment | Active |
| Skill  | `workloads/skills/`  | `vault-skills`  | Supply-chain defense (skill scanner + CDR) | Active. Ships standalone (GitHub Action + Marketplace). |
| Social | `workloads/social/` | `vault-social` | Agent-social-feed analysis | Opt-in / on-demand. Original Moltbook target parked 2026-05-03; a live AT Protocol (Bluesky) adapter shipped (ADR-0017). Full build-out as a generalized agent-social shield is the **deferred** third concern (MISSION Thread C / ADR-0024 — after Vault/Skill/GUI). |

### Development vs. distribution (the modular model — do not flatten)

This is **one monorepo for development** ([ADR-0013](docs/adr/0013-monorepo-consolidation.md)): no submodule-coordination tax, one history. Architecturally it is **three standalone concerns** (Vault / Skill / Social), each independently **CLI-operable** and independently **distributable** (ADR-0014): the Skill Firewall already ships standalone as a GitHub Action + Marketplace listing. The **GUI is an optional projection** of those CLIs — never a fourth concern, never the entry point. Monorepo ≠ bundle: a user runs only the concern they need. If a concern ever earns standalone adoption it is extracted via an OCI image / package, never back into a git submodule.

## 3. UI rule (non-negotiable)

The frontend must never expose developer concepts to the end user. No containers, proxies, manifests, shell levels, or component IDs in user-visible text. The mappings between developer terms and user-facing terms are in [`GLOSSARY.md`](GLOSSARY.md) Section 1.

The 28-term ban is enforced by [`app/e2e/user-facing.spec.ts`](app/e2e/user-facing.spec.ts) on every commit. New developer-jargon terms encountered during contribution work must either be replaced with their user-facing mapping or added to the banned-terms array if they should never be surfaced to end users.

## 4. The manifest contract

Each component self-describes via `component.yml`. The Tauri backend reads these manifests at startup; the React frontend renders dashboards generically from them. The schema is at [`schemas/component.schema.json`](schemas/component.schema.json) and contains six sections:

1. **identity** — `id`, `name`, `version`, `role`, `icon`, `color`
2. **status** — declared states and the probe commands that distinguish them
3. **commands** — individual operations with argument schemas, danger levels, output formats
4. **configs** — editable configuration files with format metadata
5. **health** — lightweight probes for dashboard badges
6. **workflows** — multi-step automated sequences (chains of commands presented to the user as a single action)

### Workflow types

- **Component workflows** — declared inside a single `component.yml`; reference command IDs within that component only
- **Orchestrator workflows** — declared in [`config/orchestrator-workflows.yml`](config/orchestrator-workflows.yml); reference component IDs plus command or workflow IDs across components

### Schema-alignment requirement

The schema is implemented in three places that must stay in sync:

- `schemas/component.schema.json` (source of truth)
- `app/src-tauri/crates/core/src/orchestrator/manifest.rs` (Rust serde structs)
- `app/src/lib/types.ts` (TypeScript types)

Enum-value alignment is verified by [`tests/orchestrator-check.sh`](tests/orchestrator-check.sh) section 7. Cross-references (commands referenced from workflows, states referenced from `available_when`, orchestrator workflow steps referencing component commands) are also validated.

## 5. The "generic backend" constraint

The Tauri backend reads manifests and executes what they declare. It must not contain component-specific logic. The backend's responsibilities are:

- Workflow execution — running multi-step sequences from manifest definitions
- Container management — building, starting, stopping containers via compose
- Cross-component routing — directing workflow steps to the correct component

The backend knows *how* to execute workflows generically; it does not know *what* any specific component does internally.

## 6. Key files

| Purpose | File |
|---------|------|
| Manifest schema (source of truth) | [`schemas/component.schema.json`](schemas/component.schema.json) |
| Rust manifest structs | [`app/src-tauri/crates/core/src/orchestrator/manifest.rs`](app/src-tauri/crates/core/src/orchestrator/manifest.rs) |
| TypeScript types | [`app/src/lib/types.ts`](app/src/lib/types.ts) |
| Tauri command handlers | [`app/src-tauri/src/commands/`](app/src-tauri/src/commands/) |
| Tauri invoke wrappers | [`app/src/lib/tauri.ts`](app/src/lib/tauri.ts) |
| React hooks | [`app/src/hooks/`](app/src/hooks/) |
| Perimeter compose | [`compose.yml`](compose.yml) |
| Orchestrator workflows | [`config/orchestrator-workflows.yml`](config/orchestrator-workflows.yml) |
| Orchestration tests | [`tests/orchestrator-check.sh`](tests/orchestrator-check.sh) |
| Rust orchestrator unit tests | [`app/src-tauri/crates/core/src/orchestrator/tests.rs`](app/src-tauri/crates/core/src/orchestrator/tests.rs) |
| Architecture (this repository) | [`docs/trifecta.md`](docs/trifecta.md) |
| Threat model | [`docs/threat-model.md`](docs/threat-model.md) |
| Prior-art comparison | [`docs/why-not-x.md`](docs/why-not-x.md) |
| Reproducibility recipe | [`docs/reproduce.md`](docs/reproduce.md) + [`docs/reproduce.sh`](docs/reproduce.sh) |
| Mermaid architecture diagrams | [`docs/diagrams.md`](docs/diagrams.md) |
| Architecture decisions (ADRs) | [`docs/adr/`](docs/adr/) — 15 records covering proxy-side credentials, adaptive shells, CDR, social-workload parking, deserve-to-exist, four-container topology (superseded by 0009), manifest-driven backend, Tauri, five-container L7/L3 split, pinned-resolver DNS, zero-trust bootstrap, subscription-OAuth feasibility, monorepo consolidation (0013), modular distribution + `openagent-*` naming (0014), and local-AI judgment layer / Sentinel (0015) |
| Whitepaper | [`docs/whitepaper.md`](docs/whitepaper.md) |
| Architecture v2 design spec (historical, supersded by `docs/trifecta.md`) | [`docs/archive/superpowers/2026-04-15-architecture-v2-perimeter-redesign.md`](docs/archive/superpowers/2026-04-15-architecture-v2-perimeter-redesign.md) |

## 7. Build and test

```bash
# Rust backend
cd app/src-tauri && cargo build
cd app/src-tauri && cargo test --lib    # 56 tests at v0.3.0

# Frontend
cd app && npm install
cd app && npm run lint                  # eslint --max-warnings 0 (CI GATE — must be clean)
cd app && npm test -- --run             # vitest
cd app && npx tsc --noEmit              # TypeScript strict
cd app && npx playwright test           # end-to-end, 25 tests
cd app && npm run dev                   # Vite dev server

# Manifest and orchestration validation
bash tests/orchestrator-check.sh        # must report 0 warnings
bash tests/integration-test.sh --ci     # cross-module contracts (CI GATE — 0 failures)

# NOTE: `npm run lint` and `integration-test.sh` are CI gate jobs in ci.yml —
# a local "green" that omits them does NOT mean CI is green. Run the full set.

# Container perimeter (smoke)
podman compose up -d                    # start all five containers
podman compose down                     # stop perimeter
```

### What the orchestration check validates

1. Repository structure (directories, essential files)
2. JSON Schema validity (six sections)
3. All workload manifests parse, valid identity, cross-references, enums
4. Build artifacts (`Cargo.toml`, `tauri.conf.json`, `package.json`, `tsconfig.json`)
5. Frontend-backend contract: every Rust command handler has a matching frontend invoke wrapper
6. Manifest enum values match Rust serde expectations
7. Prerequisites cross-references valid
8. Workflow step → command references valid; orchestrator workflow references valid

## 8. Working in the monorepo

Post [ADR-0013](docs/adr/0013-monorepo-consolidation.md): no submodules. Every workload
and infra container lives in this repository. Edit, build, and commit in one place.

The earlier three-submodule layout (`components/{opencli-container,openagent-skills,openagent-social}/`)
was consolidated 2026-05-30 because the lifecycle test failed — the submodules co-shipped
in lockstep with the parent and had zero external consumers. Three archived GitHub repos
exist as a historical reference; do not push to them.

## 9. Security considerations

- **Command injection prevention** — `app/src-tauri/crates/core/src/orchestrator/runner.rs` wraps all interpolated arguments in single quotes with shell escaping.
- **Path traversal protection** — `app/src-tauri/crates/core/src/config_ops.rs` (`read_within` / `write_within`) validates that canonical paths stay within the component's directory (TOCTOU-safe via the canonical path); the Tauri `commands/config.rs` shim and the future loopback web route both call it. Traversal-rejection tests live in that module.
- **Regex in probes** — `app/src-tauri/crates/core/src/status.rs` uses the `regex` crate for `stdout_regex` rules; never shells out to grep (the `commands/status.rs` shim delegates here).
- **Stream deduplication** — `app/src-tauri/src/commands/stream.rs` kills any prior streaming process before starting a new one.
- **Network isolation** — compose networks are `internal: true`; no default gateway. Containers cannot reach the public internet directly; only `vault-proxy` can.
- **No untrusted content on the host** — all skill downloads, scanning, and feed processing happen inside containers.

## 10. Constraints

The application must not:

- Contain workload-specific logic in Rust or React (the generic-backend constraint)
- Duplicate domain logic that belongs in a workload directory (`workloads/<name>/`)
- Run AI models or agent code directly
- Expose network services (no remote-management surface)
- Process untrusted content on the host filesystem

When contributing:

- Do not add workload-specific logic to the Tauri backend; it must remain generic
- Workload code lives under `workloads/<name>/`; infra container code lives under `infra/<name>/`
- Do not change the manifest schema without updating all three alignment layers (`schemas/component.schema.json`, `manifest.rs`, `types.ts`)
- Do not commit `node_modules/`, `target/`, `app/src-tauri/gen/`, or `.env` (covered by `.gitignore`)

## 11. Verification discipline (non-negotiable)

A claim is verified at the end that **consumes** the output, not the end that produces it. Building, compiling, writing a file, or starting a process is the cheap, misleading end; the real test is the consumer.

- **Verify at the consumption end.** "Done" means the thing that consumes the output is confirmed correct — the perimeter actually reads the credentials, the agent receives the message exactly once, the resumed boundary actually blocks egress. A green at the producing end (it built, the process is up, the workflow ran) is necessary, never sufficient. Examples that bit us: keys "saved" to a file the perimeter never reads (the v0.6 first-run dead-end); a local "green" that skipped the CI gate jobs in §7.
- **Verification gates dependent work — never run them in parallel.** Do not sequence a dependent step (publish, a narrative, the next feature) alongside the verification it rests on. The dependent step starts only after its gate is green. Asserting the dependent step is ready while its gate is unverified is the failure mode to avoid.
- **Gate the claim, not the workstream.** Don't hold a finished, shippable thing hostage to an unfinished optimization — instead, scope what you *assert* to what's verified. A release can ship before an optimization lands; its copy just may not claim the unverified property. Block the claim, release the work.
- **Unverifiable ≠ verified.** When the consuming end cannot be exercised here (e.g., a long-soak or sustained-load test the available hardware or CI cannot sustain), the claim is **unverified, not done**. Say so explicitly; route the check to capable hardware or CI rather than asserting it. (Note: the full perimeter and the T0 boundary self-test **do** run on the 7.2 GB dev laptop when it is cleaned of heavy apps — verified 2026-06-16 and 2026-06-22; the swap-storm is co-tenancy with the IDE/browser, not the perimeter.)
- **For security boundaries, "running" ≠ "correct."** A perimeter that is rebuilt or resumed (e.g. after idle auto-pause) must pass the **same** boundary self-tests as a fresh cold start before it is reported healthy — network isolation holds, credentials inject, the allowlist is loaded, the proxy CA is unchanged, the L3 egress filter is active. Any failure holds **fail-closed** and alerts. A boundary that is "alive but subtly wrong" is worse than a visible failure, because the breach is silent.

## 12. The bar (non-negotiable)

OpenTrApp is a high-stakes security application. It is public and already being forked, so the maintainer's reputation rides on every change. This bar overrides convenience, speed, and a green checkmark.

1. **End-user-faithful tests only.** Verify through the product's own entrypoints, the OpenTrApp daemon CLI (`opentrapp-daemon vault <verb>`, the CLI-first end-user surface per ADR-0020) or the app, never through dev scaffolding. `make perimeter-up`, `podman-compose`, and the `make` targets are local conveniences; a pass there does not prove the shipped path. Reproduce what a user actually runs, then verify at the consumption end (§11). A test that exercises a path no user takes is not evidence.
2. **Guard the reputation: highest quality, no glossing, no handwaving.** No quick fixes, no patching to force a passing run, no working around a failure, no glossing over a gap. Find the root cause and fix it properly, or state plainly that it is unverified and route it (§11: unverified is not done). For a security tool, reporting "it looks fine" as "it is correct" is the failure that ends trust.
3. **Protect the end user from agent-related dangers, always.** Judge every change first by its containment effect: does it keep the contained agent caged, keep credentials and the network separated, keep untrusted skill content off the host, keep boundary-weakening operations human-gated (ADR-0021)? When convenience and the user's safety conflict, safety wins and the trade is documented.
4. **Build something genuinely useful, and earn the stars and forks honestly.** The aim is a real, trustworthy security perimeter for open agent systems (OpenClaw, opencode, and others) that people adopt because it actually protects them. Substance first; visibility follows. Never let promotion outrun the verified foundation.
