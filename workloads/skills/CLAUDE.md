# OpenSkill Forge — Security Gatekeeper for the OpenClaw Skill Ecosystem

## What This Is

OpenSkill Forge is the **security gatekeeper** that ensures every skill entering or leaving a user's system is verified clean. It serves three roles:

| Role | What It Does |
|------|-------------|
| **Shield** | Downloads skills safely via Content Disarm & Reconstruction (CDR) — quarantine, scan, rebuild from intent, verify |
| **Anvil** | Helps users create new skills with AI assistance and automatic pipeline verification |
| **Stamp** | Publishes skills with security certificates proving they passed the full pipeline |

**Role in ecosystem**: `toolchain` — the supply chain defense layer that vets skills before they reach the agent runtime (`vault-agent`).

**Authoritative design document:** `docs/forge-identity-and-design.md` — the complete identity, feature spec, and handoff document.

## Target User

**Non-technical users** who run the OpenTrApp desktop app. They interact with forge through:
- The **OpenTrApp GUI** (primary) — buttons, wizards, status badges
- Their **OpenClaw agent** (secondary) — the agent assists with skill creation
- **Claude Code on the host** (power users) — direct CLI access to Makefile targets

The forge makes all security decisions FOR the user and presents clear pass/fail results.

## This Repo Is a OpenTrApp Component

This is the **skills workload** of the [opentrapp](https://github.com/albertdobmeyer/opentrapp) monorepo, at `workloads/skills/` ([ADR-0013](../../docs/adr/0013-monorepo-consolidation.md) dissolved the former `components/openagent-skills/` submodule). The `component.yml` here is the **manifest contract** that tells the OpenTrApp GUI how to discover, display, and control this component.

### Manifest Contract Rules
- `component.yml` must always parse as valid YAML
- `identity.id` must be `skills` (the GUI uses this as a stable key)
- `identity.role` must be `toolchain`
- Commands with `options_from` must have working commands (e.g., `ls skills/` must work from repo root)
- All `available_when` values must reference states declared in `status.states`
- Command IDs and health probe IDs must be unique

### Validating the Manifest
From the opentrapp root:
```bash
bash tests/orchestrator-check.sh    # Validates all manifests including this one
cargo test -p opentrapp          # Rust tests parse this manifest specifically
```

## Containerized Deployment (Perimeter Model)

In production, the skill firewall runs inside **vault-skills** — a dedicated container in the OpenTrApp five-container perimeter. All untrusted content (downloaded SKILL files) is processed inside this container, never on the user's host machine.

- **Containerfile** in this repo's root defines the image (~233MB, python:3.10-slim + bash toolchain)
- **vault-skills** is one of five services in `compose.yml` at the opentrapp root
- Runs on **skills-net** (internal network) — can reach vault-proxy but CANNOT reach vault-agent or vault-social
- Certified skills delivered to agent via **skills-deliveries** shared volume (write in forge, read-only in agent)
- Non-root user, capabilities dropped, 1GB memory limit
- HTTPS through vault-proxy (mitmproxy CA cert shared via environment)

| Context | How Forge Runs | When to Use |
|---------|---------------|-------------|
| **Development** | CLI/Makefile on host (`make scan`, `make test`) | Feature development, debugging, writing new patterns |
| **Production** | Inside vault-skills container via compose | Deployed perimeter, user-facing product |
| **Integration testing** | `podman compose up vault-skills` | Verifying container behavior and network isolation |

The CLI/Makefile usage documented below still applies for development. The Containerfile copies this repo and runs the same bash toolchain.

## Directory Structure

```
openagent-skills/
├── Makefile                      Single entry point for all commands (~35 targets)
├── component.yml                 MANIFEST — OpenTrApp contract
├── skills/                       25 published skills
├── tools/
│   ├── lib/
│   │   ├── common.sh             Colors, logging, skill discovery
│   │   ├── frontmatter.sh        YAML frontmatter parser + validator
│   │   ├── patterns.sh           87 malicious patterns (MITRE ATT&CK)
│   │   ├── line-classifier.sh    SAFE/SUSPICIOUS/MALICIOUS classifier
│   │   ├── trust-manifest.sh     .trust file + SHA-256 validation
│   │   └── sarif_formatter.py    SARIF 2.1.0 output formatter
│   ├── skill-lint.sh             Linter
│   ├── skill-scan.sh             Security scanner
│   ├── skill-verify.sh           Zero-trust verifier
│   ├── skill-test.sh             Test runner
│   ├── skill-new.sh              Scaffolder
│   ├── skill-publish.sh          Gated publisher
│   ├── skill-stats.sh            Adoption metrics
│   ├── registry-explore.sh       Registry browser
│   ├── workbench-verify.sh       12-point health check
│   └── pipeline-report.sh        Value summary
├── templates/                    Skill templates (cli-tool, workflow, language-ref)
├── tests/
│   ├── _framework/               Test runner + assertions
│   ├── scanner-self-test/        Scanner accuracy validation
│   └── *.test.sh                 25 test files (100% coverage)
├── .github/workflows/
│   └── skill-ci.yml              CI: lint → scan → test on PR
└── docs/                         Research reports, setup guides
```

## Key Makefile Commands

### Development
```bash
make new SKILL=name               # Scaffold new skill
make create                        # AI-assisted skill creation (interactive)
make create-noninteractive NAME=n TYPE=t DESC="d"  # AI skill creation (non-interactive, for GUI)
make lint                          # Lint all skills
make scan                          # Security scan all skills
make scan-one SKILL=name           # Scan single skill
make verify-skill SKILL=name       # Zero-trust verify single skill
make test                          # Run all tests (168+ assertions)
make publish SKILL=name VERSION=x  # Gated publish (lint→scan→test must pass)
```

### Analytics
```bash
make stats                         # Adoption metrics
make explore QUERY=term            # Browse ClawHub registry
make report                        # Pipeline value summary
```

### Verification
```bash
make verify                        # 12-point workbench health check
make check                         # Full pipeline: lint + scan + test
make self-test                     # Scanner self-test (known patterns)
```

## Commands Exposed to GUI (component.yml)

The manifest exposes 14 commands in 3 groups:

**Operations**: new-skill, lint, scan, verify, test, publish, lint-all, scan-all, verify-all
**Monitoring**: stats, explore, report
**Maintenance**: clean, setup

Several commands use `options_from` to dynamically populate dropdowns:
- `ls skills/` provides the skill picker for lint, scan, verify, test, publish
- `ls templates/` provides template options for new-skill
- Sort options for explore are static: `downloads`, `trending`, `installs`

## Security Pipeline

### Gated Publishing Flow (exists)
```
make new → make lint → make scan → make verify-skill → make test → make publish
                                                                        │
                                                            ALL must pass
```

### Content Disarm & Reconstruction (CDR)
```
Download → Quarantine → Pre-filter (87 patterns) → Isolated LLM extracts intent
    → Generator rebuilds clean SKILL.md → Post-verify → Deliver or Discard
```
The original downloaded file is NEVER accessible. Binary: clean rebuild or discard entirely.

### Scanning Capabilities
- 87 malicious patterns across 13 MITRE ATT&CK categories
- 16 prompt injection detection patterns for LLM manipulation
- Zero-trust line-by-line classification (SAFE / SUSPICIOUS / MALICIOUS)
- SARIF output for GitHub code scanning integration
- Post-install quarantine scan for newly downloaded skills

## Monorepo (no submodule sync)

This is the `workloads/skills/` workload in the opentrapp monorepo ([ADR-0013](../../docs/adr/0013-monorepo-consolidation.md)) — there is no submodule and no dual-copy sync; edit and commit here directly. The former standalone `openagent-skills` GitHub repo is archived; do not push there.

## Development Principles

1. **Security first** — this is a public security promise. Every line must uphold it.
2. **Spec before code** — every new feature requires a written spec before implementation.
3. **One task at a time** — always validate before moving to the next.
4. **CLI-first** — bash tools + Makefile targets. GUI wraps via component.yml.
5. **The original downloaded file is NEVER used** — binary: CDR rebuild or discard.

## What NOT to Do

- Do not change `identity.id` or `identity.role` in component.yml without coordinating with opentrapp
- Do not remove or rename command IDs that the GUI depends on — add new ones instead
- Do not modify `tools/lib/patterns.sh` without running `make self-test` to verify scanner accuracy
- Do not bypass the gated publish pipeline — `make publish` enforces lint→scan→test
- Do not add skills without tests — 100% coverage is the current standard
- Do not break the `ls skills/` command — the GUI uses it for dynamic dropdowns
