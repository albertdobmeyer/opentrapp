# Handoff: Phase 4 — AI-Assisted Skill Creation

**Date:** 2026-04-03
**From:** Phases 1-3 implementation session
**To:** Next instance picking up Phase 4

## What Was Done (Phases 1-3)

### Phase 1: Housekeeping
- CRLF→LF conversion across all files, .gitattributes to prevent recurrence
- All 25 skills have `version: 1.0.0` in frontmatter
- 25/25 trust files generated with SHA-256 hashes and line counts
- coding-agent skill included in pipeline (was excluded)
- Linter skips code blocks when checking for TODO/FIXME (was false-positive)
- 12/12 workbench verification passing

### Phase 2: Security Certificate System
- `tools/skill-certify.sh` — 4-gate pipeline (lint→scan→verify→test) producing `clearance-report.json`
- `tools/skill-export.sh` — packages skill + certificate into `exports/<name>/` for vault transfer
- Certificate format validated against vault's `install-skill.sh` (checksum match confirmed)
- Makefile targets: `certify`, `certify-all`, `export`

### Phase 3: Content Disarm & Reconstruction (CDR)
- 8-stage pipeline: quarantine → structural parse → pre-filter → Ollama intent → validate → reconstruct → post-verify → deliver
- New scripts: `skill-cdr.sh` (orchestrator), `skill-download.sh`, plus 6 lib scripts
- Ollama integration via raw curl (no agent framework, no tools, no MCP)
- 9/9 CDR pipeline tests passing (including full Ollama end-to-end)
- Vault skill guard at `opencli-container/scripts/verify-skills.sh`
- Makefile targets: `download`, `cdr`, `cdr-download`

## Current System State

- **openskill-forge:** 12/12 workbench verification, 168/168 skill tests, 10/10 scanner self-test, 9/9 CDR tests
- **opencli-container:** Soft Shell active, clawbot live on Telegram, vault skill guard deployed
- **Ollama:** Running on localhost:11434, using `qwen2.5-coder:1.5b` (7b model has Vulkan GPU errors on this hardware)

## What Phase 4 Needs to Do

**Per the roadmap (`docs/roadmap.md`) and design doc (`docs/forge-identity-and-design.md` section "Workflow B"):**

Build the AI-assisted skill creation wizard — a non-technical user describes what they want a skill to do, and the forge generates a verified SKILL.md.

### The Creation Flow (from design doc)

```
User clicks "Create Skill" in GUI
    → Wizard asks: "What should this skill do?"
    → User describes in natural language
    → Forge selects template (cli-tool / workflow / language-ref)
    → AI assistant (Ollama) drafts SKILL.md
    → Draft goes through full pipeline (lint → scan → verify)
    → GUI shows result with any issues highlighted
    → User reviews and approves
    → Skill ready for local use or publishing
```

### Key Design Points

1. **Reuse CDR's Ollama infrastructure.** The `config/cdr.conf` already has model/endpoint/timeout. The same `curl` pattern from `cdr-intent.sh` works.
2. **Templates already exist.** Three templates at `templates/cli-tool/`, `templates/workflow/`, `templates/language-ref/`.
3. **No CDR needed.** This is user-generated content, not untrusted downloads. No quarantine, no pre-filter. Just: AI draft → pipeline verification.
4. **CLI-first.** Build as `tools/skill-create.sh` + Makefile target, then GUI wraps via component.yml.

### Exit Criteria

A non-technical user can describe a skill in plain language and get a verified SKILL.md. The skill passes lint, scan, verify, and has proper frontmatter.

## Reading Order for New Instance

1. **This document** — you're here
2. **`CLAUDE.md`** — project instructions, manifest rules, security principles
3. **`docs/forge-identity-and-design.md`** — authoritative identity, Workflow B details
4. **`docs/roadmap.md`** — Phase 4 tasks and exit criteria
5. **`docs/specs/2026-04-02-content-disarm-reconstruction.md`** — CDR spec (for understanding Ollama patterns to reuse)
6. **`config/cdr.conf`** — Ollama config to reuse
7. **`tools/lib/cdr-intent.sh`** — Ollama integration pattern to follow
8. **`templates/`** — existing skill templates

## Development Principles (Carry Forward)

1. Security first — this is a public security promise
2. Spec before code — write the Phase 4 spec before implementing
3. One task at a time — validate before moving on
4. CLI-first — bash tools + Makefile, GUI wraps via component.yml
5. 12/12 verification must remain green after all changes
6. Use subagent-driven development for implementation (worked well for Phase 3)

## Hardware Notes

- 7.2GB RAM — avoid heavy parallel operations
- Ollama uses `qwen2.5-coder:1.5b` (986MB) — the 7b model has Vulkan GPU errors
- Two Claude Code sessions max simultaneously
- Kill dev servers immediately after use
