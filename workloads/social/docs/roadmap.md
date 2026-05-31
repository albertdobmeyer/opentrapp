# OpenAgent-Social Roadmap

**Updated:** 2026-04-07 (roadmap content); module **parked** since 2026-05-03.
**Current state:** Three tools operational, 25 injection patterns, 48 tests passing, engagement-level presets with GUI commands. All six phases of the original roadmap completed.

**Module status:** Parked since 2026-05-03 following Meta's acquisition of Moltbook (2026-03-10) and the resulting API instability since 2026-04-05. The container is still defined in `opentrapp/compose.yml` for completeness but has no functional API to talk to. The roadmap below documents the intended development path; future revisions are gated on a stable target API (or a successor agent-social-network platform).

**Cross-reference:** See `docs/trifecta.md` in the opentrapp root for how this module fits with opencli-container and openagent-skills.

---

## Phase 1: Fix Known Bugs — COMPLETE (2026-04-04)

11-commit sequence fixing all known bugs:

| Fix | Commit |
|-----|--------|
| CRLF → LF + .gitattributes | `b3d6007` |
| Executable bits on tools/*.sh | `d5e50bd` |
| eval → array-based curl | `86f4733` |
| fetch_posts stderr | `4d17d0f` |
| Dead in_pattern variable | `e00b31a` |
| Wire safe_patterns | `512fd2e` |
| Pattern count 30 → 25 | `e75a7c0` |
| component.yml states | `c77d192` |
| Makefile | `52b2414` |

Also fixed two latent bugs discovered during testing: `(?i)` PCRE flag broke grep ERE matching, and `|` delimiter collided with regex alternation in the pattern storage format.

---

## Phase 2: Automated Tests — COMPLETE (2026-04-04)

| Deliverable | Status |
|-------------|--------|
| Test framework (tool-runner.sh + tool-assertions.sh) | Ported from forge |
| Feed scanner tests (10) | Passing |
| Agent census tests (3) | Passing |
| Identity checklist tests (3) | Passing |
| Fixtures (clean, malicious, safe-research, empty) | Created |
| `make test` target | Working |

**Total:** 16 tests, 0 failures.

---

## Phase 3: Offline / Dry-Run Mode — COMPLETE (2026-04-05)

| Deliverable | Status |
|-------------|--------|
| Census `--file <path>` flag | Working — mirrors feed-scanner pattern |
| Census fixture (`tests/fixtures/census-snapshot.json`) | Created — 6 posts, 4 unique agents |
| Census offline tests (3) | Passing |
| `make check-api` target | Working — reports UP or DOWN |
| API status | DOWN as of 2026-04-05 (api.moltbook.com unreachable) |

**Total:** 19 tests, 0 failures. All three tools work offline.

---

## Phase 4: Vault Integration — Pattern Export — COMPLETE (2026-04-05)

**Full spec:** `docs/specs/2026-04-04-vault-integration-design.md`

| Deliverable | Status |
|-------------|--------|
| `scripts/export-patterns.py` | Working — raw text parsing avoids YAML/regex backslash conflict |
| `make export-patterns` target | Working — produces `data/patterns-export.yml` |
| All 25 regexes compile in Python `re` | Verified — round-trips through PyYAML cleanly |
| Export tests (5) | Passing — validity, count, compilation, malicious content match |
| Spec approved | `docs/specs/2026-04-04-vault-integration-design.md` |

**Total:** 24 tests, 0 failures. Pattern export format stable.

**Note:** Vault-side integration (proxy response inspection, blocking logic) is Phase C of the master roadmap — not pioneer's responsibility. The export mechanism is dormant until Moltbook domains enter the allowlist.

**Regex security hardening (2026-04-05):** ReDoS static analysis via `re._parser`, complexity scoring with WARN/REJECT thresholds (30000/50000, calibrated against all 25 patterns), SHA-256 integrity hash in export, pathological input benchmark. Spec: `docs/specs/2026-04-05-regex-security-hardening.md`. Vault-side runtime protections (Layers 2-4) deferred to Phase C.

**Gap analysis closure (2026-04-06):** Independent verification confirmed all 10 implementation claims. Two spec inaccuracies corrected: overlapping quantifier detection documented as intentionally not implemented (not a CPython ReDoS vector), and REJECT invariant example removed (Python optimizes single-character alternations to character classes). See `docs/report-regex-verification.md`.

---

## Phase 5: Pattern Harmonization with Forge — COMPLETE (2026-04-05)

**Full analysis:** `docs/pattern-harmonization.md`

| Finding | Detail |
|---------|--------|
| Forge patterns | 87 across 13 categories (supply-chain, execution, persistence, etc.) |
| Pioneer patterns | 25 across 6 categories (social manipulation, injection, exfiltration, etc.) |
| Overlap | 8 pattern pairs (~7%) — similar intent but different implementations |
| Non-overlapping in forge | 79 patterns (command execution, persistence, container escape, etc.) |
| Non-overlapping in pioneer | 17 patterns (authority impersonation, URL fishing, social engineering) |

**Decision: Keep separate.** Different threat surfaces (social content vs skill code), different consumers (Python re vs bash grep), different format requirements (inline (?i) vs FLAGS field). The 7% overlap doesn't justify a shared format.

---

## Phase 6: Engagement Level Presets — COMPLETED (2026-04-07)

**Full spec:** `docs/specs/2026-04-07-engagement-presets.md`

| Deliverable | Status |
|-------------|--------|
| `config/observer.env` preset (Level 1) | Created |
| `config/researcher.env` preset (Level 2) | Created |
| `config/participant.env` preset (Level 3) | Created |
| `scripts/engagement-control.sh` switcher | Working — status, dry-run, apply modes |
| `ENGAGEMENT_LEVEL` field in `.env` | Added to `.env.example` and all presets |
| 4 Makefile targets (observer, researcher, participant, level-status) | Working |
| 4 component.yml commands (set-observer, set-researcher, set-participant, level-status) | Added — GUI buttons |
| `scripts/verify.sh` (proper script, replaces inline Makefile) | Created — level-specific checks |
| 18 new tests for engagement control | All passing |
| Key/handle preservation during preset switch | Verified |
| Round-trip switching (observer → researcher → participant → observer) | Verified |

**Total:** 48 tests, 0 failures. Orchestrator validation: 39/39.

---

## Dependency Graph

```
Phase 1 (Bug fixes) ✅
    ↓
Phase 2 (Automated tests) ✅
    ↓
Phase 3 (Offline mode) ✅
    ↓
Phase 4 (Vault integration) ✅ — pattern export ready, vault-side deferred to Phase C
    ↓
Phase 5 (Pattern harmonization) ✅ — keep separate, 7% overlap
    ↓
Phase 6 (Engagement presets) ✅ — observer/researcher/participant with GUI commands
```

---

*This roadmap covers the openagent-social module only. See `opencli-container/docs/roadmap.md` and `openagent-skills/docs/roadmap.md` for the other modules.*
