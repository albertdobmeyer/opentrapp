#!/usr/bin/env bash
# Tests for infra-as-code skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_terraform() {
  assert_contains "$SKILL" "terraform"
  assert_contains "$SKILL" "(plan|apply|state)"
}

test_covers_alternatives() {
  assert_contains "$SKILL" "(CloudFormation|Pulumi)"
}

test_covers_state_management() {
  assert_contains "$SKILL" "(state|backend)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^infra-as-code$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
