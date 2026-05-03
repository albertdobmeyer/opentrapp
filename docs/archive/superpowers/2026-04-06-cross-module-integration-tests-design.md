# Cross-Module Integration Test Suite — Design Specification

**Date:** 2026-04-06
**Status:** Approved design, pending implementation
**Authors:** albertd + Claude
**Scope:** New `tests/integration-test.sh` at lobster-trapp root

---

## Context

All three component modules are functionally complete:
- **openclaw-vault** — Phases 1-7, all 3 shell levels operational, 24-point verify passing
- **clawhub-forge** — Phases 1-4, 87-pattern scanner, CDR, AI creation wizard, security certificates
- **moltbook-pioneer** — All 5 phases, 25 injection patterns, pattern export with ReDoS hardening

The existing `tests/orchestrator-check.sh` (39 checks) validates **manifest structure** — schema compliance, enum alignment, cross-references within manifests. What's missing is validation of the **operational seams** — the actual data contracts flowing between modules.

Additionally, `docs/trifecta.md` (last updated 2026-03-27) contains stale information that no longer reflects reality. The integration test suite will catch such drift going forward.

## What This Is

A bash script (`tests/integration-test.sh`) that validates cross-module data contracts:
- Forge's clearance report format matches what vault expects
- Pioneer's pattern export format matches the vault integration spec
- Cross-references in documentation point to files that exist
- Submodules are healthy and synced

**What this is NOT:**
- Not a replacement for `orchestrator-check.sh` (manifest/structure checks)
- Not container-level testing (no running containers required)
- Not implementing deferred features (feed scanning stays deferred)

## Script Location & Style

- **Path:** `tests/integration-test.sh` (lobster-trapp root)
- **Style:** Matches `orchestrator-check.sh` — colored PASS/FAIL/WARN, sections, exit 1 on failure
- **Dependencies:** bash, python3, jq
- **No containers needed** — validates contracts at the file/format level

## Test Categories

### Category 1: Clearance Report Contract (Forge -> Vault) — 6 checks

Tests that forge's `skill-export.sh` output is consumable by vault's `install-skill.sh`.

| Check | What | How |
|-------|------|-----|
| 1.1 | Forge certify produces clearance-report.json | Run `make -C components/clawhub-forge certify SKILL=api-dev`, verify `skills/api-dev/clearance-report.json` exists |
| 1.2 | Report has required fields | jq: `.skill`, `.version`, `.scan.status`, `.scan.critical`, `.verify.verdict`, `.checksum` all present |
| 1.3 | Field types are correct | `scan.status` is string, `scan.critical` is number, `checksum` starts with `sha256:` |
| 1.4 | SHA-256 checksum matches SKILL.md | Recompute sha256sum of the skill file, compare to `.checksum` value |
| 1.5 | Pattern count matches actual scanner | `.scan.pattern_count` equals `grep -c 'pattern\[' components/clawhub-forge/tools/lib/patterns.sh` (87) |
| 1.6 | Forge export packages correctly | Run `make -C components/clawhub-forge export SKILL=api-dev`, verify exports/ dir has SKILL.md + clearance-report.json + .trust |

### Category 2: Pattern Export Contract (Pioneer -> Vault) — 6 checks

Tests that pioneer's `export-patterns.py` output matches what vault's proxy spec expects.

| Check | What | How |
|-------|------|-----|
| 2.1 | Pattern export produces YAML | Run `make -C components/moltbook-pioneer export-patterns`, verify `data/patterns-export.yml` exists |
| 2.2 | All patterns have required fields | Python: each entry has `id`, `severity`, `regex` |
| 2.3 | All regexes compile | Python: `re.compile(pattern['regex'])` for each |
| 2.4 | Integrity hash is present and valid | File contains `# Integrity: sha256:...`, recompute and compare |
| 2.5 | Pattern count matches source config | Exported count equals entries in `config/injection-patterns.yml` |
| 2.6 | Severity values are valid | Each pattern's severity is one of: CRITICAL, HIGH, MEDIUM, LOW |

### Category 3: Cross-Reference Integrity — 5 checks

Tests that documentation cross-references point to real files.

| Check | What | How |
|-------|------|-----|
| 3.1 | trifecta.md referenced files exist | Parse file references (design docs, roadmaps), verify each path exists |
| 3.2 | Vault skill-install spec references match forge output | Clearance report fields in vault's spec match fields forge actually produces |
| 3.3 | Vault feed-scanning spec references match pioneer export | Expected pattern format in vault's deferred spec matches pioneer's actual export format |
| 3.4 | Each module has CLAUDE.md and component.yml | All 3 submodule directories contain both files |
| 3.5 | Ownership matrix — referenced tools exist | Each tool path in trifecta.md ownership descriptions maps to an existing file |

### Category 4: Submodule Health — 3 checks

| Check | What | How |
|-------|------|-----|
| 4.1 | All submodule directories are non-empty | `ls components/{openclaw-vault,clawhub-forge,moltbook-pioneer}/component.yml` |
| 4.2 | Submodules are on a branch (not detached HEAD) | `git -C components/<mod> symbolic-ref HEAD` succeeds |
| 4.3 | Submodule working trees are clean | `git -C components/<mod> status --porcelain` is empty |

### Category 5: Orchestrator Passthrough — 2 checks

| Check | What | How |
|-------|------|-----|
| 5.1 | orchestrator-check.sh passes | Run it, verify exit 0 |
| 5.2 | Component roles are correct | vault=runtime, forge=toolchain, pioneer=network (parsed from component.yml) |

**Total: 22 checks**

## Known Issues to Flag

During exploration, these discrepancies were found. The test suite should catch similar drift:

1. **trifecta.md says "30 patterns"** for pioneer — actual count is 25
2. **trifecta.md says forge is 85%** — it's 100% (Phases 1-4 complete)
3. **trifecta.md says skill installation "not yet implemented"** — both sides are built
4. **trifecta.md says monitoring "NOT YET"** — network-log-parser.py and session-report.py exist
5. **trifecta.md says 23-point verification** — vault now has 24 checks

These should be fixed in trifecta.md as part of implementation.

## Verification Plan

After the script is written:

```bash
# Run integration tests
cd ~/Repositories/lobster-trapp
bash tests/integration-test.sh

# Expected: 22/22 PASS (after trifecta.md fixes)
# If any fail: fix the source of truth, re-run
```

## Out of Scope

- Container-level testing (starting vault, installing skills into running container)
- Implementing deferred integrations (feed scanning, Moltbook domains)
- GUI/Tauri testing
- Performance benchmarking
