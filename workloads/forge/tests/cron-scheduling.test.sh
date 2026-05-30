#!/usr/bin/env bash
# Tests for cron-scheduling skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
}

test_covers_crontab() {
  assert_contains "$SKILL" "crontab"
}

test_covers_systemd_timers() {
  assert_contains "$SKILL" "systemd"
  assert_contains "$SKILL" "(timer|OnCalendar)"
}

test_covers_cron_syntax() {
  assert_contains "$SKILL" "(minute|hour|day)"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^cron-scheduling$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
