#!/usr/bin/env bash
# Scaffold a new skill from a template
# Usage: skill-new.sh <name> [type]
# Types: cli-tool (default), workflow, language-ref
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

SKILL_NAME="${1:-}"
SKILL_TYPE="${2:-cli-tool}"

if [[ -z "$SKILL_NAME" ]]; then
  echo "Usage: make new SKILL=<name> [TYPE=cli-tool|workflow|language-ref]"
  echo ""
  echo "Templates:"
  echo "  cli-tool      Command-line tool reference (default)"
  echo "  workflow       Process/methodology skill"
  echo "  language-ref   Language/syntax reference"
  exit 1
fi

# Validate name is a slug
if ! echo "$SKILL_NAME" | grep -qE '^[a-z0-9]([a-z0-9-]*[a-z0-9])?$'; then
  echo "Error: Skill name must be a valid slug (lowercase letters, numbers, hyphens)"
  echo "  Example: my-tool, csv-parser, git-helpers"
  exit 1
fi

# Check template exists
TEMPLATE_DIR="$REPO_ROOT/templates/$SKILL_TYPE"
if [[ ! -d "$TEMPLATE_DIR" ]]; then
  echo "Error: Template type '$SKILL_TYPE' not found"
  echo "  Available: cli-tool, workflow, language-ref"
  exit 1
fi

# Check skill doesn't already exist
SKILL_DIR="$REPO_ROOT/skills/$SKILL_NAME"
if [[ -d "$SKILL_DIR" ]]; then
  echo "Error: Skill '$SKILL_NAME' already exists at $SKILL_DIR"
  exit 1
fi

# Generate title from slug
SKILL_TITLE=$(echo "$SKILL_NAME" | sed 's/-/ /g' | sed 's/\b\(.\)/\u\1/g')

# Create skill directory and copy template
mkdir -p "$SKILL_DIR"
cp "$TEMPLATE_DIR/SKILL.md" "$SKILL_DIR/SKILL.md"

# Replace placeholders in SKILL.md
sed -i "s/{{SKILL_NAME}}/$SKILL_NAME/g" "$SKILL_DIR/SKILL.md"
sed -i "s/{{SKILL_TITLE}}/$SKILL_TITLE/g" "$SKILL_DIR/SKILL.md"

# Generate test file from template
TEST_FILE="$REPO_ROOT/tests/${SKILL_NAME}.test.sh"
TEST_TEMPLATE="$REPO_ROOT/templates/_test.template.sh"
if [[ -f "$TEST_TEMPLATE" && ! -f "$TEST_FILE" ]]; then
  cp "$TEST_TEMPLATE" "$TEST_FILE"
  sed -i "s/{{SKILL_NAME}}/$SKILL_NAME/g" "$TEST_FILE"
fi

echo ""
echo -e "${GREEN}Created skill: $SKILL_NAME${RESET}"
echo -e "  Type:     $SKILL_TYPE"
echo -e "  Skill:    skills/$SKILL_NAME/SKILL.md"
echo -e "  Test:     tests/$SKILL_NAME.test.sh"
echo -e "  Template: templates/$SKILL_TYPE/SKILL.md"
echo ""
echo "Next steps:"
echo "  1. Edit skills/$SKILL_NAME/SKILL.md"
echo "  2. Edit tests/$SKILL_NAME.test.sh — add domain-specific tests"
echo "  3. Run: make check SKILL=$SKILL_NAME"
echo ""
