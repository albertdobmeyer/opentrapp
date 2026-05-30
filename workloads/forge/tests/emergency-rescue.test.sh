#!/usr/bin/env bash
# Tests for emergency-rescue skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  # Match standalone TODO/FIXME/XXX but not XXX inside masked keys like AKIAXXXXXXXX
  assert_not_contains "$SKILL" "(^|[^X])(TODO|FIXME)([^X]|$)"
}

test_covers_git_disasters() {
  assert_contains "$SKILL" "force-push"
  assert_contains "$SKILL" "reflog"
}

test_covers_credential_leaks() {
  assert_contains "$SKILL" "(credential|secret|leak)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^emergency-rescue$"
}

test_has_recovery_patterns() {
  # Rescue kit should follow diagnose → fix → verify pattern
  assert_contains "$SKILL" "(DIAGNOSE|FIX|VERIFY|diagnose|fix|verify)"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
