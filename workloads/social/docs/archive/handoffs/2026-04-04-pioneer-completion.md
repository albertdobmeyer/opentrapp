# Handoff: OpenAgent-Social Completion

**Date:** 2026-04-04
**From:** Ecosystem harmonization session
**To:** Next instance completing pioneer to release quality

## What This Module Is

OpenAgent-Social is the **network safety layer** of the OpenTrApp trifecta. It provides three standalone bash tools for safe observation and participation in the Moltbook agentic social network (~1.5M registered agents, ~201K verified, acquired by Meta March 2026, untrusted feed content).

| Tool | Purpose | Lines |
|------|---------|-------|
| `feed-scanner.sh` | Scan feed posts for prompt injection patterns | 381 |
| `agent-census.sh` | Pull platform statistics and track trends | 233 |
| `identity-checklist.sh` | Pre-flight safety check before agent registration | 204 |

**Role in ecosystem**: `network` — the outermost layer. Vault isolates the runtime, forge secures the supply chain, pioneer scans the social feed.

## Current State: 70% Complete

**What works (standalone):**
- Three tools exist and implement their core logic
- 25 injection patterns across 6 categories (YAML-based)
- Comprehensive threat model and safe participation docs
- component.yml manifest for GUI integration
- .env-based config with rate limits

**What's broken:**
- CRLF line endings on every file (shebangs break on Linux)
- No executable bits on scripts (permission denied on clone)
- `eval` in curl on line 197 of feed-scanner.sh (injection surface)
- `safe_patterns` config key silently ignored by scanner
- Zero automated tests
- No Makefile (unlike vault and forge)
- Pattern count claims 30, actual count is 25

**What doesn't exist:**
- Test framework and test files
- `--dry-run` / offline mode
- Vault integration (proxy-level feed scanning)
- Pattern export mechanism for vault-proxy.py

## What Other Modules Expect

### From Vault (future — Phase C of master roadmap)

When Moltbook domains enter the Soft Shell allowlist, vault-proxy.py needs to check API responses for injection patterns. Pioneer must:
1. Export its 25 patterns in a machine-readable format (JSON or shared YAML)
2. vault-proxy.py loads them and scans Moltbook response bodies
3. Flagged content is logged; agent never sees it

**Not blocking anything now** — Moltbook domains aren't in the allowlist. But the pattern format should be designed with export in mind.

### From Forge

No direct dependency. Forge has 87 skill-focused patterns (bash functions), pioneer has 25 social-focused patterns (YAML). Different threat surfaces, different formats. No convergence needed unless maintenance burden justifies it.

### From the GUI

The GUI expects 6 commands from component.yml to work reliably:
- `feed-scan`, `feed-scan-agent`, `agent-census`, `census-trend`, `identity-check`, `setup`
- All must exit cleanly, produce readable output, and work after a fresh clone

## Bugs to Fix (Priority Order)

### Bug 1: CRLF Line Endings (CRITICAL)

Every file has Windows line endings (`\r\n`). On Linux, `#!/usr/bin/env bash\r` resolves to `bash\r` which doesn't exist, so the script runs under `sh` and `set -euo pipefail` fails with exit code 2.

**Fix:** `sed -i 's/\r$//' tools/*.sh config/*.yml` + add `.gitattributes` with `*.sh text eol=lf`.

**Verify:** `file tools/*.sh` shows "Bourne-Again shell script" (not "with CRLF line terminators").

### Bug 2: No Executable Bits

All three scripts are tracked as `100644`. Fresh clone = permission denied.

**Fix:** `chmod +x tools/*.sh` + `git add tools/*.sh` (git tracks the mode change).

**Verify:** `git ls-tree HEAD tools/` shows `100755`.

### Bug 3: eval in curl (Security)

Line 197 of `feed-scanner.sh`:
```bash
if ! eval curl -sf "${MOLTBOOK_API_BASE}/posts?limit=${TARGET}" "$auth_header" > "$tmp_file" 2>/dev/null; then
```

If `.env` contains shell metacharacters in `MOLTBOOK_API_KEY`, this is an injection vector.

**Fix:** Replace with array-based approach:
```bash
local curl_args=(-sf "${MOLTBOOK_API_BASE}/posts?limit=${TARGET}")
if [[ -n "$MOLTBOOK_API_KEY" ]]; then
  curl_args+=(-H "Authorization: Bearer ${MOLTBOOK_API_KEY}")
fi
if ! curl "${curl_args[@]}" > "$tmp_file" 2>/dev/null; then
```

### Bug 4: safe_patterns Silently Ignored

`config/feed-allowlist.yml` declares two safe patterns (research meta-discussion, bot greetings). `load_allowlist()` only reads `trusted_agents`. The `safe_patterns` key is dead config.

**Fix:** Parse `safe_patterns` entries alongside trusted agents. Add an `is_safe_content()` check before flagging findings — if content matches a safe pattern, skip the finding.

### Bug 5: fetch_posts() Status Messages Captured in Return Value

`fetch_posts()` uses `echo` for both status messages ("Scanning file: ...") and the return value (temp file path). The caller captures everything: `posts_file=$(fetch_posts)`. Result: `posts_file` contains status text + path, Python can't parse the path, `post_count` is 0.

**Fix:** Send status messages to stderr: `echo -e "..." >&2`.

### Bug 6: Pattern Count Discrepancy

README, CLAUDE.md, and roadmap all say "30 patterns." Actual count in `injection-patterns.yml` is 25. The example doc (`examples/feed-analysis.md`) shows "28 patterns."

**Fix:** Either add 5 more patterns to reach 30, or update all docs to say 25. The 25 existing patterns cover 6 categories well — updating docs to 25 is the honest choice.

### Bug 7: Dead Variable

Line 106 of `feed-scanner.sh`: `local in_pattern=false` — declared, never used.

**Fix:** Remove.

## Features to Add

### 1. Test Framework + Tests

Model on forge's `tests/_framework/` (tool-runner.sh + tool-assertions.sh). The assertion primitives are generic and can be copied verbatim from openskill-forge.

**Minimum tests:**

feed-scanner.sh (~10 tests):
- `--help` exits 0, output contains "Usage"
- `--bogus` exits 1
- `--file /nonexistent` exits 1
- `--file clean-posts.json` exits 0, shows clean count
- `--file malicious-posts.json` exits 1, output contains "CRITICAL"
- `--file empty-posts.json` exits 0 (graceful)
- `--file safe-research.json` exits 0 (safe pattern suppresses finding)
- `--verbose` shows matched content

agent-census.sh (~3 tests):
- `--help` exits 0
- `--bogus` exits 1
- Output contains "Usage"

identity-checklist.sh (~3 tests):
- Runs without crash (exit 0 or 1, not 2)
- Output contains "Configuration" section
- Output contains "Pre-Flight Checklist"

**Fixtures needed:**
- `tests/fixtures/clean-posts.json` — 3 benign posts
- `tests/fixtures/malicious-posts.json` — 4 posts with injection patterns
- `tests/fixtures/safe-research-posts.json` — 1 post discussing injection research (must actually trigger a pattern, then be suppressed by safe_patterns)
- `tests/fixtures/empty-posts.json` — empty array

**Important:** The safe-research fixture must contain content that genuinely matches an injection pattern (e.g., include "ignore all previous instructions" as a quoted example in a research discussion). Otherwise the test passes vacuously.

### 2. Makefile

Standard targets matching vault/forge conventions:

```makefile
help            # Show available commands
scan            # Scan recent feed (alias for feed-scanner --recent)
scan-agent      # Scan specific agent
census          # Pull census
census-trend    # Show census trends
checklist       # Run identity checklist
test-tools      # Run tool test suite
setup           # Copy .env.example, create data/
verify          # Verify workbench health (config exists, tools executable, patterns load)
```

### 3. Offline Mode (Deferred — Phase 3 of pioneer roadmap)

`--file` already exists for feed-scanner. Consider `--file` for census too (mock API response). Not blocking for Phase 1/2, but needed for CI.

## Reading Order

1. **This document** — you're here
2. **`CLAUDE.md`** — project conventions and manifest rules
3. **`TODO.md`** — the original audit (still accurate)
4. **`docs/roadmap.md`** — 5-phase plan
5. **`docs/threat-landscape.md`** — understand what we're defending against
6. **`tools/feed-scanner.sh`** — the most complex tool, most bugs
7. **`config/injection-patterns.yml`** — the 25 patterns
8. **`config/feed-allowlist.yml`** — safe_patterns that need wiring
9. **`component.yml`** — what the GUI expects
10. **Forge's test framework** — `openskill-forge/tests/_framework/` (model for our tests)

## Development Principles

1. **One task at a time** — validate each fix before moving on
2. **Fix bugs before adding features** — CRLF/chmod first, then tests
3. **Each fix is a separate commit** — matching vault/forge discipline
4. **Test what you claim** — if a test says "safe pattern suppresses finding," the fixture must actually trigger a finding first
5. **Honest counts** — if we have 25 patterns, say 25, not 30

## Component.yml Issues to Fix

1. **`identity-check` availability** — currently `available_when: [ready]` but the checklist's purpose is pre-registration guidance. Should also be available in `not_setup` state.
2. **`error` state unreachable** — declared in `status.states` but no probe rule maps to it. Either add a probe rule or remove the state.

## Commit Order

1. `fix: convert CRLF to LF, add .gitattributes`
2. `fix: set executable bits on tool scripts`
3. `fix: replace eval with array-based curl in feed-scanner`
4. `fix: send fetch_posts status messages to stderr`
5. `fix: remove dead in_pattern variable`
6. `feat: wire safe_patterns from feed-allowlist.yml`
7. `docs: correct pattern count from 30 to 25`
8. `fix: identity-check available in not_setup state, remove unreachable error state`
9. `feat: add Makefile with standard targets`
10. `feat: add test framework and fixture-based tests`
11. `docs: update CLAUDE.md — remove Windows paths, reflect completion`

---

*Cross-reference: `opentrapp/docs/superpowers/plans/2026-04-04-master-roadmap-v3.md` Phase A describes this work at the ecosystem level.*
