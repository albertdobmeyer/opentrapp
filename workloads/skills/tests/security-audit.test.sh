#!/usr/bin/env bash
# Tests for security-audit skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_owasp() {
  assert_contains "$SKILL" "OWASP"
}

test_covers_vulnerability_scanning() {
  assert_contains "$SKILL" "(npm audit|pip.audit|cargo audit|trivy)"
}

test_covers_secret_detection() {
  assert_contains "$SKILL" "(secret|credential|API.key)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^security-audit$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
