#!/usr/bin/env bash
# Tool tests for registry-explore.sh

EXPLORER="$REPO_ROOT/tools/registry-explore.sh"

test_syntax_valid() {
  assert_command_succeeds bash -n "$EXPLORER"
}

test_unknown_flag_exits_one() {
  assert_command_fails bash "$EXPLORER" "--completely-invalid-flag-xyz"
}
