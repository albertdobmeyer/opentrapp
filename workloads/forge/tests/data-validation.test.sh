#!/usr/bin/env bash
# Tests for data-validation skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_json_schema() {
  assert_contains "$SKILL" "JSON Schema"
}

test_covers_validation_libraries() {
  assert_contains "$SKILL" "(Zod|Pydantic)"
}

test_covers_validation_concepts() {
  assert_contains "$SKILL" "(validat|schema)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^data-validation$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
