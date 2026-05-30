#!/usr/bin/env bash
# Tests for log-analyzer skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
}

test_covers_search_tools() {
  assert_contains "$SKILL" "grep"
  assert_contains "$SKILL" "(awk|jq)"
}

test_covers_stack_traces() {
  assert_contains "$SKILL" "(stack trace|traceback|Traceback)"
}

test_covers_structured_logging() {
  assert_contains "$SKILL" "(structured|pino|structlog|zerolog)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^log-analyzer$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
