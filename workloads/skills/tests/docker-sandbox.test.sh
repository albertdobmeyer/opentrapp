#!/usr/bin/env bash
# Tests for docker-sandbox skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
}

test_references_docker_commands() {
  assert_contains "$SKILL" "docker\s+(run|build|exec|sandbox|ps|logs)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^docker-sandbox$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 5
}
