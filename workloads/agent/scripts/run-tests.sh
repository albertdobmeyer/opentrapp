#!/usr/bin/env bash
# OpenCli-Container: Test Runner
#
# Runs all test scripts in tests/ sequentially and reports results.
# Requires the opencli-container container to be running.
#
# Usage: bash scripts/run-tests.sh
#        make test

set -uo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME="podman"
command -v podman &>/dev/null || RUNTIME="docker"
export RUNTIME

BOLD='\033[1m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo ""
echo -e "${BOLD}OpenCli-Container: Test Suite${NC}"
echo "========================="
echo ""

# Check container is running
if ! $RUNTIME inspect opencli-container --format '{{.State.Status}}' 2>/dev/null | grep -q "running"; then
    echo -e "${RED}ERROR: opencli-container container is not running.${NC}"
    echo "  Start it first: make start"
    exit 1
fi

# Find all test scripts, excluding destructive tests by default.
# test-kill-switch.sh destroys containers — run it manually or with --include-destructive.
INCLUDE_DESTRUCTIVE=false
if [ "${1:-}" = "--include-destructive" ]; then
    INCLUDE_DESTRUCTIVE=true
fi

test_files=$(find "$VAULT_DIR/tests" -name "test-*.sh" -type f | sort)
if ! $INCLUDE_DESTRUCTIVE; then
    test_files=$(echo "$test_files" | grep -v "test-kill-switch")
    echo "(Skipping destructive tests. Use --include-destructive to include.)"
    echo ""
fi
total=$(echo "$test_files" | wc -l)
passed=0
failed=0
failed_names=""

echo "Found $total test scripts."
echo ""

for test_file in $test_files; do
    test_name=$(basename "$test_file" .sh)
    printf "  %-45s " "$test_name"

    # Run test, capture output and exit code
    output=$(bash "$test_file" 2>&1) && exit_code=0 || exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}PASS${NC}"
        passed=$((passed + 1))
    else
        echo -e "${RED}FAIL${NC}"
        failed=$((failed + 1))
        failed_names="$failed_names    $test_name\n"
        # Show first 5 lines of output for failed tests
        echo "$output" | head -5 | sed 's/^/       /'
        echo ""
    fi
done

echo ""
echo "========================="
echo -e "Results: ${GREEN}$passed passed${NC}, ${RED}$failed failed${NC} (of $total)"
echo ""

if [ $failed -gt 0 ]; then
    echo -e "${RED}Failed tests:${NC}"
    echo -e "$failed_names"
    exit 1
else
    echo -e "${GREEN}ALL TESTS PASSED.${NC}"
fi
