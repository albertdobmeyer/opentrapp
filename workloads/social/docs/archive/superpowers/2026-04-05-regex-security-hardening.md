# Regex Security Hardening — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add ReDoS static analysis, complexity scoring, and integrity hashing to pioneer's pattern export script, preventing unsafe patterns from reaching vault-proxy.py.

**Architecture:** Extend `scripts/export-patterns.py` with three new validation stages (ReDoS detection, complexity scoring, integrity hash) that run between the existing `re.compile()` check and the YAML export. Add corresponding tests. This is pioneer-side only — vault-side runtime protections (Layers 2-4) are Phase C of the master roadmap.

**Tech Stack:** Python 3 stdlib (`re._parser` for AST analysis, `hashlib` for SHA-256), PyYAML, bash test framework.

**Spec:** `docs/specs/2026-04-05-regex-security-hardening.md`

**Working directory:** `openagent-social repo root`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `scripts/export-patterns.py` | Modify | Add ReDoS check, complexity scoring, integrity hash |
| `tests/tools/export-patterns.tool-test.sh` | Modify | Add 4 new tests (ReDoS rejection, complexity, pathological input, integrity hash) |

Two files total. No new files needed.

---

### Task 1: Add ReDoS Detection

Add a function to `scripts/export-patterns.py` that analyzes a compiled regex AST for catastrophic backtracking patterns using `re._parser`.

**Files:**
- Modify: `scripts/export-patterns.py`
- Modify: `tests/tools/export-patterns.tool-test.sh`

- [ ] **Step 1: Write the failing test**

Add to `tests/tools/export-patterns.tool-test.sh`:

```bash
test_redos_pattern_is_rejected() {
  # A known ReDoS pattern should cause the export to fail
  # We test the check_redos function directly rather than modifying the source YAML
  python3 -c "
import sys
sys.path.insert(0, '$REPO_ROOT/scripts')
# Import the module — we need to test the function directly
# since we can't easily inject a bad pattern into the YAML
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader('export_patterns', '$REPO_ROOT/scripts/export-patterns.py').load_module()
# (a+)+ is the classic ReDoS pattern
result = mod.check_redos(r'(a+)+\$')
if result is None:
    print('FAIL: ReDoS pattern was not detected', file=sys.stderr)
    sys.exit(1)
"
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd components/openagent-social && make test`

Expected: `test_redos_pattern_is_rejected` FAILS because `check_redos` function doesn't exist yet.

- [ ] **Step 3: Implement check_redos function**

Add to `scripts/export-patterns.py` after the imports, before `parse_patterns()`:

```python
import hashlib

try:
    import re._parser as sre_parse
except ImportError:
    import sre_parse  # Python < 3.11 fallback


def check_redos(regex_str):
    """Check a regex string for ReDoS vulnerability via AST analysis.

    Returns a description string if vulnerable, None if safe.
    Detects: nested quantifiers, unbounded repetition on alternation groups.
    """
    try:
        parsed = sre_parse.parse(regex_str)
    except Exception:
        return None  # Can't parse = can't check; re.compile() catches syntax errors

    def has_quantifier(node_list):
        """Check if a node list contains any quantifier (MAX_REPEAT or MIN_REPEAT)."""
        for op, av in node_list:
            if op in (sre_parse.MAX_REPEAT, sre_parse.MIN_REPEAT):
                return True
            if op == sre_parse.SUBPATTERN and av[3]:
                if has_quantifier(av[3]):
                    return True
            if op == sre_parse.BRANCH:
                for branch in av[1]:
                    if has_quantifier(branch):
                        return True
        return False

    def walk(node_list, inside_quantifier=False):
        """Walk the AST looking for nested quantifiers."""
        for op, av in node_list:
            if op in (sre_parse.MAX_REPEAT, sre_parse.MIN_REPEAT):
                _min, _max, subpattern = av
                # Check if this quantifier contains another quantifier
                if inside_quantifier and has_quantifier(subpattern):
                    return "nested quantifiers detected"
                # Check for unbounded quantifier on alternation
                if _max == sre_parse.MAXREPEAT:
                    for sub_op, sub_av in subpattern:
                        if sub_op == sre_parse.SUBPATTERN and sub_av[3]:
                            for inner_op, inner_av in sub_av[3]:
                                if inner_op == sre_parse.BRANCH:
                                    return "unbounded repetition on alternation"
                        if sub_op == sre_parse.BRANCH:
                            return "unbounded repetition on alternation"
                # Recurse into the quantifier body
                result = walk(subpattern, inside_quantifier=True)
                if result:
                    return result
            elif op == sre_parse.SUBPATTERN:
                if av[3]:
                    result = walk(av[3], inside_quantifier)
                    if result:
                        return result
            elif op == sre_parse.BRANCH:
                for branch in av[1]:
                    result = walk(branch, inside_quantifier)
                    if result:
                        return result
        return None

    return walk(parsed)
```

- [ ] **Step 4: Wire check_redos into the main validation loop**

In `scripts/export-patterns.py`, in the `main()` function, after the `re.compile()` check (line 79) and before appending to `exported`, add:

```python
        try:
            re.compile(regex)
        except re.error as e:
            print(f"  FAIL: {pid} — compile error: {e}", file=sys.stderr)
            failures += 1
            continue

        # ReDoS static analysis
        redos_issue = check_redos(regex)
        if redos_issue:
            print(f"  REJECT: {pid} — {redos_issue}", file=sys.stderr)
            failures += 1
            continue

        exported.append({"id": pid, "severity": severity, "regex": regex})
```

- [ ] **Step 5: Run tests to verify all pass**

Run: `cd components/openagent-social && make test`

Expected: All tests pass including `test_redos_pattern_is_rejected`. The existing 25 patterns must all still pass (they are ReDoS-safe).

- [ ] **Step 6: Add test that safe patterns are NOT rejected**

Add to `tests/tools/export-patterns.tool-test.sh`:

```bash
test_safe_bounded_pattern_passes_redos_check() {
  # A bounded alternation pattern (like our real patterns) should pass
  python3 -c "
import sys
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader('export_patterns', '$REPO_ROOT/scripts/export-patterns.py').load_module()
# This mimics our real patterns: bounded quantifier + alternation
result = mod.check_redos(r'(?i)(ignore|forget).{0,20}(instructions|rules)')
if result is not None:
    print(f'FAIL: Safe pattern was rejected: {result}', file=sys.stderr)
    sys.exit(1)
"
}
```

- [ ] **Step 7: Run tests**

Run: `cd components/openagent-social && make test`

Expected: All pass.

- [ ] **Step 8: Commit**

```bash
cd components/openagent-social
git add scripts/export-patterns.py tests/tools/export-patterns.tool-test.sh
git commit -m "feat: add ReDoS static analysis to pattern export

Uses re._parser AST to detect nested quantifiers and unbounded
repetition on alternation groups. Rejects unsafe patterns at export time.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Add Complexity Scoring

Add complexity scoring to `scripts/export-patterns.py` that flags patterns exceeding a threshold.

**Files:**
- Modify: `scripts/export-patterns.py`
- Modify: `tests/tools/export-patterns.tool-test.sh`

- [ ] **Step 1: Write the failing test**

Add to `tests/tools/export-patterns.tool-test.sh`:

```bash
test_all_patterns_below_warn_threshold() {
  # All 25 current patterns must score below the WARN threshold
  run_export
  python3 -c "
import sys
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader('export_patterns', '$REPO_ROOT/scripts/export-patterns.py').load_module()
import yaml
with open('$EXPORT_FILE') as f:
    data = yaml.safe_load(f)
for p in data['patterns']:
    score = mod.complexity_score(p['regex'])
    if score >= mod.WARN_THRESHOLD:
        print(f'FAIL: {p[\"id\"]} scored {score} (WARN threshold: {mod.WARN_THRESHOLD})', file=sys.stderr)
        sys.exit(1)
"
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `make test`

Expected: FAIL because `complexity_score` and `WARN_THRESHOLD` don't exist.

- [ ] **Step 3: Implement complexity_score function**

Add to `scripts/export-patterns.py` after `check_redos()`:

```python
# Complexity thresholds — calibrated so all 25 current patterns are SAFE
WARN_THRESHOLD = 5000
REJECT_THRESHOLD = 50000


def complexity_score(regex_str):
    """Score a regex for backtracking complexity.

    score = alternation_branches * max_quantifier_bound * nesting_depth

    Low scores are safe. High scores indicate patterns that could be slow
    on pathological inputs.
    """
    try:
        parsed = sre_parse.parse(regex_str)
    except Exception:
        return 0

    def analyze(node_list, depth=1):
        """Walk AST and compute complexity metrics."""
        branches = 1
        max_bound = 1
        max_depth = depth

        for op, av in node_list:
            if op == sre_parse.BRANCH:
                branch_count = len(av[1])
                branches *= branch_count
                for branch in av[1]:
                    b, m, d = analyze(branch, depth)
                    branches *= b
                    max_bound = max(max_bound, m)
                    max_depth = max(max_depth, d)
            elif op in (sre_parse.MAX_REPEAT, sre_parse.MIN_REPEAT):
                _min, _max, subpattern = av
                bound = _max if _max != sre_parse.MAXREPEAT else 999
                max_bound = max(max_bound, bound)
                b, m, d = analyze(subpattern, depth + 1)
                branches *= b
                max_bound = max(max_bound, m)
                max_depth = max(max_depth, d)
            elif op == sre_parse.SUBPATTERN:
                if av[3]:
                    b, m, d = analyze(av[3], depth)
                    branches *= b
                    max_bound = max(max_bound, m)
                    max_depth = max(max_depth, d)

        return branches, max_bound, max_depth

    branches, max_bound, max_depth = analyze(parsed)
    return branches * max_bound * max_depth
```

- [ ] **Step 4: Wire complexity scoring into the main validation loop**

In `scripts/export-patterns.py`, in `main()`, after the ReDoS check and before `exported.append(...)`:

```python
        # Complexity scoring
        score = complexity_score(regex)
        if score >= REJECT_THRESHOLD:
            print(f"  REJECT: {pid} — complexity score {score} exceeds {REJECT_THRESHOLD}", file=sys.stderr)
            failures += 1
            continue
        if score >= WARN_THRESHOLD:
            print(f"  WARN: {pid} — complexity score {score} (threshold {WARN_THRESHOLD})", file=sys.stderr)

        exported.append({"id": pid, "severity": severity, "regex": regex})
```

- [ ] **Step 5: Calibrate thresholds**

Run: `cd components/openagent-social && python3 -c "
import sys
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader('ep', 'scripts/export-patterns.py').load_module()
patterns = mod.parse_patterns('config/injection-patterns.yml')
scores = []
for p in patterns:
    s = mod.complexity_score(p['regex'])
    scores.append((p['id'], s))
    print(f'{p[\"id\"]:>10}  score={s}')
print(f'\nMax score: {max(s for _, s in scores)}')
print(f'WARN threshold: {mod.WARN_THRESHOLD}')
print(f'REJECT threshold: {mod.REJECT_THRESHOLD}')
"`

Verify: All 25 patterns score below `WARN_THRESHOLD`. If not, adjust thresholds in the code so the invariant holds (all current patterns < WARN, dangerous patterns > REJECT).

- [ ] **Step 6: Run tests**

Run: `make test`

Expected: All pass including `test_all_patterns_below_warn_threshold`.

- [ ] **Step 7: Commit**

```bash
git add scripts/export-patterns.py tests/tools/export-patterns.tool-test.sh
git commit -m "feat: add complexity scoring to pattern export

Scores patterns based on alternation branches, quantifier bounds, and
nesting depth. WARN and REJECT thresholds calibrated against all 25
current patterns.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Add Integrity Hash

Add SHA-256 integrity hash to the export file header.

**Files:**
- Modify: `scripts/export-patterns.py`
- Modify: `tests/tools/export-patterns.tool-test.sh`

- [ ] **Step 1: Write the failing test**

Add to `tests/tools/export-patterns.tool-test.sh`:

```bash
test_export_contains_integrity_hash() {
  run_export
  # The export file should contain an Integrity comment with a sha256 hash
  grep -q '^# Integrity: sha256:' "$EXPORT_FILE"
}

test_integrity_hash_is_valid() {
  run_export
  python3 -c "
import yaml, hashlib, sys, re as re_mod
with open('$EXPORT_FILE') as f:
    lines = f.readlines()
# Extract stored hash from header comment
stored_hash = None
for line in lines:
    m = re_mod.match(r'^# Integrity: sha256:(\w+)', line)
    if m:
        stored_hash = m.group(1)
        break
if not stored_hash:
    print('No integrity hash found', file=sys.stderr)
    sys.exit(1)
# Recompute hash from pattern regexes
with open('$EXPORT_FILE') as f:
    data = yaml.safe_load(f)
hash_input = '\n'.join(p['regex'] for p in sorted(data['patterns'], key=lambda x: x['id']))
computed = hashlib.sha256(hash_input.encode()).hexdigest()
if stored_hash != computed:
    print(f'Hash mismatch: stored={stored_hash[:16]}... computed={computed[:16]}...', file=sys.stderr)
    sys.exit(1)
"
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `make test`

Expected: Both new tests FAIL because the export file doesn't contain a hash line yet.

- [ ] **Step 3: Add integrity hash to export output**

In `scripts/export-patterns.py`, in `main()`, replace the header construction and file writing block (the section starting at `# Write export file with header comment`) with:

```python
    # Ensure output directory exists
    os.makedirs(os.path.dirname(EXPORT_FILE), exist_ok=True)

    # Compute integrity hash over regex content (sorted by ID)
    hash_input = "\n".join(
        p["regex"] for p in sorted(exported, key=lambda x: x["id"])
    )
    integrity = hashlib.sha256(hash_input.encode()).hexdigest()

    # Write export file with header comment
    header = (
        "# Generated by: make export-patterns\n"
        "# Source: config/injection-patterns.yml\n"
        f"# Count: {len(exported)} patterns\n"
        f"# Integrity: sha256:{integrity}\n"
    )
    body = yaml.dump(
        {"patterns": exported},
        default_flow_style=False,
        sort_keys=False,
        allow_unicode=True,
    )

    with open(EXPORT_FILE, "w") as f:
        f.write(header)
        f.write(body)
```

Note: `import hashlib` was already added in Task 1 Step 3.

- [ ] **Step 4: Run tests**

Run: `make test`

Expected: All pass including both new hash tests.

- [ ] **Step 5: Commit**

```bash
git add scripts/export-patterns.py tests/tools/export-patterns.tool-test.sh
git commit -m "feat: add SHA-256 integrity hash to pattern export

Hash covers regex content sorted by ID. Vault-proxy.py will verify
this hash at startup to detect pattern file tampering.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Add Pathological Input Test

Add a test that runs all exported patterns against adversarial input to verify performance.

**Files:**
- Modify: `tests/tools/export-patterns.tool-test.sh`

- [ ] **Step 1: Write the test**

Add to `tests/tools/export-patterns.tool-test.sh`:

```bash
test_patterns_complete_on_pathological_input() {
  run_export
  python3 -c "
import re, yaml, time, sys
with open('$EXPORT_FILE') as f:
    data = yaml.safe_load(f)
# 10KB of repeated 'a' characters — worst case for backtracking
pathological = 'a' * 10240
start = time.monotonic()
for p in data['patterns']:
    re.search(p['regex'], pathological)
elapsed = time.monotonic() - start
if elapsed > 1.0:
    print(f'Pathological test took {elapsed:.2f}s (limit 1.0s)', file=sys.stderr)
    sys.exit(1)
"
}
```

- [ ] **Step 2: Run test**

Run: `make test`

Expected: PASS — all 25 patterns complete in well under 1 second on 10KB input.

- [ ] **Step 3: Commit**

```bash
git add tests/tools/export-patterns.tool-test.sh
git commit -m "test: add pathological input benchmark for pattern safety

Runs all 25 patterns against 10KB of repeated 'a' characters.
Verifies completion within 1 second (current patterns complete in <1ms).

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Final Verification and Roadmap Update

Run all tests, update roadmap.

**Files:**
- Modify: `docs/roadmap.md`

- [ ] **Step 1: Run full test suite**

Run: `make test`

Expected: All tests pass. Count should be 29 (24 existing + 2 ReDoS tests + 2 integrity tests + 1 pathological test).

- [ ] **Step 2: Run export and verify output**

Run: `make export-patterns && head -5 data/patterns-export.yml`

Expected output:
```
Exported 25 patterns to .../data/patterns-export.yml
# Generated by: make export-patterns
# Source: config/injection-patterns.yml
# Count: 25 patterns
# Integrity: sha256:<64-char hex digest>
```

- [ ] **Step 3: Update roadmap**

In `docs/roadmap.md`, add a note to the Phase 4 section indicating the regex security hardening is complete:

After the Phase 4 table, add:

```markdown
**Regex security hardening (2026-04-05):** ReDoS static analysis via `re._parser`, complexity scoring with WARN/REJECT thresholds, SHA-256 integrity hash in export, pathological input benchmark. Spec: `docs/specs/2026-04-05-regex-security-hardening.md`. Vault-side runtime protections (Layers 2-4) deferred to Phase C.
```

- [ ] **Step 4: Commit**

```bash
git add docs/roadmap.md
git commit -m "docs: update roadmap with regex security hardening completion

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 5: Clean up generated files**

```bash
rm -f data/patterns-export.yml data/census-*.json
```

---

## Verification Checklist

After all tasks complete:

- [ ] `make test` — all 29 tests pass
- [ ] `make export-patterns` — 25 patterns exported with integrity hash
- [ ] Known ReDoS pattern `(a+)+$` is rejected by `check_redos()`
- [ ] All 25 current patterns pass ReDoS check
- [ ] All 25 current patterns score below WARN threshold
- [ ] Integrity hash in export file validates correctly
- [ ] Pathological input (10KB) completes in < 1 second
- [ ] No changes to `config/injection-patterns.yml` (source patterns untouched)
