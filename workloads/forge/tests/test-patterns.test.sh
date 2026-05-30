#!/usr/bin/env bash
# Tests for test-patterns skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_test_frameworks() {
  assert_contains "$SKILL" "(Jest|Vitest|pytest)"
}

test_covers_mocking() {
  assert_contains "$SKILL" "(mock|Mock|stub|spy)"
}

test_covers_coverage() {
  assert_contains "$SKILL" "(coverage|--cov)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^test-patterns$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
