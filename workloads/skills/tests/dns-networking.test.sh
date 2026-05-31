#!/usr/bin/env bash
# Tests for dns-networking skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(FIXME|PLACEHOLDER)"
}

test_covers_dns_tools() {
  assert_contains "$SKILL" "(dig|nslookup)"
}

test_covers_ports_and_firewall() {
  assert_contains "$SKILL" "(port|firewall|iptables|ufw)"
}

test_covers_ssl_tls() {
  assert_contains "$SKILL" "(SSL|TLS|certificate)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^dns-networking$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
