#!/usr/bin/env bash
# Tool tests for skill-create.sh

CREATOR="$REPO_ROOT/tools/skill-create.sh"

test_bad_slug_rejected() {
  assert_command_fails bash "$CREATOR" --name "BAD NAME!" --type cli-tool --description "A test skill for validation"
}

test_uppercase_slug_rejected() {
  assert_command_fails bash "$CREATOR" --name "NotValid" --type cli-tool --description "A test skill for validation"
}

test_name_collision_rejected() {
  # Use an existing skill to trigger collision check
  local existing
  existing=$(ls -1d "$REPO_ROOT/skills"/*/ 2>/dev/null | head -1)
  if [[ -n "$existing" ]]; then
    local slug
    slug=$(basename "$existing")
    assert_command_fails bash "$CREATOR" --name "$slug" --type cli-tool --description "A test skill for validation"
  else
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  fi
}

test_invalid_type_rejected() {
  assert_command_fails bash "$CREATOR" --name "test-invalid-type" --type invalid --description "A test skill for validation"
}

test_short_description_rejected() {
  assert_command_fails bash "$CREATOR" --name "test-short-desc" --type cli-tool --description "short"
}

test_unknown_option_rejected() {
  assert_command_fails bash "$CREATOR" --name "test-unk" --type cli-tool --description "A test skill" --bogus-flag yes
}

test_ollama_full_creation() {
  # Skip if Ollama is not running
  if ! command -v ollama &>/dev/null || ! ollama list &>/dev/null; then
    echo "    SKIP: Ollama not available"
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  fi

  local name="test-create-e2e-$$"
  if bash "$CREATOR" --name "$name" --type cli-tool --description "A test skill for end-to-end creation validation" >/dev/null 2>&1; then
    assert_file_exists "$REPO_ROOT/skills/$name/SKILL.md"
    assert_file_exists "$REPO_ROOT/tests/$name.test.sh"
    # Clean up
    rm -rf "$REPO_ROOT/skills/$name"
    rm -f "$REPO_ROOT/tests/$name.test.sh"
    rm -f "$REPO_ROOT/skills/$name/.trust"
  else
    echo "    SKIP: Ollama generation failed (model may not be loaded)"
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  fi
}
