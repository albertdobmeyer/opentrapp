#!/usr/bin/env bash
# Tests for feed-scanner.sh

SCANNER="$REPO_ROOT/tools/feed-scanner.sh"
FIXTURES="$REPO_ROOT/tests/fixtures"

test_help_exits_0() {
  assert_exit_code 0 bash "$SCANNER" --help
}

test_help_shows_usage() {
  assert_output_contains "Usage" bash "$SCANNER" --help
}

test_bogus_flag_exits_1() {
  assert_exit_code 1 bash "$SCANNER" --bogus
}

test_nonexistent_file_exits_1() {
  assert_command_fails bash "$SCANNER" --file /nonexistent/path.json
}

test_clean_posts_exits_0() {
  assert_exit_code 0 bash "$SCANNER" --file "$FIXTURES/clean-posts.json"
}

test_malicious_posts_exits_1() {
  assert_exit_code 1 bash "$SCANNER" --file "$FIXTURES/malicious-posts.json"
}

test_malicious_posts_shows_critical() {
  assert_output_contains "CRITICAL" bash "$SCANNER" --file "$FIXTURES/malicious-posts.json"
}

test_empty_posts_exits_0() {
  assert_exit_code 0 bash "$SCANNER" --file "$FIXTURES/empty-posts.json"
}

test_safe_research_exits_0() {
  assert_exit_code 0 bash "$SCANNER" --file "$FIXTURES/safe-research-posts.json"
}

test_verbose_shows_match() {
  assert_output_contains "Ignore" bash "$SCANNER" --verbose --file "$FIXTURES/malicious-posts.json"
}
