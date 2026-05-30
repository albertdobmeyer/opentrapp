# Handoff: Verify Regex Security Hardening Implementation

**Date:** 2026-04-05
**For:** A fresh Claude instance verifying all claims from the implementation session
**Working directory:** `openagent-social repo root`

## What Was Built (This Session)

5 commits on main (`58adf3f`..`220887d`), modifying 3 files:

| Commit | Description | Files |
|--------|-------------|-------|
| `58adf3f` | ReDoS static analysis via `re._parser` AST | `scripts/export-patterns.py`, `tests/tools/export-patterns.tool-test.sh` |
| `e5432b0` | Complexity scoring (WARN=30000, REJECT=50000) | same two files |
| `34eda36` | SHA-256 integrity hash in export header | same two files |
| `404bcbe` | Pathological input benchmark (10KB, <1s) | `tests/tools/export-patterns.tool-test.sh` |
| `220887d` | Roadmap update | `docs/roadmap.md` |

## Your Job: Verify Everything

The previous session claimed all of the following. **Trust nothing. Run commands. Read output. Report what you actually find.**

### Claims to Verify

| # | Claim | How to Verify |
|---|-------|---------------|
| 1 | `make test` passes with 30 tests, 0 failures | Run `make test`, count results |
| 2 | `make export-patterns` produces 25 patterns with integrity hash | Run it, inspect output file header |
| 3 | `check_redos(r'(a+)+$')` returns a non-None string | Call function directly |
| 4 | `check_redos(r'(a*){2,}')` catches this (spec example) | Call function directly |
| 5 | All 25 current patterns pass `check_redos()` (return None) | Loop over all patterns |
| 6 | All 25 current patterns score below WARN_THRESHOLD (30000) | Score each, compare |
| 7 | Max complexity score is 23976 (enc-004) | Score each, find max |
| 8 | Integrity hash in export file matches recomputed hash | Export, extract, recompute, compare |
| 9 | 10KB pathological input completes in < 1 second | Time it |
| 10 | `config/injection-patterns.yml` was NOT modified | `git diff HEAD~5..HEAD -- config/` |

### Known Spec Gaps to Investigate

The governing spec (`docs/specs/2026-04-05-regex-security-hardening.md`) makes claims the implementation may NOT fully satisfy. These require investigation:

#### Gap A: Overlapping Quantifiers Not Implemented

**Spec says (line 66):** Flag "Overlapping quantifiers: Adjacent quantifiers on overlapping character sets (e.g., `.*.*`, `\w+\w+`)"

**Implementation:** `check_redos()` only detects nested quantifiers (quantifier inside quantifier) and unbounded repetition on alternation. It does NOT detect adjacent quantifiers like `.*.*` or `\w+\w+`.

**Verify:** Run `check_redos(r'.*.*')` and `check_redos(r'\w+\w+')`. If they return None, the gap is confirmed.

**Impact assessment needed:** Are any of the 25 current patterns vulnerable to this? Does this gap matter for the threat model?

#### Gap B: Complexity REJECT Invariant May Not Hold

**Spec says (line 87):** "A pattern like `(a|b|c|d|e){0,100}(f|g|h|i|j){0,100}` must score above REJECT."

**REJECT = 50000.** The previous session did not test this invariant.

**Verify:** Run `complexity_score(r'(a|b|c|d|e){0,100}(f|g|h|i|j){0,100}')`. If the score is below 50000, the invariant is violated and either the thresholds or the scoring formula need adjustment.

#### Gap C: Fixed-Count Quantifier Refinement

**Spec says (line 64):** `(a*){2,}` should be caught as nested quantifiers.

**Implementation refined this:** Only flags nested quantifiers where the inner quantifier has variable length (`_min != _max`). This was done because patterns like `(\\x[0-9a-fA-F]{2}){4,}` (enc-002, enc-003) use fixed-count inner `{2}` which is ReDoS-safe.

**Verify:**
- `check_redos(r'(a*){2,}')` should return non-None (inner `*` is variable-length: min=0, max=MAXREPEAT)
- `check_redos(r'(a{2}){4,}')` should return None (inner `{2}` is fixed-count: min=max=2)
- Confirm this refinement is actually sound (fixed-count inner quantifiers genuinely cannot cause backtracking)

### Implementation-Level Checks

Beyond the claims, verify the code itself:

#### D: Validation Pipeline Order

Read `scripts/export-patterns.py` lines 187-210. The validation order should be:
1. `re.compile()` — syntax check
2. `check_redos()` — ReDoS AST analysis
3. `complexity_score()` — threshold check
4. `exported.append()` — only if all three pass

Verify each gate actually prevents reaching the next step (i.e., `continue` statements are present).

#### E: Hash Computation Determinism

The integrity hash sorts patterns by ID before hashing. Verify:
- Export the file twice → hash should be identical both times
- The sort key is `x["id"]` (line 217) — confirm this produces a stable ordering

#### F: Test Independence

Each test calls `run_export()` which silently regenerates the export file. Verify:
- Tests don't depend on execution order
- A test failure in one doesn't cascade into false failures in others
- The `data/` directory is gitignored (generated files aren't committed)

#### G: sre_parse Compatibility

The code uses `re._parser` with fallback to `sre_parse`. Verify:
- `import re._parser as sre_parse` succeeds on the system's Python version
- Check `python3 --version` and confirm `re._parser` is available (Python 3.11+)

### Edge Case Tests to Write (If Gaps Found)

If you find spec gaps are real, write additional tests:

```bash
# For Gap A (overlapping quantifiers):
test_overlapping_quantifiers_detected() {
  python3 -c "
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader('ep', '$REPO_ROOT/scripts/export-patterns.py').load_module()
# Adjacent unbounded quantifiers on overlapping character sets
for pattern in [r'.*.*', r'\w+\w+', r'.+.+']:
    result = mod.check_redos(pattern)
    if result is None:
        print(f'GAP: {pattern} was not detected', file=sys.stderr)
        # Note: this is a spec gap, not necessarily a bug
"
}

# For Gap B (REJECT invariant):
test_wide_alternation_exceeds_reject() {
  python3 -c "
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader('ep', '$REPO_ROOT/scripts/export-patterns.py').load_module()
score = mod.complexity_score(r'(a|b|c|d|e){0,100}(f|g|h|i|j){0,100}')
print(f'Score: {score}, REJECT: {mod.REJECT_THRESHOLD}')
if score < mod.REJECT_THRESHOLD:
    print(f'INVARIANT VIOLATED: spec says this must exceed REJECT', file=sys.stderr)
"
}
```

## Reading Order

| # | File | Why |
|---|------|-----|
| 1 | This handoff | You're reading it |
| 2 | `scripts/export-patterns.py` | The implementation — 247 lines, read it all |
| 3 | `tests/tools/export-patterns.tool-test.sh` | All 11 test functions — 163 lines |
| 4 | `docs/specs/2026-04-05-regex-security-hardening.md` | The governing spec — compare claims vs implementation |
| 5 | `config/injection-patterns.yml` | The 25 source patterns — verify nothing was modified |

## Verification Commands

```bash
# Basic smoke test
make test                                    # 30 tests expected
make export-patterns                         # 25 patterns expected

# Detailed verification
python3 -c "
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader('ep', 'scripts/export-patterns.py').load_module()

# ReDoS detection
print('=== ReDoS Detection ===')
print(f'(a+)+$:     {mod.check_redos(r\"(a+)+\")!r}')        # Should be non-None
print(f'(a*)(2,):   {mod.check_redos(r\"(a*){2,}\")!r}')     # Should be non-None (spec example)
print(f'(a{2}){4,}: {mod.check_redos(r\"(a{2}){4,}\")!r}')   # Should be None (fixed inner)
print(f'.*.*:       {mod.check_redos(r\".*.*\")!r}')          # GAP A: likely None
print(f'\w+\w+:     {mod.check_redos(r\"\w+\w+\")!r}')       # GAP A: likely None

# Complexity scoring
print('\n=== Complexity Scoring ===')
patterns = mod.parse_patterns('config/injection-patterns.yml')
scores = [(p['id'], mod.complexity_score(p['regex'])) for p in patterns]
for pid, s in sorted(scores, key=lambda x: -x[1])[:5]:
    print(f'{pid:>10}: {s}')
print(f'Max: {max(s for _, s in scores)}, WARN: {mod.WARN_THRESHOLD}, REJECT: {mod.REJECT_THRESHOLD}')

# Gap B invariant
gap_b = mod.complexity_score(r'(a|b|c|d|e){0,100}(f|g|h|i|j){0,100}')
print(f'\nGap B pattern score: {gap_b} (spec says must exceed {mod.REJECT_THRESHOLD})')
"

# Source patterns untouched
git diff HEAD~5..HEAD -- config/injection-patterns.yml
```

## Deliverables

After verification, report:

1. **Test results** — exact pass/fail counts from fresh `make test`
2. **Spec compliance** — for each of Gaps A, B, C: confirmed/refuted, impact assessment
3. **Recommendations** — if gaps are real: fix now, update spec, or document as known limitation?
4. **Any additional issues** found during code review

## Development Rules (From CLAUDE.md)

- Work slowly, one thing at a time
- Security-first pace — verify before moving on
- Do NOT modify `config/injection-patterns.yml`
- `data/` is gitignored — clean up `data/patterns-export.yml` after test runs

---

*Spec: `docs/specs/2026-04-05-regex-security-hardening.md`*
*Implementation plan: `docs/superpowers/plans/2026-04-05-regex-security-hardening.md`*
*Previous handoff: `docs/handoff-regex-security.md`*
