#!/usr/bin/env bash
# Tool tests for workbench-verify.sh

VERIFIER="$REPO_ROOT/tools/workbench-verify.sh"

test_syntax_valid() {
  assert_command_succeeds bash -n "$VERIFIER"
}

test_produces_verification_output() {
  assert_output_contains "(PASS|FAIL|WARN|SKIP)" bash "$VERIFIER"
}
