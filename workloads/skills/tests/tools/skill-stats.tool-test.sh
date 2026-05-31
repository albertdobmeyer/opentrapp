#!/usr/bin/env bash
# Tool tests for skill-stats.sh

STATS="$REPO_ROOT/tools/skill-stats.sh"

test_syntax_valid() {
  assert_command_succeeds bash -n "$STATS"
}

test_unknown_flag_exits_one() {
  assert_command_fails bash "$STATS" "--completely-invalid-flag-xyz"
}
