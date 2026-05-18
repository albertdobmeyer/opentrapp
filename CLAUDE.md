# OpenTrApp — Contributor Guide

Project documentation for AI coding assistants and human contributors working in this repository.

For end-user documentation, see [`README.md`](README.md). For the full architecture, see [`docs/trifecta.md`](docs/trifecta.md). For terminology, see [`GLOSSARY.md`](GLOSSARY.md).

---

## 1. What this project is

OpenTrApp is a Tauri 2 desktop application that runs an autonomous CLI agent inside a four-container security perimeter on the user's own computer. The reference integration is OpenClaw; the architecture is designed to extend to other CLI agents. The application is the perimeter orchestrator: it composes the four containers, owns their lifetime, and exposes a manifest-driven GUI for user-facing operations.

The agent's reasoning is delegated to the agent's vendor API (Anthropic's, for OpenClaw). The agent's execution layer (file work, tool calls, skill loading) runs locally inside `vault-agent`. The application can never claim to make running an autonomous agent absolutely safe; it raises the cost of compromise via defense-in-depth and is open about the residual risks.

## 2. Repository layout

```
opentrapp/                          (this repository — public)
├── components/
│   ├── opencli-container/                 git submodule — runtime containment (vault-agent + vault-proxy)
│   ├── openskill-forge/                  git submodule — supply-chain defense (vault-forge)
│   └── openagent-social/               git submodule — social-content analysis (vault-pioneer); parked
├── app/                                Tauri 2 + React 18 desktop application
│   ├── src/                            React frontend
│   └── src-tauri/                      Rust backend
├── compose.yml                         four-service perimeter compose definition
├── schemas/
│   └── component.schema.json           manifest contract schema
├── config/
│   └── orchestrator-workflows.yml      cross-component workflow definitions
├── tests/
│   └── orchestrator-check.sh           42-check validation suite
└── docs/
    ├── trifecta.md                     architecture, threat model, defense layers
    ├── handoff.md                      current session-state documentation
    └── archive/                        historical planning artifacts
```

### Component status

| Component | Role | Container | Status |
|-----------|------|-----------|--------|
| `opencli-container` | runtime containment | `vault-agent` + `vault-proxy` | Active |
| `openskill-forge` | supply-chain defense | `vault-forge` | Active |
| `openagent-social` | social-content analysis | `vault-pioneer` | Parked since 2026-05-03 (Moltbook acquired by Meta 2026-03-10; API intermittent since 2026-04-05) |

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
- `app/src-tauri/src/orchestrator/manifest.rs` (Rust serde structs)
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
| Rust manifest structs | [`app/src-tauri/src/orchestrator/manifest.rs`](app/src-tauri/src/orchestrator/manifest.rs) |
| TypeScript types | [`app/src/lib/types.ts`](app/src/lib/types.ts) |
| Tauri command handlers | [`app/src-tauri/src/commands/`](app/src-tauri/src/commands/) |
| Tauri invoke wrappers | [`app/src/lib/tauri.ts`](app/src/lib/tauri.ts) |
| React hooks | [`app/src/hooks/`](app/src/hooks/) |
| Perimeter compose | [`compose.yml`](compose.yml) |
| Orchestrator workflows | [`config/orchestrator-workflows.yml`](config/orchestrator-workflows.yml) |
| Orchestration tests | [`tests/orchestrator-check.sh`](tests/orchestrator-check.sh) |
| Rust orchestrator unit tests | [`app/src-tauri/src/orchestrator/tests.rs`](app/src-tauri/src/orchestrator/tests.rs) |
| Architecture (this repository) | [`docs/trifecta.md`](docs/trifecta.md) |
| Threat model | [`docs/threat-model.md`](docs/threat-model.md) |
| Prior-art comparison | [`docs/why-not-x.md`](docs/why-not-x.md) |
| Reproducibility recipe | [`docs/reproduce.md`](docs/reproduce.md) + [`docs/reproduce.sh`](docs/reproduce.sh) |
| Mermaid architecture diagrams | [`docs/diagrams.md`](docs/diagrams.md) |
| Architecture decisions (ADRs) | [`docs/adr/`](docs/adr/) — eight records covering proxy-side credentials, adaptive shells, CDR, pioneer parking, deserve-to-exist, four-container topology, manifest-driven backend, Tauri |
| Whitepaper | [`docs/whitepaper.md`](docs/whitepaper.md) |
| Architecture v2 design spec (historical, supersded by `docs/trifecta.md`) | [`docs/archive/superpowers/2026-04-15-architecture-v2-perimeter-redesign.md`](docs/archive/superpowers/2026-04-15-architecture-v2-perimeter-redesign.md) |

## 7. Build and test

```bash
# Rust backend
cd app/src-tauri && cargo build
cd app/src-tauri && cargo test --lib    # 56 tests at v0.3.0

# Frontend
cd app && npm install
cd app && npm test -- --run             # vitest, 74 tests at v0.3.0
cd app && npx tsc --noEmit              # TypeScript strict
cd app && npx playwright test           # end-to-end, 25 tests
cd app && npm run dev                   # Vite dev server

# Manifest and orchestration validation
bash tests/orchestrator-check.sh        # 42 checks, must report 0 warnings

# Container perimeter (smoke)
podman compose up -d                    # start all four containers
podman compose down                     # stop perimeter
```

### What the orchestration check validates

1. Repository structure (directories, essential files)
2. JSON Schema validity (six sections)
3. All component manifests parse, valid identity, cross-references, enums
4. Submodule synchronization status
5. Build artifacts (`Cargo.toml`, `tauri.conf.json`, `package.json`, `tsconfig.json`)
6. Frontend-backend contract: every Rust command handler has a matching frontend invoke wrapper
7. Manifest enum values match Rust serde expectations
8. Prerequisites cross-references valid
9. Workflow step → command references valid; orchestrator workflow references valid

## 8. Submodule discipline

Each component exists in two places on a contributor's machine:

- **Standalone clone:** `~/<component>/` (focused development on one component)
- **Submodule copy:** `~/opentrapp/components/<component>/` (orchestrator integration)

These are independent git checkouts. Changes in one do not propagate to the other automatically.

### Sync workflow after a submodule change

```bash
cd components/<component>
git pull
cd ../..
git add components/<component>
git commit -m "Update <component> submodule reference"
```

## 9. Security considerations

- **Command injection prevention** — `app/src-tauri/src/orchestrator/runner.rs` wraps all interpolated arguments in single quotes with shell escaping.
- **Path traversal protection** — `app/src-tauri/src/commands/config.rs` validates that canonical paths stay within the component's directory.
- **Regex in probes** — `app/src-tauri/src/commands/status.rs` uses the `regex` crate for `stdout_regex` rules; never shells out to grep.
- **Stream deduplication** — `app/src-tauri/src/commands/stream.rs` kills any prior streaming process before starting a new one.
- **Network isolation** — compose networks are `internal: true`; no default gateway. Containers cannot reach the public internet directly; only `vault-proxy` can.
- **No untrusted content on the host** — all skill downloads, scanning, and feed processing happen inside containers.

## 10. Constraints

The application must not:

- Contain component-specific logic in Rust or React (the generic-backend constraint)
- Duplicate domain logic that belongs in a submodule
- Run AI models or agent code directly
- Expose network services (no remote-management surface)
- Process untrusted content on the host filesystem

When contributing:

- Do not add component-specific logic to the Tauri backend; it must remain generic
- Do not modify `component.yml` files in submodules without also pushing the change to the component's own remote
- Do not change the manifest schema without updating all three alignment layers (`schemas/component.schema.json`, `manifest.rs`, `types.ts`)
- Do not commit `node_modules/`, `target/`, `app/src-tauri/gen/`, or `.env` (covered by `.gitignore`)
- Do not force-push submodule references; this breaks other clones
