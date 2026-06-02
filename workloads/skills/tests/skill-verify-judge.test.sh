#!/usr/bin/env bash
# skill-verify --judge: the rung-2 second opinion on the auto-allow (v0.6 Item D1).
#
# The regression test is Ollama-free (a semantically-malicious skill that passes
# the STATIC scan must still be VERIFIED without --judge — that is the gap D1
# closes). The tighten / no-over-quarantine tests need a local judge (qwen2.5-coder:3b).
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILLS_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERIFY="$SKILLS_ROOT/tools/skill-verify.sh"
FIX="$SCRIPT_DIR/judge-fixtures"

PASS=0; FAIL=0; SKIP=0
pass() { echo -e "  \033[0;32mPASS\033[0m  $1"; PASS=$((PASS+1)); }
fail() { echo -e "  \033[0;31mFAIL\033[0m  $1"; FAIL=$((FAIL+1)); }
skip() { echo -e "  \033[0;33mSKIP\033[0m  $1"; SKIP=$((SKIP+1)); }

verdict() { # <args...> -> prints the JSON verdict field
  # Capture stdout independent of exit code — skill-verify exits 1 on QUARANTINED,
  # which must not be conflated with a parse failure.
  local out
  out=$(bash "$VERIFY" --json "$@" 2>/dev/null) || true
  printf '%s' "$out" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("verdict","?"))' 2>/dev/null || echo "?"
}
judge_available() {
  command -v ollama >/dev/null 2>&1 || return 1
  ollama list 2>/dev/null | grep -q "qwen2.5-coder:3b" || return 1
}

echo ""
echo "=== skill-verify --judge tests (Item D1) ==="
echo ""

# 1. Regression (no Ollama): a semantically-malicious skill that carries no code
#    patterns passes the STATIC scan → VERIFIED. This is the gap D1 closes.
if [[ "$(verdict "$FIX/note-summarizer")" == "VERIFIED" ]]; then
  pass "static scan alone VERIFIES the semantically-malicious skill (the gap D1 closes)"
else
  fail "expected the malicious-prose skill to pass the static scan (VERIFIED) without --judge"
fi

# 2. Default behaviour unchanged: a genuinely clean skill is VERIFIED.
if [[ "$(verdict "$FIX/judge-clean")" == "VERIFIED" ]]; then
  pass "a genuinely clean skill is VERIFIED (no --judge)"
else
  fail "clean skill should be VERIFIED"
fi

if judge_available; then
  # 3. --judge TIGHTENS the malicious skill to QUARANTINED (catches what static missed).
  if [[ "$(verdict --judge "$FIX/note-summarizer")" == "QUARANTINED" ]]; then
    pass "--judge tightens the malicious skill VERIFIED -> QUARANTINED (second opinion)"
  else
    fail "--judge did not quarantine the semantically-malicious skill"
  fi

  # 4. --judge does NOT over-quarantine a clean skill.
  if [[ "$(verdict --judge "$FIX/judge-clean")" == "VERIFIED" ]]; then
    pass "--judge keeps a clean skill VERIFIED (no over-quarantine)"
  else
    fail "--judge wrongly quarantined a clean skill"
  fi
else
  skip "qwen2.5-coder:3b not available — skipping the live judge tighten/clean tests"
fi

echo ""
echo "skill-verify-judge: ${PASS} passed, ${FAIL} failed, ${SKIP} skipped"
[[ $FAIL -eq 0 ]]
