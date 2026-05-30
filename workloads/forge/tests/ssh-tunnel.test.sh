#!/usr/bin/env bash
# Tests for ssh-tunnel skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_port_forwarding() {
  assert_contains "$SKILL" "ssh -L"
  assert_contains "$SKILL" "ssh -R"
  assert_contains "$SKILL" "ssh -D"
}

test_covers_jump_hosts() {
  assert_contains "$SKILL" "(ProxyJump|jump.host|bastion)"
}

test_covers_key_management() {
  assert_contains "$SKILL" "(ssh-keygen|ssh-copy-id|ssh-add)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^ssh-tunnel$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
