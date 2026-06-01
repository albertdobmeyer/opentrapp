#!/usr/bin/env bash
# Adapter abstraction tests (spec 04 §2a).
#
# Headline property: the scanner CORE has zero Moltbook hostname/string
# references in its own path — all Moltbook coupling lives exclusively in the
# Moltbook adapter under tools/lib/adapters/.
#
# These tests are Ollama-free: mock adapter only, no network.
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCIAL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ADAPTERS_DIR="$SOCIAL_ROOT/tools/lib/adapters"
SCANNER="$SOCIAL_ROOT/tools/feed-scanner.sh"
CENSUS="$SOCIAL_ROOT/tools/agent-census.sh"
FIXTURES="$SCRIPT_DIR/fixtures"

PASS=0; FAIL=0; SKIP=0
pass() { echo -e "  \033[0;32mPASS\033[0m  $1"; PASS=$((PASS+1)); }
fail() { echo -e "  \033[0;31mFAIL\033[0m  $1"; FAIL=$((FAIL+1)); }
skip() { echo -e "  \033[0;33mSKIP\033[0m  $1"; SKIP=$((SKIP+1)); }

echo ""
echo "=== Adapter abstraction tests (spec 04 §2a) ==="
echo ""

# ── §2a property 1: adapter files exist ─────────────────────────────────────

if [[ -f "$ADAPTERS_DIR/moltbook.sh" ]]; then
  pass "Moltbook adapter exists: tools/lib/adapters/moltbook.sh"
else
  fail "Moltbook adapter missing: tools/lib/adapters/moltbook.sh"
fi

if [[ -f "$ADAPTERS_DIR/mock.sh" ]]; then
  pass "Mock adapter exists: tools/lib/adapters/mock.sh"
else
  fail "Mock adapter missing: tools/lib/adapters/mock.sh"
fi

if [[ -f "$ADAPTERS_DIR/file.sh" ]]; then
  pass "File adapter exists: tools/lib/adapters/file.sh"
else
  fail "File adapter missing: tools/lib/adapters/file.sh"
fi

# ── §2a property 2: scanner core has zero Moltbook hostname references ───────
# "Core path" = feed-scanner.sh itself (the hardcoded Moltbook HTTP calls must
# have moved entirely into the Moltbook adapter). We grep for the canonical
# hostname and the bare name as a string literal.
MOLTBOOK_PATTERNS=(
  'api\.moltbook\.com'
  'moltbook\.com'
  'MOLTBOOK_API_BASE'
)

scanner_clean=true
for pat in "${MOLTBOOK_PATTERNS[@]}"; do
  if grep -qE "$pat" "$SCANNER" 2>/dev/null; then
    fail "scanner core still contains Moltbook reference: $pat (in feed-scanner.sh)"
    scanner_clean=false
  fi
done
if $scanner_clean; then
  pass "feed-scanner.sh core path is free of Moltbook endpoint strings"
fi

# Same check for agent-census.sh
census_clean=true
for pat in "${MOLTBOOK_PATTERNS[@]}"; do
  if grep -qE "$pat" "$CENSUS" 2>/dev/null; then
    fail "agent-census.sh still contains Moltbook reference: $pat"
    census_clean=false
  fi
done
if $census_clean; then
  pass "agent-census.sh core path is free of Moltbook endpoint strings"
fi

# Moltbook references should live ONLY in the Moltbook adapter (positive check)
if grep -qE 'moltbook\.com' "$ADAPTERS_DIR/moltbook.sh" 2>/dev/null; then
  pass "Moltbook adapter contains the Moltbook endpoint (coupling is isolated)"
else
  fail "Moltbook adapter does not reference moltbook.com — adapter may be empty/stub"
fi

# ── §2a property 3: mock adapter produces normalised posts ───────────────────
# Source the mock adapter and call fetch_feed; verify output is valid JSON with
# the required {id, author, content, timestamp} shape.
if [[ ! -f "$ADAPTERS_DIR/mock.sh" ]]; then
  skip "mock adapter missing — skipping normalised-post shape tests"
else
  mock_out=$(bash "$ADAPTERS_DIR/mock.sh" fetch_feed '{}' 2>/dev/null || true)

  if [[ -z "$mock_out" ]]; then
    fail "mock adapter fetch_feed returned empty output"
  else
    # Validate JSON and check required fields
    shape_ok=$(python3 -c "
import sys, json
try:
    posts = json.loads('''$mock_out''')
    assert isinstance(posts, list) and len(posts) > 0, 'not a non-empty list'
    required = {'id', 'author', 'content', 'timestamp'}
    for p in posts:
        missing = required - set(p.keys())
        assert not missing, f'post missing fields: {missing}'
    print('ok')
except Exception as e:
    print(f'fail: {e}')
" 2>/dev/null || echo "fail: python3 error")

    if [[ "$shape_ok" == "ok" ]]; then
      pass "mock adapter fetch_feed returns normalised [{id,author,content,timestamp}] posts"
    else
      fail "mock adapter fetch_feed normalisation check: $shape_ok"
    fi
  fi

  # fetch_agent
  agent_out=$(bash "$ADAPTERS_DIR/mock.sh" fetch_agent 'test-agent' 2>/dev/null || true)
  if [[ -n "$agent_out" ]]; then
    pass "mock adapter fetch_agent returns output"
  else
    fail "mock adapter fetch_agent returned empty output"
  fi

  # stats
  stats_out=$(bash "$ADAPTERS_DIR/mock.sh" stats '{}' 2>/dev/null || true)
  if python3 -c "import sys,json; d=json.loads('''$stats_out'''); assert isinstance(d,dict)" 2>/dev/null; then
    pass "mock adapter stats returns a JSON object"
  else
    fail "mock adapter stats did not return a JSON object"
  fi
fi

# ── §2a property 4: file adapter wraps existing file mode ────────────────────
if [[ -f "$ADAPTERS_DIR/file.sh" ]]; then
  file_out=$(bash "$ADAPTERS_DIR/file.sh" fetch_feed "{\"path\":\"$FIXTURES/clean-posts.json\"}" 2>/dev/null || true)
  shape_ok=$(python3 -c "
import sys, json
try:
    posts = json.loads('''$file_out''')
    assert isinstance(posts, list) and len(posts) > 0, 'not a non-empty list'
    required = {'id', 'author', 'content', 'timestamp'}
    for p in posts:
        missing = required - set(p.keys())
        assert not missing, f'post missing fields: {missing}'
    print('ok')
except Exception as e:
    print(f'fail: {e}')
" 2>/dev/null || echo "fail: python3 error")

  if [[ "$shape_ok" == "ok" ]]; then
    pass "file adapter fetch_feed normalises clean-posts.json to {id,author,content,timestamp}"
  else
    fail "file adapter fetch_feed normalisation: $shape_ok"
  fi
fi

# ── §2a property 5: scanner accepts --adapter mock ───────────────────────────
if bash "$SCANNER" --help 2>&1 | grep -q '\-\-adapter'; then
  pass "feed-scanner.sh --help advertises --adapter option"
else
  fail "feed-scanner.sh does not expose --adapter option"
fi

if bash "$SCANNER" --adapter mock --recent 5 >/dev/null 2>&1; then
  pass "feed-scanner.sh --adapter mock --recent 5 exits 0"
else
  fail "feed-scanner.sh --adapter mock --recent 5 failed"
fi

# ── §2a property 6: no regression — malicious/clean fixtures still work ──────
if bash "$SCANNER" --adapter file --file "$FIXTURES/malicious-posts.json" >/dev/null 2>&1; then
  fail "malicious posts should cause exit 1 (CRITICAL findings)"
else
  pass "feed-scanner.sh --adapter file: malicious-posts.json exits non-zero (regression check)"
fi

if bash "$SCANNER" --adapter file --file "$FIXTURES/clean-posts.json" >/dev/null 2>&1; then
  pass "feed-scanner.sh --adapter file: clean-posts.json exits 0 (regression check)"
else
  fail "clean-posts.json should exit 0"
fi

# ── §2a property 7: census accepts --adapter flag ───────────────────────────
if bash "$CENSUS" --help 2>&1 | grep -q '\-\-adapter'; then
  pass "agent-census.sh --help advertises --adapter option"
else
  fail "agent-census.sh does not expose --adapter option"
fi

if bash "$CENSUS" --adapter mock >/dev/null 2>&1; then
  pass "agent-census.sh --adapter mock exits 0"
else
  fail "agent-census.sh --adapter mock failed"
fi

echo ""
echo "Results: $PASS passed, $FAIL failed, $SKIP skipped"
[[ "$FAIL" -eq 0 ]]
