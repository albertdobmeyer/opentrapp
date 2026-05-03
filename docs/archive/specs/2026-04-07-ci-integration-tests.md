# Spec: CI Integration Tests

**Date:** 2026-04-07
**Phase:** F (Finalization Roadmap v4)
**Depends on:** Nothing
**Blocks:** Nothing (but provides confidence for all subsequent phases)

---

## Problem

`tests/integration-test.sh` (467 lines, 28 checks) validates critical cross-module contracts:

1. **Forge → Vault:** Clearance reports have required fields, checksums match SKILL.md content
2. **Pioneer → Vault:** Pattern exports compile as valid regex, integrity hashes verify
3. **Cross-reference integrity:** trifecta.md references exist, key tools exist in expected locations
4. **Submodule health:** component.yml exists, branches named, trees clean
5. **Orchestrator passthrough:** orchestrator-check.sh embedded run

These checks **only run manually**. CI runs `orchestrator-check.sh` (40 structural checks) but not the integration tests that validate data contracts. A regression in the clearance report format or pattern export could silently break the forge→vault skill installation path.

---

## Current CI Pipeline

File: `.github/workflows/ci.yml` (178 lines)

```
check-frontend ─┐
check-rust ──────┼──→ build-and-release (4 platforms)
check-orchestration ─┘
                 │
smoke-test ──────┘ (depends on check-frontend)
```

**Missing:** Integration test job between `check-orchestration` and `build-and-release`.

---

## Design: `--ci` Flag

Add a `--ci` flag to `tests/integration-test.sh` that skips checks requiring:
- A running container (Podman/Docker)
- Network access to external APIs (Moltbook, ClawHub)
- Ollama running locally

### Check Classification

| Check Group | CI-Safe? | Reason |
|-------------|----------|--------|
| Clearance report contract (section 1) | YES | `make certify` and `make export` use lint/scan/verify/test — all pure bash/python, no Ollama |
| Pattern export contract (section 2) | YES | `make export-patterns` uses python3 + pyyaml only |
| Cross-reference integrity (section 3) | YES | `test -f` on local paths |
| Submodule health (section 4) | YES | Local git operations |
| Orchestrator passthrough (5.1) | SKIP | Already runs as separate CI job — skip to avoid duplicate work |
| Component roles (5.2) | YES | Reads local YAML |

Note: `make certify` does NOT invoke CDR/Ollama — it runs the 4-gate pipeline (lint → scan → verify → test). CDR is a separate workflow (`make cdr`).

### Implementation

Modify `tests/integration-test.sh`:

```bash
CI_MODE=false
if [[ "${1:-}" == "--ci" ]]; then
  CI_MODE=true
fi

# Before each CI-unsafe check:
if [[ "$CI_MODE" == true ]]; then
  warn "SKIP: Forge live certify (requires Ollama)"
  skipped=$((skipped + 1))
else
  # ... existing check ...
fi
```

**Expected result in CI:** ~27 checks run, ~1 skipped (5.1 orchestrator passthrough), 0 failures.

---

## New CI Job

Add after `check-orchestration` in `.github/workflows/ci.yml`:

```yaml
  integration-tests:
    name: Integration tests (cross-module contracts)
    needs: check-orchestration
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: true

      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"

      - run: pip install pyyaml

      - name: Install jq
        run: sudo apt-get install -y jq

      - run: bash tests/integration-test.sh --ci
```

### Updated Pipeline

```
check-frontend ─────┐
check-rust ──────────┼──→ build-and-release
check-orchestration ─┤
integration-tests ───┘
                     │
smoke-test ──────────┘
```

`integration-tests` depends on `check-orchestration` (which validates manifest structure that integration tests assume is correct).

---

## Files to Modify

| Action | Path | Change |
|--------|------|--------|
| **Modify** | `tests/integration-test.sh` | Add `--ci` flag parsing, wrap CI-unsafe checks in conditionals, add skip counter to summary |
| **Modify** | `.github/workflows/ci.yml` | Add `integration-tests` job after `check-orchestration` |

---

## Verification

1. Run `bash tests/integration-test.sh --ci` locally — all CI-safe checks pass, CI-unsafe checks report SKIP
2. Run `bash tests/integration-test.sh` locally (without flag) — all checks run as before (no regression)
3. Push to a PR branch — CI shows `integration-tests` job passing
4. Intentionally break a clearance report field — CI catches the regression

---

## Future: Full Integration Tests

When a CI runner with Podman becomes available (e.g., self-hosted runner), the `--ci` flag can be removed to run the full suite including container-dependent checks. This is not required for v0.1.0.
