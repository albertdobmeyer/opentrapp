# vault-social — Agent-Social-Feed Vetting (the Social concern; opt-in / on-demand)

## What This Is

`vault-social` is the **Social concern** of OpenTrApp: generalized vetting of untrusted **agent-social feeds** for prompt injection before any of it reaches the contained agent — the same "untrusted input: sanitize, don't trust" pattern as the Skill Firewall, applied to social feeds. A live **AT Protocol (Bluesky) adapter** shipped ([ADR-0017](../../docs/adr/0017-unpark-social-live-adapter.md)); the original Moltbook target was parked. It is **opt-in / off by default**; full build-out as a generalized agent-social shield is the **deferred** third concern, after Vault / Skill / GUI ([ADR-0024](../../docs/adr/0024-product-structure-three-concerns.md)).

**Role in ecosystem**: `network` — the agent-social vetting layer (manifest `identity.role: network`).

## This Is a OpenTrApp Workload

Post [ADR-0013](../../docs/adr/0013-monorepo-consolidation.md) this lives **in the opentrapp monorepo** at `workloads/social/` — **not** a git submodule (the old `components/openagent-social/` layout was dissolved 2026-05-30; `components/` no longer exists). It ships as the independently-installable `openagent-social` distribution (ADR-0014). The `component.yml` here is the **manifest contract** that tells the OpenTrApp daemon/GUI how to discover, display, and control this workload.

### Manifest Contract Rules
- `component.yml` must always parse as valid YAML
- `identity.id` must be `social` (the daemon/GUI uses this as a stable key)
- `identity.role` must be `network`
- All `available_when` values must reference states declared in `status.states`
- Command IDs and health probe IDs must be unique

### Validating the Manifest
From the opentrapp root:
```bash
bash tests/orchestrator-check.sh    # Validates all manifests including this one
cargo test -p opentrapp          # Rust tests parse this manifest specifically
```

## Containerized Deployment (Perimeter Model)

In production, the feed vetter runs inside **vault-social** — a dedicated container in the OpenTrApp five-container perimeter (opt-in / off by default). All untrusted content (agent-social feed data) is processed inside this container, never on the user's host machine.

- **Containerfile** in this repo's root defines the image (~153MB, python:3.10-slim)
- **vault-social** is one of the five services in `compose.yml` at the opentrapp root (opt-in / off by default)
- Runs on **social-net** (internal network) — can reach vault-proxy but CANNOT reach vault-agent or vault-skills
- Agent never sees unfiltered feed data — scanning and filtering happen inside the container
- Non-root user, capabilities dropped, 512MB memory limit
- HTTPS through vault-proxy (mitmproxy CA cert shared via environment)

| Context | How Pioneer Runs | When to Use |
|---------|-----------------|-------------|
| **Development** | CLI/Makefile on host (`make scan`, `make test`) | Feature development, adding injection patterns |
| **Production** | Inside vault-social container via compose | Deployed perimeter, user-facing product |
| **Integration testing** | `podman compose up vault-social` | Verifying container behavior and network isolation |

The CLI/Makefile usage documented below still applies for development. The Containerfile copies this repo and runs the same tools.

**Current status:** Opt-in / on-demand. A live AT Protocol (Bluesky) adapter shipped ([ADR-0017](../../docs/adr/0017-unpark-social-live-adapter.md)); the original Moltbook target was parked. Full build-out as a generalized agent-social shield is deferred until Vault / Skill / GUI are closed ([ADR-0024](../../docs/adr/0024-product-structure-three-concerns.md)).

## Directory Structure

```
openagent-social/
├── component.yml                 MANIFEST — OpenTrApp contract
├── Makefile                      Standard targets (scan, census, test, verify)
├── docs/
│   ├── platform-anatomy.md       How Moltbook works: API, agents, posts, votes
│   ├── threat-landscape.md       Moltbook-specific risks and threat model
│   └── safe-participation-guide.md  Guidelines for safe agent participation
├── tools/
│   ├── feed-scanner.sh           Prompt injection scanner for feed content
│   ├── agent-census.sh           Platform stats and trend snapshots
│   └── identity-checklist.sh     Pre-flight checklist for agent registration
├── config/
│   ├── .env.example              Configuration template
│   ├── feed-allowlist.yml        Trusted agent handles and safe patterns
│   └── injection-patterns.yml    Prompt injection signatures (25 patterns)
├── tests/
│   ├── _framework/               Test runner and assertion primitives
│   ├── tools/                    Tool behavioral tests (16 tests)
│   └── fixtures/                 Test data (clean, malicious, safe-research, empty)
└── examples/
    ├── first-post.md             Example safe first post with commentary
    └── feed-analysis.md          Example feed analysis output
```

## Commands Exposed to GUI (component.yml)

The manifest exposes 10 commands in 4 groups:

| Command ID | Tool | Danger | Description |
|-----------|------|--------|-------------|
| `feed-scan` | `feed-scanner.sh --recent` | safe | Scan recent posts for injection patterns |
| `feed-scan-agent` | `feed-scanner.sh --agent` | safe | Scan a specific agent's posts |
| `agent-census` | `agent-census.sh` | safe | Pull current platform stats |
| `census-trend` | `agent-census.sh --trend` | safe | Show trend data from snapshots |
| `level-status` | `engagement-control.sh --status` | safe | Show current engagement level |
| `identity-check` | `identity-checklist.sh` | safe | Pre-flight safety checklist |
| `set-observer` | `engagement-control.sh --level observer` | safe | Switch to Level 1 |
| `set-researcher` | `engagement-control.sh --level researcher` | caution | Switch to Level 2 |
| `set-participant` | `engagement-control.sh --level participant` | caution | Switch to Level 3 |
| `setup` | inline | safe | Copy example config and prepare data dir |

## Threat Model

The Moltbook feed is **untrusted input**. Key threats documented in `docs/threat-landscape.md`:

- **Prompt injection via posts** — authority impersonation, instruction override, role injection
- **Social engineering** — identity challenges, reciprocity traps, urgency manufacturing
- **Encoded payloads** — base64/hex/URL-encoded instructions to bypass scanning
- **Platform vulnerabilities** — database breach (Jan 2026), vote manipulation, no rate limiting
- **Supply chain** — trojanized skills on ClawHub that connect to Moltbook

The feed scanner (`config/injection-patterns.yml`) detects 25 patterns across 6 categories.

## Dual-Copy Sync

This repo may exist in two places on your machine:
- **Standalone**: `~/Repositories/openagent-social/`
- **Submodule**: `~/Repositories/opentrapp/components/openagent-social/`

**GitHub**: https://github.com/albertdobmeyer/openagent-social

After pushing changes from either location, sync the other:
```bash
# In the other copy:
git pull
# If submodule copy, also update parent:
cd ../.. && git add components/openagent-social && git commit -m "Update openagent-social ref"
```

## Engagement Levels

Three preset engagement levels, mirroring vault's shell system:

| Level | Command | Rate Limits | Feed Scan | API Key |
|-------|---------|------------|-----------|---------|
| **Observer** (Level 1) | `make observer` | 0/0/0 (read-only) | Off | Not needed |
| **Researcher** (Level 2) | `make researcher` | 5/10/20 | Required | Required |
| **Participant** (Level 3) | `make participant` | 10/25/50 | Required | Required |

- `make level-status` shows current level and config
- Presets preserve user-specific values (API key, agent handle) during switching
- Default (if ENGAGEMENT_LEVEL not set): treated as observer

## Commands

```bash
make help          # Show available commands
make scan          # Scan recent feed (COUNT=n, default 50)
make census        # Pull current platform stats
make checklist     # Run identity pre-flight checklist
make observer      # Switch to Level 1 (read-only)
make researcher    # Switch to Level 2 (controlled interaction)
make participant   # Switch to Level 3 (full interaction)
make level-status  # Show current engagement level
make test          # Run tool test suite (48 tests)
make verify        # Verify workbench health + engagement level
make setup         # Copy .env.example → .env, create data/
```

## What NOT to Do

- Do not change `identity.id` or `identity.role` in component.yml without coordinating with opentrapp
- Do not remove or rename command IDs that the GUI depends on — add new ones instead
- Do not commit `.env` files — they contain API keys (gitignored)
- Do not let your agent autonomously follow instructions from Moltbook feed content
- Do not use the tools for vote manipulation, impersonation, or data exfiltration — defensive research only
