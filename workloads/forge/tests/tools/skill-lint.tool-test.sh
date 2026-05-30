#!/usr/bin/env bash
# Tool tests for skill-lint.sh

LINTER="$REPO_ROOT/tools/skill-lint.sh"

test_valid_skill_passes() {
  # Use an existing known-good skill
  local skill_dir="$REPO_ROOT/skills/docker-sandbox"
  if [[ -f "$skill_dir/SKILL.md" ]]; then
    assert_command_succeeds bash "$LINTER" "$skill_dir"
  else
    # Skip if skill doesn't exist (CI without skills)
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  fi
}

test_empty_dir_fails() {
  local tmpdir
  tmpdir=$(mktemp -d)
  assert_command_fails bash "$LINTER" "$tmpdir"
  rm -rf "$tmpdir"
}

test_missing_frontmatter_fails() {
  local tmpdir
  tmpdir=$(mktemp -d)
  mkdir -p "$tmpdir/broken-skill"
  echo "# Just a heading, no frontmatter" > "$tmpdir/broken-skill/SKILL.md"
  assert_command_fails bash "$LINTER" "$tmpdir/broken-skill"
  rm -rf "$tmpdir"
}
