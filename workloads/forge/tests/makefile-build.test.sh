#!/usr/bin/env bash
# Tests for makefile-build skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
}

test_covers_phony_targets() {
  assert_contains "$SKILL" "\.PHONY"
}

test_covers_target_recipe_structure() {
  assert_contains "$SKILL" "(target|recipe|prerequisit)"
}

test_covers_alternatives() {
  assert_contains "$SKILL" "(Just|Task)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^makefile-build$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
