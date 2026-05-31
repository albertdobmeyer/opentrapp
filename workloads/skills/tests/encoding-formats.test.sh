#!/usr/bin/env bash
# Tests for encoding-formats skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_base64() {
  assert_contains "$SKILL" "base64"
}

test_covers_hex() {
  assert_contains "$SKILL" "(xxd|hex)"
}

test_covers_jwt() {
  assert_contains "$SKILL" "JWT"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^encoding-formats$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
