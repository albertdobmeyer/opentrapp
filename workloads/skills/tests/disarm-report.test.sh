#!/usr/bin/env bash
# Disarm-report tests. Pure file scan (no Ollama) — always runs. Verifies the
# GUI's read-only trust-artifact channel: cleaned skills + their plain-language
# disarm text, as JSON the frontend renders.
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOL="$MODULE_ROOT/tools/disarm-report.sh"
PASS=0; FAIL=0
pass() { echo -e "  \033[0;32mPASS\033[0m  $1"; PASS=$((PASS+1)); }
fail() { echo -e "  \033[0;31mFAIL\033[0m  $1"; FAIL=$((FAIL+1)); }

echo ""
echo "=== Disarm-report tests ==="

# 1. lists a cleaned skill + its plain-language report text.
tmp="$(mktemp -d)"; mkdir -p "$tmp/csv-helper"
printf 'What I did (disarm summary):\n  Removed: reads your saved passwords and sends them out\n  Kept: formats CSV files\n' \
  > "$tmp/csv-helper/DISARM-DIFF.txt"
out=$(bash "$TOOL" --root "$tmp")
if echo "$out" | jq -e '.count==1 and .cleaned[0].skill=="csv-helper" and (.cleaned[0].report|test("passwords"))' >/dev/null 2>&1; then
  pass "lists a cleaned skill with its disarm report text"
else
  fail "did not list the cleaned skill: $out"
fi

# 2. multiple cleaned skills are all listed.
mkdir -p "$tmp/net-tool"; printf 'Kept: pings a host\n' > "$tmp/net-tool/DISARM-DIFF.txt"
out=$(bash "$TOOL" --root "$tmp")
if echo "$out" | jq -e '.count==2' >/dev/null 2>&1; then
  pass "lists multiple cleaned skills"
else
  fail "expected count 2: $out"
fi

# 3. an empty/absent deliveries root yields an explicit empty result, not an error.
out=$(bash "$TOOL" --root "$(mktemp -d)")
if echo "$out" | jq -e '.count==0 and (.cleaned|length)==0' >/dev/null 2>&1; then
  pass "empty deliveries -> count 0 (clean empty state, no error)"
else
  fail "empty root not handled: $out"
fi

# 4. output is always valid JSON (the frontend parses it).
if bash "$TOOL" --root "$tmp" | jq -e . >/dev/null 2>&1; then
  pass "output is valid JSON"
else
  fail "output is not valid JSON"
fi

rm -rf "$tmp"
echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]]
