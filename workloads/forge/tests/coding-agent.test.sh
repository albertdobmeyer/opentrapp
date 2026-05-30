#!/usr/bin/env bash
# Tests for coding-agent skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "The Pattern"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_background_workdir_pattern() {
  assert_contains "$SKILL" "(background|workdir)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^coding-agent$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 3
}
