#!/usr/bin/env bash
# Tests for api-dev skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_http_tools() {
  assert_contains "$SKILL" "curl"
  assert_contains "$SKILL" "(jq|JSON|json)"
}

test_covers_rest_and_graphql() {
  assert_contains "$SKILL" "(REST|GET|POST|PUT|DELETE)"
}

test_covers_openapi() {
  assert_contains "$SKILL" "(OpenAPI|Swagger|openapi)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^api-dev$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
