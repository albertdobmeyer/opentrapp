#!/usr/bin/env bash
# Tests for csv-pipeline skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
}

test_references_tools() {
  assert_contains "$SKILL" "(awk|python3?|cut|sort)"
}

test_covers_csv_operations() {
  assert_contains "$SKILL" "(filter|join|aggregate|deduplic)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^csv-pipeline$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
