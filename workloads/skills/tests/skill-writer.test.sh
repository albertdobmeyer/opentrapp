#!/usr/bin/env bash
# Tests for skill-writer skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  # Allow mentioning TODO/FIXME in backtick-quoted checklist instructions
  assert_not_contains "$SKILL" "^(TODO|FIXME|XXX):"
}

test_documents_skill_format() {
  assert_contains "$SKILL" "SKILL\.md"
  assert_contains "$SKILL" "frontmatter"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^skill-writer$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 5
}
