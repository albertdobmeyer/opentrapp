#!/usr/bin/env bash
# Tests for cicd-pipeline skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_github_actions() {
  assert_contains "$SKILL" "GitHub Actions"
  assert_contains "$SKILL" "(actions/checkout|runs-on)"
}

test_covers_matrix_builds() {
  assert_contains "$SKILL" "matrix"
}

test_covers_caching() {
  assert_contains "$SKILL" "(cache|cach)"
}

test_covers_secrets() {
  assert_contains "$SKILL" "(secret|SECRET)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^cicd-pipeline$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 15
}
