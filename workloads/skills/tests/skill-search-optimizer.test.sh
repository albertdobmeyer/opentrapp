#!/usr/bin/env bash
# Tests for skill-search-optimizer skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_search_concepts() {
  assert_contains "$SKILL" "(semantic|vector|embedding)"
}

test_covers_discoverability() {
  assert_contains "$SKILL" "(discoverab|visibil|ranking)"
}

test_covers_description_optimization() {
  assert_contains "$SKILL" "(description|Description)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^skill-search-optimizer$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
