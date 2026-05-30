#!/usr/bin/env bash
# Tests for skill-reviewer skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(PLACEHOLDER)"
}

test_covers_scoring() {
  assert_contains "$SKILL" "(rubric|scor|Scor)"
}

test_covers_skill_format() {
  assert_contains "$SKILL" "SKILL\.md"
}

test_covers_defect_categories() {
  assert_contains "$SKILL" "(defect|quality|Critical|Major|Minor)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^skill-reviewer$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
