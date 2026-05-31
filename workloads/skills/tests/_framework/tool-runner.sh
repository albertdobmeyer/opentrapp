#!/usr/bin/env bash
# Tool test runner — discovers and executes tool test files
# Usage: tool-runner.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$TESTS_DIR/.." && pwd)"

source "$SCRIPT_DIR/tool-assertions.sh"

# Colors
if [[ -t 1 ]]; then
  RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
  CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'
else
  RED='' GREEN='' YELLOW='' CYAN='' BOLD='' RESET=''
fi

TOTAL_TESTS=0
TOTAL_PASSED=0
TOTAL_FAILED=0
FAILED_TESTS=()

# Export REPO_ROOT for test files
export REPO_ROOT

# Discover tool test files
test_files=()
for f in "$TESTS_DIR"/tools/*.tool-test.sh; do
  [[ -f "$f" ]] || continue
  test_files+=("$f")
done

if (( ${#test_files[@]} == 0 )); then
  echo "No tool test files found in tests/tools/"
  exit 1
fi

echo -e "\n${BOLD}Running tool tests${RESET}"
echo -e "Found ${#test_files[@]} test file(s)\n"

for test_file in "${test_files[@]}"; do
  test_name=$(basename "$test_file" .tool-test.sh)
  echo -e "${CYAN}--- ${test_name} ---${RESET}"

  # Reset assertion counters
  TOOL_ASSERT_PASS=0
  TOOL_ASSERT_FAIL=0

  # Source the test file (defines test_ functions)
  source "$test_file"

  # Discover test_ functions
  test_funcs=()
  while IFS= read -r func; do
    [[ "$func" =~ ^test_ ]] && test_funcs+=("$func")
  done < <(declare -F | awk '{print $3}')

  for func in "${test_funcs[@]}"; do
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    if "$func" 2>/dev/null; then
      echo -e "  ${GREEN}PASS${RESET}  $func"
      TOTAL_PASSED=$((TOTAL_PASSED + 1))
    else
      echo -e "  ${RED}FAIL${RESET}  $func"
      TOTAL_FAILED=$((TOTAL_FAILED + 1))
      FAILED_TESTS+=("${test_name}::${func}")
    fi
  done

  # Unset test functions to prevent contamination
  for func in "${test_funcs[@]}"; do
    unset -f "$func"
  done
done

# Summary
echo ""
echo -e "${BOLD}Tool Test Results:${RESET}"
echo -e "  ${GREEN}Passed: ${TOTAL_PASSED}${RESET}"
echo -e "  ${RED}Failed: ${TOTAL_FAILED}${RESET}"
echo -e "  Total:  ${TOTAL_TESTS}"

if (( TOTAL_FAILED > 0 )); then
  echo ""
  echo -e "${RED}Failed tests:${RESET}"
  for t in "${FAILED_TESTS[@]}"; do
    echo "  - $t"
  done
  exit 1
fi

echo ""
