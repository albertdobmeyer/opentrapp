#!/usr/bin/env bash
# CDR pipeline tests — validates end-to-end CDR behavior
# Tests requiring Ollama are marked and skip if Ollama is not running.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found"; exit 1
}

FIXTURES="$SCRIPT_DIR/cdr-fixtures"
PASS=0
FAIL=0
SKIP=0
ERRORS=()

pass() { echo -e "  \033[0;32mPASS\033[0m  $1"; PASS=$((PASS + 1)); }
fail() { echo -e "  \033[0;31mFAIL\033[0m  $1"; FAIL=$((FAIL + 1)); ERRORS+=("$1"); }
skip() { echo -e "  \033[0;33mSKIP\033[0m  $1"; SKIP=$((SKIP + 1)); }

OLLAMA_UP=false
curl -sf --max-time 3 "http://localhost:11434/api/tags" > /dev/null 2>&1 && OLLAMA_UP=true

echo ""
echo "=== CDR Pipeline Tests ==="
echo ""

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# ── Test 1: Parser accepts clean skill ──
if "$PY" "$REPO_ROOT/tools/lib/cdr-parse.py" "$FIXTURES/clean-skill.md" > "$TMPDIR/out.json" 2>&1; then
  sections=$("$PY" -c "import json; print(len(json.load(open('$TMPDIR/out.json'))['sections']))")
  if (( sections >= 2 )); then
    pass "Parser extracts sections from clean skill ($sections sections)"
  else
    fail "Parser extracted too few sections ($sections)"
  fi
else
  fail "Parser rejected clean skill"
fi

# ── Test 2: Parser rejects no-frontmatter ──
if "$PY" "$REPO_ROOT/tools/lib/cdr-parse.py" "$FIXTURES/no-frontmatter.md" > /dev/null 2>&1; then
  fail "Parser should reject file without frontmatter"
else
  pass "Parser rejects no-frontmatter file"
fi

# ── Test 3: Pre-filter rejects injected skill ──
if "$PY" "$REPO_ROOT/tools/lib/cdr-parse.py" "$FIXTURES/injected-skill.md" > "$TMPDIR/injected.json" 2>&1; then
  if bash "$REPO_ROOT/tools/lib/cdr-prefilter.sh" "$TMPDIR/injected.json" > /dev/null 2>&1; then
    fail "Pre-filter should reject skill with curl|bash pattern"
  else
    pass "Pre-filter rejects injected skill (forbidden pattern)"
  fi
else
  fail "Parser failed on injected skill (should parse, pre-filter should reject)"
fi

# ── Test 4: Pre-filter passes clean skill ──
if "$PY" "$REPO_ROOT/tools/lib/cdr-parse.py" "$FIXTURES/clean-skill.md" > "$TMPDIR/clean.json" 2>&1; then
  if bash "$REPO_ROOT/tools/lib/cdr-prefilter.sh" "$TMPDIR/clean.json" > "$TMPDIR/filtered.json" 2>&1; then
    sections=$("$PY" -c "import json; print(len(json.load(open('$TMPDIR/filtered.json'))['sections']))")
    if (( sections >= 2 )); then
      pass "Pre-filter passes clean skill ($sections sections preserved)"
    else
      fail "Pre-filter stripped too much from clean skill"
    fi
  else
    fail "Pre-filter rejected clean skill"
  fi
fi

# ── Test 5: Intent validation accepts valid intent ──
echo '{"name":"test","purpose":"A test skill for validation purposes","use_cases":["testing CDR"],"commands":[{"cmd":"echo hi","context":"greeting"}],"tips":["be careful"],"patterns":[]}' > "$TMPDIR/valid-intent.json"
if "$PY" "$REPO_ROOT/tools/lib/cdr-validate.py" "$TMPDIR/valid-intent.json" > /dev/null 2>&1; then
  pass "Validator accepts valid intent"
else
  fail "Validator rejected valid intent"
fi

# ── Test 6: Intent validation rejects missing fields ──
echo '{"name":"test"}' > "$TMPDIR/bad-intent.json"
if "$PY" "$REPO_ROOT/tools/lib/cdr-validate.py" "$TMPDIR/bad-intent.json" > /dev/null 2>&1; then
  fail "Validator should reject intent with missing fields"
else
  pass "Validator rejects incomplete intent"
fi

# ── Test 7: Reconstruction produces valid SKILL.md ──
"$PY" "$REPO_ROOT/tools/lib/cdr-reconstruct.py" "$TMPDIR/valid-intent.json" "$TMPDIR/recon.md" > /dev/null 2>&1
if [[ -f "$TMPDIR/recon.md" ]] && head -1 "$TMPDIR/recon.md" | grep -q '^---$'; then
  pass "Reconstruction produces SKILL.md with frontmatter"
else
  fail "Reconstruction did not produce valid SKILL.md"
fi

# ── Test 8: Full CDR pipeline on clean skill (requires Ollama) ──
if [[ "$OLLAMA_UP" == true ]]; then
  CDR_OUTPUT=$(bash "$REPO_ROOT/tools/skill-cdr.sh" "$FIXTURES/clean-skill.md" 2>&1) || true
  if echo "$CDR_OUTPUT" | grep -q "CDR complete"; then
    pass "Full CDR pipeline succeeds on clean skill"
    # Clean up delivered skill
    rm -rf "$REPO_ROOT/skills/test-clean"
  else
    fail "Full CDR pipeline failed on clean skill"
  fi
else
  skip "Full CDR pipeline (Ollama not running)"
fi

# ── Test 9: Quarantine cleaned up after CDR ──
if find "$REPO_ROOT/quarantine" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | grep -q .; then
  fail "Quarantine directory not cleaned up"
else
  pass "Quarantine cleaned up after CDR"
fi

# ── Summary ──
echo ""
echo "CDR Test Results: $PASS passed, $FAIL failed, $SKIP skipped"
if (( FAIL > 0 )); then
  echo ""
  echo "Failures:"
  for e in "${ERRORS[@]}"; do
    echo "  - $e"
  done
  exit 1
fi
