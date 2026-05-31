#!/usr/bin/env bash
# Tests for git-workflows skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_rebase_and_bisect() {
  assert_contains "$SKILL" "rebase"
  assert_contains "$SKILL" "bisect"
}

test_covers_worktree() {
  assert_contains "$SKILL" "worktree"
}

test_covers_recovery() {
  assert_contains "$SKILL" "reflog"
  assert_contains "$SKILL" "cherry-pick"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^git-workflows$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
