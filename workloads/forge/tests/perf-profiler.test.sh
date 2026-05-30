#!/usr/bin/env bash
# Tests for perf-profiler skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_flame_graphs() {
  assert_contains "$SKILL" "(flamegraph|flame.graph|Flame)"
}

test_covers_profiling() {
  assert_contains "$SKILL" "(profil|benchmark)"
}

test_covers_resource_types() {
  assert_contains "$SKILL" "(CPU|memory|heap)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^perf-profiler$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
