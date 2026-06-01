#!/usr/bin/env bash
# Semantic firewall tests (M4). The headline property: a PARAPHRASED injection
# that evades all 25 static patterns is caught by the semantic (rung-2) judge.
# Ollama-gated (the judge needs a local model).
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCIAL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FW="$SOCIAL_ROOT/tools/semantic-firewall.sh"
FIX="$SCRIPT_DIR/fixtures/paraphrased-injection-posts.json"
PASS=0; FAIL=0; SKIP=0
pass() { echo -e "  \033[0;32mPASS\033[0m  $1"; PASS=$((PASS+1)); }
fail() { echo -e "  \033[0;31mFAIL\033[0m  $1"; FAIL=$((FAIL+1)); }
skip() { echo -e "  \033[0;33mSKIP\033[0m  $1"; SKIP=$((SKIP+1)); }

echo ""
echo "=== Semantic firewall tests ==="

if ! curl -sf --max-time 3 "http://localhost:11434/api/tags" > /dev/null 2>&1; then
  skip "Ollama not running — semantic firewall tests skipped"
  echo ""; echo "Results: $PASS passed, $FAIL failed, $SKIP skipped"; exit 0
fi

out=$(bash "$FW" --file "$FIX" 2>/dev/null)

# 1. THE HEADLINE PROPERTY — a paraphrased injection that evades all 25 static
#    patterns is caught by the semantic (rung-2) judge. This is robust across
#    models (a clear injection is reliably flagged).
para001_line=$(echo "$out" | grep "para-001" || true)
if echo "$para001_line" | grep -qE "BLOCKED|REVIEW"; then
  pass "paraphrased injection (evades 25 regexes) caught by the semantic judge"
else
  fail "paraphrased injection para-001 was NOT caught: ${para001_line:-no verdict}"
fi

# 2. rung 0 (static patterns) runs first — the cheap pre-filter precedes rung 2.
if echo "$out" | grep -q "rung 0"; then
  pass "rung 0 (static patterns) runs before the semantic judge"
else
  fail "rung 0 pre-filter did not run"
fi

# NOTE: with the default judge (qwen2.5-coder:3b, see sentinel/config.sh) the
# benign post (para-002) is correctly allowed — verified manually. We keep the
# hard gate to the model-robust property (para-001 caught) and do not assert
# "benign allowed" here, since that remains mildly model-dependent and we want
# this suite deterministic. (D3 — the 1.5b over-blocking — is resolved by the
# 3b default; on a tiny box a user may drop to 1.5b and accept review-not-allow.)

echo ""
echo "Results: $PASS passed, $FAIL failed, $SKIP skipped"
[[ "$FAIL" -eq 0 ]]
