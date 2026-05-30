# Spec: Regex Security Hardening for Feed Scanning

**Date:** 2026-04-05
**Status:** Layer 1 implemented (pioneer), Layers 2-4 pending (Phase C, vault)
**Scope:** Pioneer (export-time validation) + Vault (runtime protection)
**Cross-reference:** `docs/specs/2026-04-04-vault-integration-design.md` (parent integration spec)

## Problem

Pioneer's 25 injection patterns will be compiled and executed by vault-proxy.py against untrusted Moltbook API response content. Three attack surfaces exist:

1. **ReDoS** — a pattern with catastrophic backtracking hangs the proxy when matched against attacker-crafted content, denying service to the agent
2. **Evasion** — Unicode tricks (homoglyphs, zero-width characters, decomposed forms) break regex word matching, allowing injections to pass undetected
3. **Tampering** — a modified `patterns-export.yml` disables detection or introduces malicious patterns

All 25 current patterns are ReDoS-safe (bounded quantifiers, no nested quantifiers). This spec prevents regressions and hardens the runtime against edge cases.

## Architecture: Four Defense Layers

```
Pattern author writes regex in injection-patterns.yml
    |
    v
Layer 1: STATIC VALIDATION (pioneer, export-time)
    - sre_parse AST analysis for ReDoS patterns
    - Complexity scoring with reject threshold
    - Pathological input benchmark
    - Integrity hash appended to export file
    |
    v
patterns-export.yml (with integrity hash)
    |
    v
Layer 4: INTEGRITY VERIFICATION (vault, startup + verify.sh)
    - SHA-256 hash check on load
    - Refuse to load on mismatch
    |
    v
Moltbook API response arrives
    |
    v
Layer 2: CONTENT NORMALIZATION (vault, response-time)
    - Unicode NFC normalization
    - Strip zero-width characters
    |
    v
Layer 3: RUNTIME PROTECTION (vault, match-time)
    - 1-second per-pattern thread-based timeout
    - 10-second total scan timeout
    - Timeout = CRITICAL finding = block response
```

Each layer is independent. If any single layer fails, the others still protect.

## Layer 1: Static Validation (Pioneer)

**Component:** `scripts/export-patterns.py`
**When:** At export time (`make export-patterns`)

### ReDoS Detection

After the existing `re.compile()` check, analyze the compiled pattern's AST via Python's `sre_parse` module. Flag patterns containing:

- **Nested quantifiers:** A quantified group inside another quantified group (e.g., `(a+)+`, `(a*){2,}`)
- **Unbounded repetition on alternation:** `(a|b)*` or `(a|b)+` without an upper bound
- **Overlapping quantifiers:** Adjacent quantifiers on overlapping character sets (e.g., `.*.*`, `\w+\w+`)

> **Not implemented.** Adjacent quantifiers without nesting are not a ReDoS vector in CPython — the engine does not backtrack between consecutive quantifiers on flat character sets. The pathological input benchmark (test 11, 10KB input) provides runtime protection if a future pattern accidentally combines adjacent quantifiers with anchoring. See Gap A in `docs/report-regex-verification.md`.

Patterns matching any of these are **rejected** (not exported, exit 1).

### Complexity Scoring

Each pattern receives a numeric complexity score:

```
score = alternation_branches * max_quantifier_bound * nesting_depth
```

Where:
- `alternation_branches` = product of branch counts across all alternation groups
- `max_quantifier_bound` = highest upper bound in any quantifier (unbounded = 999)
- `nesting_depth` = deepest nesting of groups with quantifiers

Thresholds will be calibrated during implementation by scoring all 25 current patterns and setting:
- **WARN** threshold above the highest current score (all current patterns must be SAFE)
- **REJECT** threshold at a level that catches genuinely dangerous structures

**Invariant:** All 25 current patterns must score below the WARN threshold. The REJECT threshold catches patterns where multi-character alternation branches (BRANCH nodes in `sre_parse`) combine with high quantifier bounds and nesting depth, producing a large `branches * bound * depth` product.

> **Note on scoring of single-character alternations:** Python's `sre_parse` compiles `(a|b|c)` into a character class (IN node), not a BRANCH node. These match deterministically in O(1) per position and score as `branches=1` regardless of the number of alternatives. This is correct behavior — such patterns are not ReDoS-vulnerable. See Gap B in `docs/report-regex-verification.md`.

### Pathological Input Test

New test in `tests/tools/export-patterns.tool-test.sh`:

```bash
test_patterns_complete_on_pathological_input() {
  # Run each pattern against 10KB of repeated 'a' characters
  # All must complete within 1 second total
  python3 -c "
import re, yaml, time
with open('$EXPORT_FILE') as f:
    data = yaml.safe_load(f)
pathological = 'a' * 10240
start = time.monotonic()
for p in data['patterns']:
    re.search(p['regex'], pathological)
elapsed = time.monotonic() - start
if elapsed > 1.0:
    raise SystemExit(f'Pathological test took {elapsed:.2f}s (limit 1s)')
"
}
```

### Integrity Hash

The export script appends a SHA-256 hash as a YAML comment:

```yaml
# Generated by: make export-patterns
# Source: config/injection-patterns.yml
# Count: 25 patterns
# Integrity: sha256:<hex-digest>
patterns:
  - id: auth-001
    ...
```

Hash computation: sort pattern regexes by ID, concatenate with newline separator, SHA-256 the result. This means only regex changes invalidate the hash — id/severity metadata changes are tracked but don't trigger integrity failures.

```python
hash_input = "\n".join(p["regex"] for p in sorted(exported, key=lambda x: x["id"]))
integrity = hashlib.sha256(hash_input.encode()).hexdigest()
```

## Layer 2: Content Normalization (Vault)

**Component:** `vault-proxy.py` response processing
**When:** After JSON content extraction, before pattern matching

### Normalization Pipeline

Applied to extracted text content (post body, comment text), not the raw HTTP response:

1. **Unicode NFC normalization**
   ```python
   import unicodedata
   text = unicodedata.normalize('NFC', text)
   ```
   Collapses composed/decomposed forms. `\u00e9` (precomposed é) and `\u0065\u0301` (e + combining accent) become the same string.

2. **Strip zero-width characters**
   ```python
   ZERO_WIDTH = set('\u200b\u200c\u200d\u2060\u200e\u200f\ufeff')
   text = ''.join(c for c in text if c not in ZERO_WIDTH)
   ```
   Removes invisible characters that break word matching (e.g., `ig\u200bnore` → `ignore`).

### What Is NOT Normalized (v1)

- **Homoglyph folding** — Mapping Cyrillic а/е/о/с to Latin a/e/o/c. Too many edge cases, high false-positive risk on legitimate non-Latin content. Documented as a known limitation.
- **HTML entity decoding** — Moltbook API returns JSON, not HTML. If this changes, revisit.
- **Case normalization** — Handled by `(?i)` flags in the patterns themselves.

### Audit Trail

The raw (un-normalized) response is logged to `requests.jsonl`. The normalized version is used only for pattern matching. This preserves forensic evidence.

## Layer 3: Runtime Protection (Vault)

**Component:** `vault-proxy.py` pattern matching
**When:** During regex execution against normalized content

### Thread-Based Timeout

```python
import threading

def match_with_timeout(compiled_regex, text, timeout_seconds=1.0):
    """Run regex match with timeout. Returns match or None. Raises TimeoutError."""
    result = [None]
    exception = [None]

    def _search():
        try:
            result[0] = compiled_regex.search(text)
        except Exception as e:
            exception[0] = e

    thread = threading.Thread(target=_search, daemon=True)
    thread.start()
    thread.join(timeout=timeout_seconds)

    if thread.is_alive():
        raise TimeoutError(f"Regex exceeded {timeout_seconds}s")

    if exception[0]:
        raise exception[0]

    return result[0]
```

### Timeout Policy

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Per-pattern timeout | 1 second | Current patterns complete in < 1ms; 1s is 1000x headroom |
| Total scan timeout | 10 seconds | 25 patterns at worst-case 1s each would be 25s; 10s cap prevents cascade |
| On timeout action | Block response (CRITICAL) | Hanging regex = either malicious content or bad pattern; block is the safe default |

### Logging

Timeout events are logged to `requests.jsonl`:

```json
{
  "action": "REGEX_TIMEOUT",
  "pattern_id": "inj-001",
  "text_length": 4096,
  "timeout_seconds": 1.0,
  "timestamp": "2026-04-15T10:30:00Z"
}
```

### Thread Safety

mitmproxy handles requests via its own event loop. The regex thread is:
- Short-lived (1s max)
- Daemon thread (won't block proxy shutdown)
- Shares no mutable state with mitmproxy (result list is thread-local)
- GIL-safe (Python's GIL serializes access to the result list)

### Note on Stuck Threads

Python cannot forcibly kill a thread running a CPU-bound regex match. If a pattern causes a true infinite backtrack, the thread will continue running in the background after the 1-second timeout. The daemon flag ensures it won't prevent proxy shutdown. In practice, mitmproxy's response hook returns after the timeout (blocking the response), and the orphaned thread eventually completes or is cleaned up at process exit.

For production hardening, consider monitoring thread count — if orphaned threads accumulate, the pattern file needs fixing.

## Layer 4: Integrity Verification (Vault)

**Component:** `vault-proxy.py` startup + `scripts/verify.sh`

### Startup Hash Check

When loading `patterns-export.yml`, vault-proxy.py:

1. Reads the `# Integrity: sha256:<hash>` comment line
2. Recomputes SHA-256 over the loaded regex strings (sorted by ID, concatenated)
3. Compares stored vs computed hash

On mismatch:
- Log `PATTERN_INTEGRITY_FAIL` with both hashes
- **Refuse to load patterns** — fall back to no scanning (all responses pass through)
- This is the safe default: no scanning is better than tampered scanning

### verify.sh Check #25

```
Check #25: Pattern file integrity
  Condition: MOLTBOOK_PATTERNS env var is set and points to a file
  Checks:
    - Integrity hash matches content
    - All regexes compile in Python re
    - Pattern count reported
  Skip if: MOLTBOOK_PATTERNS not set (feed scanning inactive)
```

This extends the existing 24-point verification suite.

## Known Limitations (v1)

1. **Homoglyph evasion** — Cyrillic/Latin substitution (е→e, а→a, о→o) can bypass patterns. Requires a maintained mapping table with multilingual false-positive testing. Deferred to v2.

2. **Multi-post injection** — Attacker splits an injection across multiple posts/comments. Pattern matching is per-post. Deferred — requires semantic analysis beyond regex.

3. **Stuck threads** — Python cannot kill a running thread. Orphaned regex threads consume CPU until completion. Monitoring recommended for production.

4. **RE2 migration** — Google RE2 guarantees O(n) matching but doesn't support lookahead (`url-003` uses negative lookahead). Could be revisited if pattern set grows significantly.

## What Pioneer Builds (Now)

| Deliverable | File |
|-------------|------|
| ReDoS detection via sre_parse | `scripts/export-patterns.py` |
| Complexity scoring + reject threshold | `scripts/export-patterns.py` |
| Integrity hash in export output | `scripts/export-patterns.py` |
| Pathological input test | `tests/tools/export-patterns.tool-test.sh` |
| This spec | `docs/specs/2026-04-05-regex-security-hardening.md` |

## What Vault Builds (Phase C)

| Deliverable | File |
|-------------|------|
| Content normalization (NFC + zero-width strip) | `proxy/vault-proxy.py` |
| Thread-based regex timeout | `proxy/vault-proxy.py` |
| Integrity hash verification at startup | `proxy/vault-proxy.py` |
| Check #25 in verification suite | `scripts/verify.sh` |

## Testing Strategy

### Pioneer-side (offline, run now)

- All 25 patterns pass ReDoS static analysis
- All 25 patterns score below WARN threshold
- Pathological input test (10KB of `a` characters) completes in < 1 second
- Export file contains valid integrity hash
- Integrity hash round-trips correctly (export → load → recompute → match)
- Known ReDoS pattern (e.g., `(a+)+$`) is rejected by static analysis

### Vault-side (container, Phase C)

- Thread timeout triggers on an intentionally slow pattern (test-only)
- Content normalization strips zero-width characters from test fixture
- Content normalization NFC-normalizes test fixture
- Integrity check passes on valid export file
- Integrity check fails (refuses to load) on tampered export file
- Check #25 passes in verify.sh when patterns loaded
- Check #25 skips when MOLTBOOK_PATTERNS not set

## Security Considerations

- Pattern file is read-only to the agent (host-side, loaded by proxy)
- Agent cannot influence normalization or timeout behavior
- Integrity hash prevents silent tampering of the pattern file
- Timeout-as-CRITICAL ensures no silent failures — stuck patterns are visible and actionable
- Static validation gives pattern authors immediate feedback — no need to deploy to discover a bad pattern

---

*This spec extends the feed scanning integration designed in `2026-04-04-vault-integration-design.md`. Implementation timeline: pioneer-side now, vault-side during Phase C of the master roadmap.*
