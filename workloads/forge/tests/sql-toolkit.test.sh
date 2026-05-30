#!/usr/bin/env bash
# Tests for sql-toolkit skill

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
}

test_references_sql_tools() {
  assert_contains "$SKILL" "(sqlite3|psql|mysql)"
}

test_covers_multiple_databases() {
  assert_contains "$SKILL" "SQLite"
  assert_contains "$SKILL" "PostgreSQL"
  assert_contains "$SKILL" "MySQL"
}

test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^sql-toolkit$"
}

test_has_code_blocks() {
  assert_min_code_blocks "$SKILL" 8
}
