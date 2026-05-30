#!/usr/bin/env bash
# Tests for identity-checklist.sh

CHECKLIST="$REPO_ROOT/tools/identity-checklist.sh"

test_runs_without_crash() {
  # Should exit 0 (all pass) or 1 (some fail), never 2 (bash error)
  local rc=0
  bash "$CHECKLIST" >/dev/null 2>&1 || rc=$?
  if (( rc == 0 || rc == 1 )); then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: expected exit 0 or 1, got $rc"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

test_output_contains_configuration() {
  assert_output_contains "Configuration" bash "$CHECKLIST"
}

test_output_contains_preflight() {
  assert_output_contains "Pre-Flight" bash "$CHECKLIST"
}
