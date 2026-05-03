# Lobster-TrApp — Your Personal AI Assistant, Safe on Any Computer

## Product Identity (Northstar)

**Lobster-TrApp is the safe front door to the OpenClaw ecosystem for non-technical users.**

The product is NOT a security dashboard. The security is invisible infrastructure — like a lock on a front door. The user walks through the door and uses their AI assistant. They never think about the lock.

- **What users see:** "My personal AI assistant, controlled from Telegram"
- **What we build:** A 4-container security perimeter that makes this safe
- **Northstar:** "Your own personal AI assistant, safe on any computer"
- **Design spec:** `docs/specs/2026-04-19-product-identity-spec.md`

**UI Rule:** The frontend (this parent repo) must NEVER expose developer concepts to end users. No containers, proxies, manifests, shell levels, or component IDs in user-facing text. Map developer terms to user terms (see GLOSSARY.md "User-Facing Terms").

## What This Is

Lobster-TrApp is a **desktop GUI** that lets non-technical users safely run OpenClaw (an autonomous AI agent) on their personal computer. Behind a friendly interface, it wraps the entire OpenClaw ecosystem in a containerized perimeter where all untrusted content — the agent itself, downloaded skills, social network feeds — is processed inside hardened containers, never on the user's host machine.

**You are working in the parent repo (the frontend).** This repo bundles three backend component repos as submodules, provides the Tauri desktop GUI, defines the manifest contract, and manages the container perimeter. The submodules are invisible infrastructure — the user never sees them.

## What the User Experiences

A non-technical user downloads Lobster-TrApp, enters an API key, connects Telegram, and gets a safe AI assistant they can talk to from their phone. The security is invisible — they don't know about containers, proxies, or shell levels. They just have an assistant that works.

Behind the scenes, Lobster-TrApp manages a 4-container perimeter that air-gaps the dangerous AI agent from the user's system while letting it do useful work. The **dynamic shell** adjusts restrictions based on context, and an intelligent warden (Claude Code / Opus) makes security decisions on behalf of the user.

## The Security Model

Think of this as a contained productivity  system. The OpenClaw agents are inmates — powerful but dangerous. The perimeter is the containerized workshop:

- **vault-agent** (cell block) — where the agent runs, heavily restricted
- **vault-forge** (workshop) — where untrusted SKILL files are scanned and rebuilt, inside the fence
- **vault-pioneer** (monitoring station) — where untrusted social content is analyzed, inside the fence
- **vault-proxy** (the gate) — the only door in/out, holds API keys, enforces allowlists

Nothing untrusted ever touches the user's host. See `docs/trifecta.md` for the full security model.

### Multi-Agent Trust Chain

```
TIER 1: TRUSTED — human + Claude Code (warden, host access, makes decisions)
TIER 2: INFRASTRUCTURE — Lobster-TrApp (4 containers, enforces boundaries)
TIER 3: CONTAINED — OpenClaw agents (do the work, within boundaries)
```

## Architecture

```
lobster-trapp/                    (this repo — public)
├── components/
│   ├── openclaw-vault/           git submodule — container + proxy
│   ├── clawhub-forge/            git submodule — containerized scanner
│   └── moltbook-pioneer/         git submodule — containerized feed monitor
├── app/                          Tauri 2 + React 18 desktop GUI
│   ├── src/                      React frontend
│   └── src-tauri/                Rust backend
├── compose.yml                   4-service perimeter (agent + forge + pioneer + proxy)
├── schemas/
│   └── component.schema.json     THE CONTRACT — all manifests conform to this
├── config/
│   └── orchestrator-workflows.yml  Cross-component workflow definitions
├── tests/
│   └── orchestrator-check.sh     41-check validation suite
└── docs/
    ├── trifecta.md               How the three modules work together
    └── handoff.md                Current state and next steps
```

### Component Roles
| Component | Role | Container | Status |
|-----------|------|-----------|--------|
| openclaw-vault | `runtime` — agent containment | vault-agent + vault-proxy | Active |
| clawhub-forge | `toolchain` — skill security scanner | vault-forge | Active |
| moltbook-pioneer | `network` — social feed monitor | vault-pioneer | Active (API deferred) |

## The Manifest Contract

Each component self-describes via `component.yml`. The GUI discovers these and renders dashboards generically. The contract is defined in `schemas/component.schema.json` with 6 sections:

1. **identity** — id, name, version, role, icon, color
2. **status** — declared states + probe commands
3. **commands** — individual operations with args, danger levels, output formats
4. **configs** — editable config files with format metadata
5. **health** — lightweight probes for dashboard badges
6. **workflows** — multi-step automated sequences (chains commands into user-facing actions)

### Workflows

Workflows are the bridge between the manifest-driven architecture and non-technical users. Instead of clicking individual commands, users trigger workflows that execute multi-step pipelines automatically:

- **Component workflows** (in each `component.yml`): vet-skill, safe-download, secure-start, etc.
- **Orchestrator workflows** (in `config/orchestrator-workflows.yml`): install-skill (forge.scan → vault.install), first-run-setup, full-audit

### Rules for the Contract
- **Never change enum values** without updating: schema JSON, Rust `manifest.rs`, TypeScript `types.ts`
- **Enum alignment is tested** by `tests/orchestrator-check.sh` section 7
- **Cross-references are validated**: commands, states, workflow steps, orchestrator component refs
- **Workflow steps must reference valid command IDs** within the same component

## The Generic Architecture Constraint

The Tauri backend must remain **generic** — it reads manifests and executes what they declare. It does not contain component-specific logic. However, it does understand:

- **Workflow execution** — how to run multi-step sequences from workflow definitions
- **Container management** — how to build, start, stop containers via compose
- **Cross-component workflows** — how to route steps to the correct component

The distinction: the app knows HOW to execute workflows generically. It does NOT know WHAT any specific component does internally.

## Key Files

| Purpose | File |
|---------|------|
| Manifest schema (contract) | `schemas/component.schema.json` |
| Rust manifest structs | `app/src-tauri/src/orchestrator/manifest.rs` |
| TypeScript types | `app/src/lib/types.ts` |
| Tauri command handlers | `app/src-tauri/src/commands/*.rs` |
| Tauri invoke wrappers | `app/src/lib/tauri.ts` |
| React hooks | `app/src/hooks/*.ts` |
| Perimeter compose | `compose.yml` |
| Orchestrator workflows | `config/orchestrator-workflows.yml` |
| Orchestration tests | `tests/orchestrator-check.sh` |
| Rust unit tests | `app/src-tauri/src/orchestrator/tests.rs` |
| Architecture spec | `docs/superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md` |
| Module relationships | `docs/trifecta.md` |

## Commands

### Build & Test
```bash
# Rust backend
cd app/src-tauri && cargo build
cd app/src-tauri && cargo test

# Frontend
cd app && npm install
cd app && npm run dev                   # Dev server (Vite)

# Full orchestration validation (41 checks)
bash tests/orchestrator-check.sh

# Container perimeter
podman compose up -d                    # Start all 4 containers
podman compose down                     # Stop perimeter
```

### What the Tests Validate
1. Repository structure (directories, essential files)
2. JSON Schema validity (6 sections)
3. All component manifests parse, valid identity, cross-references, enums
4. Submodule synchronization status
5. Build artifacts (Cargo.toml, tauri.conf.json, package.json, tsconfig)
6. Frontend-backend contract (Rust handlers match frontend invoke calls)
7. Manifest enum values match Rust serde expectations
8. Prerequisites cross-references valid
9. Workflow step→command references valid, orchestrator workflow references valid

## Submodule Discipline

### The Dual-Copy Problem
Each component exists in TWO places:
- **Standalone clone**: `~/<component>/` (for focused development)
- **Submodule copy**: `~/lobster-trapp/components/<component>/` (for orchestrator integration)

These are independent git checkouts. Changes in one do NOT automatically appear in the other.

### Sync Workflow
After making changes in a standalone clone:
```bash
cd components/<component>
git pull
cd ../..
git add components/<component>
git commit -m "Update <component> submodule reference"
```

## Security Considerations

- **Command injection prevention**: `runner.rs` wraps all interpolated args in single quotes with escaping
- **Path traversal protection**: `config.rs` validates canonical paths stay within component directory
- **Regex in probes**: `status.rs` uses the `regex` crate for `stdout_regex` rules
- **Stream deduplication**: `stream.rs` kills old processes before starting new streams
- **Network isolation**: compose networks are `internal: true` — no default gateway, containers can't reach the internet directly
- **Zero untrusted content on host**: all downloads, scanning, and feed processing happen inside containers

## What the App Must NEVER Do

- Contain component-specific logic in Rust or React (the generic constraint)
- Duplicate domain logic that belongs in a submodule
- Run AI models or agent code directly
- Expose network services (no remote access)
- Process untrusted content on the host (skills, feeds — always inside containers)

## What NOT to Do

- Do not add component-specific logic to the Tauri backend — it must remain generic
- Do not modify `component.yml` files in submodules without also pushing to the component's own remote
- Do not change the schema without updating all three alignment layers (schema JSON, Rust structs, TS types)
- Do not commit `node_modules/`, `target/`, `app/src-tauri/gen/`, or `.env` (covered by .gitignore)
- Do not force-push submodule references — this breaks other clones
