#!/usr/bin/env bash
# Tests for {{SKILL_NAME}} skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX|PLACEHOLDER)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^{{SKILL_NAME}}$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}

# TODO: Add domain-specific tests for {{SKILL_NAME}}
# Examples:
#   test_covers_feature() {
#     assert_contains "$SKILL" "pattern-to-match"
#   }
#   test_has_enough_content() {
#     assert_line_count "$SKILL" 150 700
#   }
