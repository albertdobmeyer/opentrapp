#!/usr/bin/env bash
# Tests for container-debug skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_docker_commands() {
  assert_contains "$SKILL" "docker logs"
  assert_contains "$SKILL" "docker exec"
  assert_contains "$SKILL" "docker inspect"
}

test_covers_compose() {
  assert_contains "$SKILL" "(docker.compose|docker-compose)"
}

test_covers_health_checks() {
  assert_contains "$SKILL" "(HEALTHCHECK|health.check)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^container-debug$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 10
}
