#!/usr/bin/env bash
# Tool tests for pipeline-report.sh

REPORTER="$REPO_ROOT/tools/pipeline-report.sh"

test_syntax_valid() {
  assert_command_succeeds bash -n "$REPORTER"
}

test_contains_quality_gates() {
  assert_output_contains "Quality Gates" bash "$REPORTER"
}
