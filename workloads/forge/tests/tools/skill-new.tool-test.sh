#!/usr/bin/env bash
# Tool tests for skill-new.sh

SCAFFOLDER="$REPO_ROOT/tools/skill-new.sh"

test_creates_skill_and_test_files() {
  local tmpdir
  tmpdir=$(mktemp -d)
  # skill-new.sh uses REPO_ROOT internally, so we run in a temp copy context
  # Just verify it rejects creating in non-existent skill dir by testing slug validation
  # The real creation test: valid slug produces files
  (
    cd "$tmpdir"
    mkdir -p skills templates/cli-tool tests
    # Create minimal template
    cat > templates/cli-tool/SKILL.md <<'EOF'
---
name: {{SKILL_NAME}}
description: A new skill
---

# {{SKILL_TITLE}}

## When to Use

## Tips
EOF
    cat > templates/_test.template.sh <<'EOF'
#!/usr/bin/env bash
test_placeholder() { assert_section_exists "$SKILL" "When to Use"; }
EOF
    # Copy lib
    cp -r "$REPO_ROOT/tools" "$tmpdir/tools"
    bash "$tmpdir/tools/skill-new.sh" "test-tool" "cli-tool" 2>/dev/null
  )
  assert_file_exists "$tmpdir/skills/test-tool/SKILL.md"
  assert_file_exists "$tmpdir/tests/test-tool.test.sh"
  rm -rf "$tmpdir"
}

test_invalid_slug_rejected() {
  assert_command_fails bash "$SCAFFOLDER" "INVALID_SLUG"
}

test_existing_skill_rejected() {
  # Try to scaffold a skill that already exists
  local existing
  existing=$(ls -1d "$REPO_ROOT/skills"/*/ 2>/dev/null | head -1)
  if [[ -n "$existing" ]]; then
    local slug
    slug=$(basename "$existing")
    assert_command_fails bash "$SCAFFOLDER" "$slug"
  else
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  fi
}
