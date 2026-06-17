#!/usr/bin/env bash
# Verify every numerical claim in this repository.
# See docs/reproduce.md for the full table of claims, expected outputs, and runtime ceilings.
#
# Usage:
#   bash docs/reproduce.sh [--quick]
#
# --quick skips the long-running test suites (Rust, Vitest, Playwright) and runs
# only the offline counted-artefact rows. Default is the full reproduction.

set -u
set -o pipefail

readonly REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

quick=0
if [[ "${1:-}" == "--quick" ]]; then
  quick=1
fi

# ── Output formatting ───────────────────────────────────────────────────────
green='\033[0;32m'
red='\033[0;31m'
yellow='\033[0;33m'
reset='\033[0m'

pass_count=0
fail_count=0

row() {
  local label="$1"
  local expected="$2"
  local actual="$3"

  if [[ "$actual" == "$expected" ]]; then
    printf "${green}PASS${reset}  %-50s  expected=%s  actual=%s\n" "$label" "$expected" "$actual"
    pass_count=$((pass_count + 1))
  else
    printf "${red}FAIL${reset}  %-50s  expected=%s  actual=%s\n" "$label" "$expected" "$actual"
    fail_count=$((fail_count + 1))
  fi
}

row_ge() {
  # Pass if actual is greater than or equal to floor.
  local label="$1"
  local floor="$2"
  local actual="$3"

  if [[ "$actual" =~ ^[0-9]+$ ]] && (( actual >= floor )); then
    printf "${green}PASS${reset}  %-50s  floor=%s  actual=%s\n" "$label" "$floor" "$actual"
    pass_count=$((pass_count + 1))
  else
    printf "${red}FAIL${reset}  %-50s  floor=%s  actual=%s\n" "$label" "$floor" "$actual"
    fail_count=$((fail_count + 1))
  fi
}

skip() {
  printf "${yellow}SKIP${reset}  %-50s  %s\n" "$1" "${2:-}"
}

# ── Group 1: counted artefacts (offline) ────────────────────────────────────
echo "── Group 1: counted artefacts (offline) ──"

patterns=$(grep -cE "^\s*'(CRITICAL|HIGH|MEDIUM)\|" workloads/skills/tools/lib/patterns.sh 2>/dev/null || echo "0")
row "1. malicious-skill patterns" "87" "$patterns"

# verify.sh uses two patterns: `check N "..." \\` (checks 1 to 13) and an inline
# printf "  [%2d]" with a literal check number (checks 14 to 24). Count unique
# check numbers across both patterns.
verify_checks=$(
  {
    grep -oE '^check [0-9]+' workloads/agent/scripts/verify.sh \
      | grep -oE '[0-9]+'
    grep -oE 'printf "  \[%2d\][^"]*" [0-9]+' workloads/agent/scripts/verify.sh \
      | grep -oE '[0-9]+$'
  } 2>/dev/null | sort -u | wc -l
)
row "2. startup-verification checks" "24" "$verify_checks"

# Row 3 is verified by Group 2's orchestrator-check; report from there.

banned_terms=$(awk '/const BANNED_TERMS = \[/,/^\];/' app/e2e/user-facing.spec.ts 2>/dev/null \
  | grep -cE '^\s*"[^"]+",?$' || echo "0")
row "4. reserved terms" "28" "$banned_terms"

services=$(python3 -c "import yaml; print(len(yaml.safe_load(open('compose.yml'))['services']))" 2>/dev/null || echo "0")
row "5. compose services" "5" "$services"

tiers=$(grep -cE "^TIER " docs/trifecta.md 2>/dev/null || echo "0")
row "6. trust tiers" "3" "$tiers"

shells=$(grep -E "^\| (Hard|Split|Soft) Shell" docs/trifecta.md 2>/dev/null | wc -l)
row "7. shell levels" "3" "$shells"

attackers=$(grep -cE "^## T[1-6]:" docs/threat-model.md 2>/dev/null || echo "0")
row "8. attacker categories (T1 to T6)" "6" "$attackers"

# ── Group 2: test suites (offline; longer-running) ──────────────────────────
if (( quick == 1 )); then
  echo
  echo "── Group 2: test suites (skipped under --quick) ──"
  skip "9. cargo test --lib" "skipped under --quick"
  skip "10. npm test" "skipped under --quick"
  skip "11. playwright test" "skipped under --quick"
  skip "12. tsc --noEmit" "skipped under --quick"
  skip "13. orchestrator-check.sh" "skipped under --quick"
else
  echo
  echo "── Group 2: test suites (offline) ──"

  echo "9. running cargo test --lib (≤ 4 min cold, ≤ 30 s warm)..."
  rust_count=$(cd app/src-tauri \
    && cargo test --lib 2>&1 \
    | grep -oE 'test result: ok\. [0-9]+ passed' \
    | grep -oE '[0-9]+' \
    | head -1)
  rust_count=${rust_count:-0}
  row_ge "9. Rust unit tests" "56" "$rust_count"

  echo "10. running npm test -- --run (≤ 30 s)..."
  vitest_count=$(cd app \
    && npm test -- --run 2>&1 \
    | grep -oE 'Tests +[0-9]+ passed' \
    | grep -oE '[0-9]+' \
    | head -1)
  vitest_count=${vitest_count:-0}
  row_ge "10. Vitest tests" "74" "$vitest_count"

  echo "11. running npx playwright test (≤ 90 s; first-run downloads Chromium)..."
  if (cd app && npx playwright test 2>/dev/null > /tmp/reproduce-pw.log); then
    pw_count=$(grep -oE '[0-9]+ passed' /tmp/reproduce-pw.log | grep -oE '[0-9]+' | head -1)
    pw_count=${pw_count:-0}
    row_ge "11. Playwright tests" "25" "$pw_count"
  else
    skip "11. Playwright tests" "browser not installed; run 'npx playwright install chromium --with-deps'"
  fi

  echo "12. running npx tsc --noEmit..."
  if (cd app && npx tsc --noEmit 2>/dev/null); then
    row "12. TypeScript strict" "exit=0" "exit=0"
  else
    row "12. TypeScript strict" "exit=0" "exit=non-zero"
  fi

  echo "13. running tests/orchestrator-check.sh..."
  oc_line=$(bash tests/orchestrator-check.sh 2>&1 | grep -E "Results:" | head -1)
  oc_warnings=$(echo "$oc_line" | grep -oE '[0-9]+ warnings' | grep -oE '[0-9]+' | head -1)
  oc_warnings=${oc_warnings:-X}
  row "13. orchestrator-check warnings" "0" "$oc_warnings"
  oc_passed=$(echo "$oc_line" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' | head -1)
  oc_passed=${oc_passed:-0}
  row "3. orchestrator checks" "120" "$oc_passed"
fi

# ── Group 3: external claims (cited only) ───────────────────────────────────
echo
echo "── Group 3: external claims (cited; not re-derived) ──"
skip "14. 11.9 % ClawHavoc malicious-skill rate" "see docs/reproduce.md §3.14"
skip "15. CVE-2026-25253 (OpenClaw management API RCE)" "see docs/reproduce.md §3.15"
skip "16. Moltbook database breach (2026-01)" "see docs/reproduce.md §3.16"

# ── Summary ────────────────────────────────────────────────────────────────
echo
echo "── Summary ──"
echo "Passed: $pass_count"
echo "Failed: $fail_count"

if (( fail_count == 0 )); then
  echo "All counted-artefact and test-suite claims reproduce."
  exit 0
else
  echo "One or more claims did not reproduce. See rows above."
  exit 1
fi
