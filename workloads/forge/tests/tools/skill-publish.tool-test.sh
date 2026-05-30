#!/usr/bin/env bash
# Tool tests for skill-publish.sh

PUBLISHER="$REPO_ROOT/tools/skill-publish.sh"

test_no_args_exits_one() {
  assert_exit_code 1 bash "$PUBLISHER"
}

test_bad_version_exits_one() {
  assert_exit_code 1 bash "$PUBLISHER" "docker-sandbox" "not-a-version"
}

test_missing_skill_exits_one() {
  assert_exit_code 1 bash "$PUBLISHER" "nonexistent-skill-xyz" "1.0.0"
}
