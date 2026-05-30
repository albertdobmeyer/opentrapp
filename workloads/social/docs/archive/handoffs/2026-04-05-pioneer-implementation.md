# Handoff: OpenAgent-Social Implementation (Phases 3-5)

**Date:** 2026-04-05
**From:** Session that completed Phases 1-2 and wrote the roadmap + specs
**To:** Next instance implementing Phases 3-5
**Previous handoff:** `docs/handoff-pioneer-completion.md` (Phase 1-2 bugs+tests, now complete)

## What This Module Is

OpenAgent-Social is the **network safety layer** of the OpenTrApp trifecta. Three bash tools for safe observation of the Moltbook agentic social network (~1.5M registered agents, ~201K verified, acquired by Meta March 10 2026).

| Tool | Purpose | Lines | Tests |
|------|---------|-------|-------|
| `tools/feed-scanner.sh` | Scan feed posts for 25 injection patterns | ~400 | 10 |
| `tools/agent-census.sh` | Pull platform stats and track trends | ~233 | 3 |
| `tools/identity-checklist.sh` | Pre-flight safety check before agent registration | ~204 | 3 |

**Role in ecosystem:** `network` — the outermost layer. Vault isolates the runtime, forge secures the supply chain, pioneer scans the social feed.

## Current State: Phases 1-2 Complete

**Phase 1 (bug fixes):** 11 commits fixing CRLF, executable bits, eval injection, stderr routing, dead variable, safe_patterns wiring, pattern count, component.yml states. Two latent bugs also found and fixed: `(?i)` PCRE flag broke grep ERE matching, `|` delimiter collided with regex alternation.

**Phase 2 (test framework):** 16 behavioral tests across 3 tools, 4 fixtures (clean, malicious, safe-research, empty), test framework ported from openskill-forge.

**Factual validation:** All docs verified against Wikipedia, Wiz blog, Vectra AI, Apidog, TechCrunch, Fortune. Key corrections applied: API URL (`api.moltbook.com`), agent counts, rate limits, breach timeline, Meta acquisition.

**Everything passes:** `make test` → 16/16 green. `make verify` → all OK.

## What's Left (3 Phases)

### Phase 3: Offline / Dry-Run Mode

**Why:** Moltbook API liveness is uncertain post-Meta acquisition. The census tool needs to work without network access.

**Work:**
1. Add `--file <path>` flag to `agent-census.sh` (model on feed-scanner's existing `--file` mode)
2. Create `tests/fixtures/census-snapshot.json` — sample API response
3. Add 2-3 census `--file` tests in `tests/tools/agent-census.tool-test.sh`
4. Add `make check-api` target to Makefile — curl the API, report status
5. Document API liveness findings in `docs/roadmap.md`

**Feed scanner already has `--file` mode** — that IS its offline mode. No changes needed there.

**Exit criteria:** All tools work offline. `make test` passes. API status documented.

### Phase 4: Vault Integration — Pattern Export

**Why:** When Moltbook domains enter the vault proxy allowlist, pioneer's 25 injection patterns need to be available to `vault-proxy.py` for response-level feed scanning.

**Full spec:** `docs/specs/2026-04-04-vault-integration-design.md`

**Pioneer-side work (this phase):**
1. Create `scripts/export-patterns.py` — reads `config/injection-patterns.yml`, validates all regexes compile with Python `re.compile()`, writes `data/patterns-export.yml`
2. Add `make export-patterns` target to Makefile
3. Add tests: export produces valid YAML, all 25 patterns compile, output has exactly 25 entries
4. Verify known malicious content matches expected patterns in Python

**Vault-side work (separate phase, master roadmap Phase C):**
- Pattern loading in `vault-proxy.py`, response inspection, critical blocking, logging
- NOT part of this handoff — that work happens in the vault repo

**Blocking policy (decided):**
- CRITICAL findings → block response (sanitized JSON returned)
- HIGH/MEDIUM findings → log to `requests.jsonl`, pass through

**Pattern export format:**
```yaml
patterns:
  - id: auth-001
    severity: HIGH
    regex: "(?i)(as the|i am the|this is the).{0,20}(admin|moderator|system|official|moltbook team)"
  # ... 25 total
```

`(?i)` flags are preserved in the export — Python's `re.compile()` handles them natively (unlike grep ERE where they break).

**Exit criteria:** `make export-patterns` works. All patterns compile in Python. Format documented.

### Phase 5: Pattern Harmonization with Forge

**Why:** Forge has 87 skill-focused patterns, pioneer has 25 social-focused patterns. Different domains, but worth comparing.

**Work:**
1. Read forge's patterns at `openskill-forge/tools/lib/patterns.sh`
2. Compare against pioneer's `config/injection-patterns.yml`
3. Document overlap in a comparison doc
4. Decision: share or keep separate (expected: keep separate — different threat surfaces)

**Exit criteria:** Comparison documented. Decision made with rationale.

## Reading Order

1. **This document** — you're here
2. **`CLAUDE.md`** — project conventions, manifest rules, commands
3. **`docs/roadmap.md`** — the full roadmap with Phase 1-2 marked complete
4. **`docs/specs/2026-04-04-vault-integration-design.md`** — Phase 4 spec (pattern export + proxy integration)
5. **`docs/platform-anatomy.md`** — verified API reference and platform details
6. **`docs/threat-landscape.md`** — what we're defending against
7. **`tools/feed-scanner.sh`** — the most complex tool, model for census `--file` mode
8. **`tools/agent-census.sh`** — needs `--file` flag added (Phase 3)
9. **`config/injection-patterns.yml`** — the 25 patterns that Phase 4 exports
10. **`tests/`** — existing framework and fixtures

## Key Technical Details

### Pattern Storage Format

Patterns are stored internally as tab-separated strings: `SEVERITY\tCATEGORY\tREGEX\tDESCRIPTION`. The `|` delimiter was abandoned because it collides with regex alternation. The `(?i)` PCRE flag is stripped during bash loading (grep -Ei handles case insensitivity), but preserved in the YAML source for Python consumers.

### API Base URL

The actual Moltbook API is at `https://api.moltbook.com` (NOT `moltbook.com/api/v1`). The `.env.example` and script defaults were corrected to this. Auth: `Authorization: Bearer moltbook_sk_<key>`.

### Platform Status

Meta acquired Moltbook March 10, 2026. The platform remained operational but its long-term API availability is uncertain. Rate limits: 100 req/min, 1 post/30min, 50 comments/hr.

### Test Framework

Ported from openskill-forge. Runner at `tests/_framework/tool-runner.sh` discovers `tests/tools/*.tool-test.sh` files. Assertions at `tests/_framework/tool-assertions.sh` (11 primitives). Tests define `test_` functions. Run via `make test`.

### Safe Patterns

`config/feed-allowlist.yml` defines two safe_patterns regexes that suppress findings for benign content (e.g., research discussing injection techniques). `is_safe_content()` in feed-scanner.sh checks these before flagging. The safe-research fixture tests this path.

## Development Principles

1. **One task at a time** — validate each change before moving on
2. **Fix bugs before adding features** — Phase 3 before Phase 4
3. **Each fix is a separate commit** — matching vault/forge discipline
4. **Spec-driven for Phase 4** — the spec exists, follow it
5. **Test what you claim** — if a test says "patterns compile in Python," verify with `re.compile()`
6. **Honest documentation** — all claims verified against real sources

## Ecosystem Context

| Component | Status | Relevant to Pioneer |
|-----------|--------|---------------------|
| opencli-container | Complete (all 8 phases) | Phase 4 pattern export feeds into vault-proxy.py |
| openskill-forge | Complete (4/5 phases) | Phase 5 compares forge's 87 patterns with pioneer's 25 |
| opentrapp GUI | Functional | Expects 6 commands from component.yml to work reliably |
| Master roadmap | Phase A (pioneer bugs+tests) done | Pioneer Phases 3-5 are the remaining Phase A work |

## Commit Discipline

Commits go to the **submodule** repo at `components/openagent-social/`. After pushing the submodule, update the parent's ref:

```bash
cd components/openagent-social && git push
cd ../.. && git add components/openagent-social && git commit -m "chore: update openagent-social submodule reference" && git push
```

---

*Cross-reference: `opentrapp/docs/superpowers/plans/2026-04-04-master-roadmap-v3.md` Phase A describes this work at the ecosystem level.*
