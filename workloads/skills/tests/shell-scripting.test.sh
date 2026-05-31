#!/usr/bin/env bash
# Tests for shell-scripting skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_error_handling() {
  assert_contains "$SKILL" "set -euo pipefail"
  assert_contains "$SKILL" "trap"
}

test_covers_argument_parsing() {
  assert_contains "$SKILL" "(getopts|getopt|\$1|\$@)"
}

test_covers_cleanup_patterns() {
  assert_contains "$SKILL" "(cleanup|trap.*EXIT)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^shell-scripting$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
