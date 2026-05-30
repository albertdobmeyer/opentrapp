#!/usr/bin/env bash
# Tests for agent-census.sh

CENSUS="$REPO_ROOT/tools/agent-census.sh"
FIXTURES="$REPO_ROOT/tests/fixtures"

test_help_exits_0() {
  assert_exit_code 0 bash "$CENSUS" --help
}

test_help_shows_usage() {
  assert_output_contains "Usage" bash "$CENSUS" --help
}

test_bogus_flag_exits_1() {
  assert_exit_code 1 bash "$CENSUS" --bogus
}

test_file_nonexistent_exits_1() {
  assert_command_fails bash "$CENSUS" --file /nonexistent/path.json
}

test_file_fixture_exits_0() {
  assert_exit_code 0 bash "$CENSUS" --file "$FIXTURES/census-snapshot.json"
}

test_file_fixture_shows_overview() {
  assert_output_contains "Platform Overview" bash "$CENSUS" --file "$FIXTURES/census-snapshot.json"
}
