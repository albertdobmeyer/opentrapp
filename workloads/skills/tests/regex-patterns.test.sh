#!/usr/bin/env bash
# Tests for regex-patterns skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "PLACEHOLDER"
}

test_covers_lookaround() {
  assert_contains "$SKILL" "(lookahead|lookbehind)"
}

test_covers_capture_groups() {
  assert_contains "$SKILL" "(capture.group|named.group|\(\?<)"
}

test_covers_quantifiers() {
  assert_contains "$SKILL" "(quantifier|greedy|lazy)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^regex-patterns$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
