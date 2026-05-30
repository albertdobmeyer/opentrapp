# Verification Report: Regex Security Hardening

**Date:** 2026-04-06
**Verifier:** Fresh Claude instance (independent of implementation session)
**Handoff:** `docs/handoff-regex-verification.md`
**Spec:** `docs/specs/2026-04-05-regex-security-hardening.md`

## 1. Test Results

```
make test: 30 passed, 0 failed
make export-patterns: 25 patterns exported
```

All 11 export-patterns tests pass, including ReDoS detection, complexity scoring, integrity hash, and pathological input benchmark.

## 2. Claim Verification

| # | Claim | Result | Notes |
|---|-------|--------|-------|
| 1 | `make test` passes with 30 tests, 0 failures | **PASS** | Exact match |
| 2 | `make export-patterns` produces 25 patterns with integrity hash | **PASS** | Hash present in header |
| 3 | `check_redos(r'(a+)+$')` returns non-None | **PASS** | Returns `'nested quantifiers detected'` |
| 4 | `check_redos(r'(a*){2,}')` catches this | **PASS** | Returns `'nested quantifiers detected'` |
| 5 | All 25 patterns pass `check_redos()` | **PASS** | All return None |
| 6 | All 25 patterns score below WARN (30000) | **PASS** | Max is 23976 |
| 7 | Max complexity score is 23976 (enc-004) | **PASS** | Exact match |
| 8 | Integrity hash matches recomputed hash | **PASS** | Both: `78d872b3...` |
| 9 | 10KB pathological input < 1 second | **PASS** | 0.024s |
| 10 | `config/injection-patterns.yml` untouched | **PASS** | `git diff HEAD~5..HEAD -- config/` empty |

**All 10 claims verified.**

## 3. Spec Gap Analysis

### Gap A: Overlapping Quantifiers — CONFIRMED, Low Impact

**Finding:** `check_redos()` does not detect adjacent quantifiers (`.*.*`, `\w+\w+`, `.+.+`). All three return None.

**Impact on current patterns:** None. My heuristic scan flagged `url-001` and `url-004` as potential matches, but both have quantifiers separated by literal characters (`.`, `/`) on non-overlapping character sets. Benchmarked at < 0.002s per 1000 iterations on 10KB input.

**Impact on future patterns:** Low. Adjacent quantifiers on overlapping character sets would need to be nested quantifiers or anchored to cause actual ReDoS — both of which `check_redos()` already catches via the nested-quantifier and unbounded-alternation detectors.

**Recommendation:** Document as known limitation in spec. Do not implement — adjacent quantifiers without nesting or anchoring are not a ReDoS vector in Python's regex engine. If a future pattern combines adjacent quantifiers with anchoring (e.g., `^.*.*$`), the pathological input benchmark (test 10) will catch the runtime regression.

### Gap B: REJECT Invariant — CONFIRMED, Spec Flaw

**Finding:** `complexity_score(r'(a|b|c|d|e){0,100}(f|g|h|i|j){0,100}')` returns **200**, not > 50000.

**Root cause (two issues):**

1. **sre_parse optimization:** Python's parser compiles `(a|b|c|d|e)` into `IN [(LITERAL, 97), ...]` (a character class), NOT a `BRANCH` node. The scoring formula sees `branches=1` because there are no BRANCH nodes in the AST.

2. **The spec's example is not ReDoS-vulnerable.** The two character sets (`[a-e]` and `[f-j]`) are disjoint — the engine deterministically assigns each character to one group. Even with overlapping character sets and bounds up to 25, I measured 0.00s on failing inputs. Python optimizes single-character alternations into character classes that match in O(1) per position.

**Scoring trace:**
```
branches=1, max_bound=100, max_depth=2
score = 1 * 100 * 2 = 200
```

**Recommendation:** Update the spec. Remove the invariant claim about `(a|b|c|d|e){0,100}(f|g|h|i|j){0,100}` — it is not a dangerous pattern. Replace with a genuinely dangerous example that the scoring formula CAN catch, such as patterns with actual `BRANCH` nodes (multi-character alternatives) and high bounds. The scoring formula is useful as a secondary defense for patterns with large `alternation_branches * max_bound * depth` products, but it was never designed to catch the pattern the spec claims.

### Gap C: Fixed-Count Quantifier Refinement — CONFIRMED SOUND

**Finding:**
- `check_redos(r'(a*){2,}')` → `'nested quantifiers detected'` (variable inner `*`: min=0, max=MAXREPEAT)
- `check_redos(r'(a{2}){4,}')` → `None` (fixed inner `{2}`: min=max=2)

**Reasoning verified:** A fixed-count inner quantifier (`{n}` where min == max) always consumes exactly n characters per repetition. The outer quantifier has only one way to partition the input into n-character chunks — no ambiguity, no backtracking. This correctly avoids false positives on patterns like `(\\x[0-9a-fA-F]{2}){4,}` (enc-002, enc-003).

**Recommendation:** No action needed. The refinement is correct and well-motivated.

## 4. Implementation-Level Checks

### Check D: Validation Pipeline Order — CORRECT

```
Line 188: re.compile()    → continue on failure
Line 195: check_redos()   → continue on failure
Line 202: complexity_score → continue on REJECT (≥50000)
Line 208: WARN print       (no continue — pattern still exported)
Line 210: exported.append() — only reached if all gates pass
```

Each gate has a `continue` that prevents reaching subsequent steps. WARN is informational only (correct — should not block export).

### Check E: Hash Determinism — VERIFIED

Two consecutive exports produce byte-identical output files. Sort key is `x["id"]` (alphabetical by pattern ID), producing stable ordering.

### Check F: Test Independence — VERIFIED

- Each test calls `run_export()` which regenerates the export file fresh
- No test depends on state from another test
- `data/patterns-export.yml` is gitignored (confirmed via `git check-ignore`)

### Check G: sre_parse Compatibility — VERIFIED

- Python 3.12.3 on this system — `import re._parser` succeeds
- Fallback `import sre_parse` present for Python < 3.11
- Both import paths are in the code (lines 21-23)

## 5. Recommendations

### Must Fix (before Phase C vault integration)

**None.** All claims are verified, all current patterns are safe, the implementation is correct.

### Should Update (spec accuracy)

1. **Spec line 66 (overlapping quantifiers):** Add a note that this class is not implemented and explain why — adjacent quantifiers without nesting are not a ReDoS vector in CPython. The pathological input benchmark provides runtime protection.

2. **Spec line 87 (REJECT invariant):** Remove or rewrite. The example pattern `(a|b|c|d|e){0,100}(f|g|h|i|j){0,100}` is not ReDoS-vulnerable (disjoint character sets, character-class optimization). Replace with a genuinely dangerous example if one exists, or document that the REJECT threshold is calibrated to catch high `branches * bound * depth` products rather than specific pattern shapes.

3. **Spec status line:** Change from "implementation pending" to "Layer 1 implemented, Layers 2-4 pending (Phase C)".

### Nice to Have (defense in depth)

4. **Add a comment in `complexity_score()`** noting that single-character alternations are compiled to character classes by sre_parse and don't produce BRANCH nodes. This explains why patterns like `(a|b|c){0,100}` score as branches=1.

## 6. No Additional Tests Needed

Gap A does not affect current patterns and is not a real threat without nested quantifiers (already detected). Gap B is a spec flaw, not an implementation bug. Gap C is working correctly. No new tests required.

---

*Verified from: `openagent-social repo root`*
*Commits reviewed: `58adf3f`..`220887d` on main*
