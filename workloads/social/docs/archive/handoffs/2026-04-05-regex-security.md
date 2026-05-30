# Handoff: Regex Security Hardening Implementation

**Date:** 2026-04-05
**For:** A fresh Claude instance implementing the regex security hardening spec
**Working directory:** `openagent-social repo root`

## What Was Done (This Session)

1. **Phases 3-5 of openagent-social implemented** — 11 commits merged to main:
   - Phase 3: Census `--file` offline mode, fixture, tests, `make check-api`
   - Phase 4: Pattern export script (`scripts/export-patterns.py`), Makefile target, tests
   - Phase 5: Pattern harmonization analysis (keep separate from forge)

2. **Regex security hardening spec written** — 4 defense layers designed:
   - Layer 1: Static validation (pioneer, export-time) — **implement now**
   - Layer 2: Content normalization (vault, response-time) — Phase C
   - Layer 3: Runtime timeout (vault, match-time) — Phase C
   - Layer 4: Integrity verification (vault, startup) — pioneer hash now, vault verification Phase C

3. **Implementation plan written** — 5 tasks with TDD, bite-sized steps, complete code

## What to Build (Your Job)

Implement the **pioneer-side** regex security hardening: Layer 1 (static validation + integrity hash).

### Reading Order (Priority)

| # | File | Why |
|---|------|-----|
| 1 | `docs/superpowers/plans/2026-04-05-regex-security-hardening.md` | **THE PLAN** — follow this task by task |
| 2 | `docs/specs/2026-04-05-regex-security-hardening.md` | The governing spec — design rationale and decisions |
| 3 | `scripts/export-patterns.py` | The file you're modifying — 115 lines, read it |
| 4 | `tests/tools/export-patterns.tool-test.sh` | The test file you're extending — 62 lines, 5 existing tests |
| 5 | `CLAUDE.md` | Project rules — security-first pace, spec-driven development |

### The 5 Tasks

| Task | What | Deliverable |
|------|------|-------------|
| 1 | ReDoS detection via `re._parser` AST | `check_redos()` function + 2 tests |
| 2 | Complexity scoring | `complexity_score()` function + threshold calibration + 1 test |
| 3 | SHA-256 integrity hash | Hash in export file header + 2 tests |
| 4 | Pathological input test | 10KB benchmark test |
| 5 | Final verification + roadmap | `make test` (29 pass), roadmap update |

### Expected Final State

- `make test` → 29 tests pass (24 existing + 5 new)
- `make export-patterns` → produces `data/patterns-export.yml` with integrity hash
- All 25 current patterns pass ReDoS check and complexity scoring
- Known dangerous pattern `(a+)+$` is rejected
- 5 new commits on main

## Technical Gotchas

1. **`re._parser` not `sre_parse`** — Python 3.11+ deprecates `sre_parse`. Use `re._parser` with a fallback:
   ```python
   try:
       import re._parser as sre_parse
   except ImportError:
       import sre_parse
   ```

2. **YAML/regex backslash conflict** — `config/injection-patterns.yml` line 160 has `\.env` in a double-quoted YAML string. This is invalid YAML but works for the bash scanner's raw text parser. The export script (`scripts/export-patterns.py`) also parses raw text — do NOT use `yaml.safe_load()` on the source file. The *export output* (`data/patterns-export.yml`) is valid YAML and can be loaded with `yaml.safe_load()`.

3. **Test framework** — Tests live in `tests/tools/*.tool-test.sh`. The runner (`tests/_framework/tool-runner.sh`) discovers `test_` functions via `declare -F`. Tests use 11 assertion primitives from `tests/_framework/tool-assertions.sh`. Key ones: `assert_exit_code`, `assert_output_contains`, `assert_command_fails`, `assert_file_exists`.

4. **Module import in tests** — `export-patterns.py` has a hyphen so it can't be imported normally. The plan uses `SourceFileLoader` to import it for testing `check_redos()` and `complexity_score()` directly.

5. **Complexity threshold calibration** — The plan sets initial thresholds (WARN=5000, REJECT=50000). You MUST calibrate by scoring all 25 current patterns. The invariant: all current patterns score below WARN. Adjust thresholds if needed.

6. **`data/` is gitignored** — `patterns-export.yml` is generated on demand. Don't try to commit it. Clean it up after test runs with `rm -f data/patterns-export.yml`.

## Development Principles (From CLAUDE.md)

- **One task at a time** — validate each change before moving on
- **Each change is a separate commit** — 5 commits expected
- **Test what you claim** — TDD, run `make test` after every change
- **Security-first pace** — work slowly, verify, don't rush

## Commit Discipline

This repo is a git submodule of opentrapp. After completing all 5 tasks:
1. Push openagent-social changes (or leave for the user)
2. The user will update the submodule reference in opentrapp

## Verification Commands

```bash
make test                    # 29 tests pass
make export-patterns         # 25 patterns exported with hash
make verify                  # Health check passes
```

---

*Spec: `docs/specs/2026-04-05-regex-security-hardening.md`*
*Plan: `docs/superpowers/plans/2026-04-05-regex-security-hardening.md`*
