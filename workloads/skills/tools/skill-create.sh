#!/usr/bin/env bash
# AI-Assisted Skill Creation Wizard
# Usage:
#   skill-create.sh                                    Interactive mode
#   skill-create.sh --name N --type T --description D  Non-interactive mode
# Optional: --commands "cmd1,cmd2" --tips "tip1,tip2"
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found"; exit 1
}

# ── Parse arguments ──
SKILL_NAME=""
SKILL_TYPE=""
DESCRIPTION=""
COMMANDS=""
TIPS=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --name)       SKILL_NAME="$2"; shift 2 ;;
    --type)       SKILL_TYPE="$2"; shift 2 ;;
    --description) DESCRIPTION="$2"; shift 2 ;;
    --commands)   COMMANDS="$2"; shift 2 ;;
    --tips)       TIPS="$2"; shift 2 ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Usage: skill-create.sh [--name N --type T --description D] [--commands C] [--tips T]" >&2
      exit 1 ;;
  esac
done

# Determine if we need interactive mode
INTERACTIVE=false
if [[ -z "$SKILL_NAME" || -z "$SKILL_TYPE" || -z "$DESCRIPTION" ]]; then
  INTERACTIVE=true
fi

log_header "OpenSkill Forge — AI-Assisted Skill Creation"
echo ""

# ── Step 1: Skill Name ──
echo -e "${BOLD}[1/6] Skill Name${RESET}"
if [[ -z "$SKILL_NAME" ]]; then
  printf "  Enter a name (lowercase, hyphens): "
  read -r SKILL_NAME
fi

# Validate slug
if ! echo "$SKILL_NAME" | grep -qE '^[a-z0-9]([a-z0-9-]*[a-z0-9])?$'; then
  echo -e "  ${RED}Error: Name must be a valid slug (lowercase letters, numbers, hyphens)${RESET}"
  exit 1
fi

# Check for collision
if [[ -d "$REPO_ROOT/skills/$SKILL_NAME" ]]; then
  echo -e "  ${RED}Error: Skill '$SKILL_NAME' already exists at skills/$SKILL_NAME/${RESET}"
  exit 1
fi
echo -e "  ${GREEN}OK${RESET} — $SKILL_NAME"

# ── Step 2: Template Type ──
echo -e "${BOLD}[2/6] Template Type${RESET}"
if [[ -z "$SKILL_TYPE" ]]; then
  echo "  1) cli-tool      — Command-line tool reference"
  echo "  2) workflow       — Process/methodology guide"
  echo "  3) language-ref   — Language/syntax reference"
  printf "  Select [1-3]: "
  read -r TYPE_CHOICE
  case "$TYPE_CHOICE" in
    1) SKILL_TYPE="cli-tool" ;;
    2) SKILL_TYPE="workflow" ;;
    3) SKILL_TYPE="language-ref" ;;
    *)
      echo -e "  ${RED}Error: Invalid choice${RESET}"
      exit 1 ;;
  esac
fi

# Validate type
case "$SKILL_TYPE" in
  cli-tool|workflow|language-ref) ;;
  *)
    echo -e "  ${RED}Error: Invalid type '$SKILL_TYPE'. Must be: cli-tool, workflow, language-ref${RESET}"
    exit 1 ;;
esac
echo -e "  ${GREEN}OK${RESET} — $SKILL_TYPE"

# ── Step 3: Description ──
echo -e "${BOLD}[3/6] Description${RESET}"
if [[ -z "$DESCRIPTION" ]]; then
  echo "  Describe what this skill should teach (2-3 sentences):"
  printf "  > "
  read -r DESCRIPTION
fi

if [[ ${#DESCRIPTION} -lt 10 ]]; then
  echo -e "  ${RED}Error: Description too short (minimum 10 characters)${RESET}"
  exit 1
fi
echo -e "  ${GREEN}OK${RESET}"

# ── Step 4: Key Commands ──
echo -e "${BOLD}[4/6] Key Commands${RESET}"
if [[ -z "$COMMANDS" ]] && [[ "$INTERACTIVE" == true ]]; then
  echo "  Comma-separated commands, or blank to let AI decide:"
  printf "  > "
  read -r COMMANDS
fi

if [[ -n "$COMMANDS" ]]; then
  echo -e "  ${GREEN}OK${RESET} — user-provided"
else
  echo -e "  ${GREEN}OK${RESET} — AI will generate"
fi

# ── Step 5: Tips ──
echo -e "${BOLD}[5/6] Tips${RESET}"
if [[ -z "$TIPS" ]] && [[ "$INTERACTIVE" == true ]]; then
  echo "  Comma-separated tips, or blank to let AI decide:"
  printf "  > "
  read -r TIPS
fi

if [[ -n "$TIPS" ]]; then
  echo -e "  ${GREEN}OK${RESET} — user-provided"
else
  echo -e "  ${GREEN}OK${RESET} — AI will generate"
fi

# ── Step 6: Draft + Verify + Test ──
echo -e "${BOLD}[6/6] Drafting with Ollama...${RESET}"

# Build input JSON
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Convert comma-separated to JSON arrays
COMMANDS_JSON=$("$PY" -c "
import json, sys
val = sys.argv[1].strip()
if val:
    items = [x.strip() for x in val.split(',') if x.strip()]
    print(json.dumps(items))
else:
    print('[]')
" "$COMMANDS")

TIPS_JSON=$("$PY" -c "
import json, sys
val = sys.argv[1].strip()
if val:
    items = [x.strip() for x in val.split(',') if x.strip()]
    print(json.dumps(items))
else:
    print('[]')
" "$TIPS")

# Write input JSON
"$PY" -c "
import json, sys
data = {
    'name': sys.argv[1],
    'type': sys.argv[2],
    'description': sys.argv[3],
    'commands': json.loads(sys.argv[4]),
    'tips': json.loads(sys.argv[5])
}
json.dump(data, open(sys.argv[6], 'w'), indent=2)
" "$SKILL_NAME" "$SKILL_TYPE" "$DESCRIPTION" "$COMMANDS_JSON" "$TIPS_JSON" "$TMPDIR/input.json"

# Create temp skill dir for pipeline verification
DRAFT_DIR="$TMPDIR/draft"
mkdir -p "$DRAFT_DIR"

# Draft generation with retry loop
MAX_RETRIES=2
ATTEMPT=0
ERRORS=""

while (( ATTEMPT <= MAX_RETRIES )); do
  if (( ATTEMPT == 0 )); then
    echo -e "  Generating draft..."
    if ! bash "$SCRIPT_DIR/lib/create-draft.sh" "$TMPDIR/input.json" "$DRAFT_DIR/SKILL.md" 2>"$TMPDIR/draft-err.txt"; then
      DRAFT_ERR=$(cat "$TMPDIR/draft-err.txt")
      echo -e "  ${RED}FAIL — draft generation failed${RESET}"
      echo "  $DRAFT_ERR"
      exit 1
    fi
  else
    echo -e "  Retry $ATTEMPT/$MAX_RETRIES — fixing issues..."
    if ! bash "$SCRIPT_DIR/lib/create-draft.sh" "$TMPDIR/input.json" "$DRAFT_DIR/SKILL.md" "$ERRORS" 2>"$TMPDIR/draft-err.txt"; then
      DRAFT_ERR=$(cat "$TMPDIR/draft-err.txt")
      echo -e "  ${RED}FAIL — retry draft generation failed${RESET}"
      echo "  $DRAFT_ERR"
      exit 1
    fi
  fi

  # Patch missing version field (common with small models)
  if ! grep -q '^version:' "$DRAFT_DIR/SKILL.md" 2>/dev/null; then
    sed -i '/^name:/a version: 1.0.0' "$DRAFT_DIR/SKILL.md"
  fi

  DRAFT_LINES=$(wc -l < "$DRAFT_DIR/SKILL.md")
  echo -e "  Draft generated ($DRAFT_LINES lines)"

  # Pipeline verification
  echo -e "  Verifying draft..."
  ERRORS=""
  PIPELINE_OK=true

  # Lint
  if ! LINT_OUT=$(bash "$SCRIPT_DIR/skill-lint.sh" "$DRAFT_DIR" 2>&1); then
    ERRORS+="Lint errors: $LINT_OUT"$'\n'
    PIPELINE_OK=false
    echo -e "  ${RED}Lint: FAIL${RESET}"
  else
    echo -e "  ${GREEN}Lint: PASS${RESET}"
  fi

  # Scan
  SCAN_RESULT=$(bash "$SCRIPT_DIR/skill-scan.sh" --json "$DRAFT_DIR" 2>/dev/null) || true
  SCAN_BLOCKED=$("$PY" -c "import sys,json; print(json.load(sys.stdin).get('blocked',1))" <<< "$SCAN_RESULT" 2>/dev/null) || SCAN_BLOCKED=1
  if [[ "$SCAN_BLOCKED" != "0" ]]; then
    ERRORS+="Scan findings: $SCAN_RESULT"$'\n'
    PIPELINE_OK=false
    echo -e "  ${RED}Scan: FAIL${RESET}"
  else
    echo -e "  ${GREEN}Scan: PASS${RESET}"
  fi

  # Verify
  if ! VERIFY_OUT=$(bash "$SCRIPT_DIR/skill-verify.sh" "$DRAFT_DIR" 2>&1); then
    ERRORS+="Verify errors: $VERIFY_OUT"$'\n'
    PIPELINE_OK=false
    echo -e "  ${RED}Verify: FAIL${RESET}"
  else
    echo -e "  ${GREEN}Verify: PASS${RESET}"
  fi

  if [[ "$PIPELINE_OK" == true ]]; then
    break
  fi

  ATTEMPT=$((ATTEMPT + 1))
  if (( ATTEMPT > MAX_RETRIES )); then
    echo ""
    echo -e "  ${YELLOW}WARNING: Draft did not pass full verification after $MAX_RETRIES retries.${RESET}"
    echo -e "  ${YELLOW}Saving draft anyway — review and fix manually.${RESET}"
    echo ""
  fi
done

# Test generation
echo ""
echo -e "  Generating tests..."
TEST_PATH="$TMPDIR/test.sh"
if bash "$SCRIPT_DIR/lib/create-tests.sh" "$DRAFT_DIR/SKILL.md" "$TEST_PATH" 2>"$TMPDIR/test-err.txt"; then
  TEST_LINES=$(wc -l < "$TEST_PATH")
  echo -e "  ${GREEN}Test file generated ($TEST_LINES lines)${RESET}"
else
  TEST_ERR=$(cat "$TMPDIR/test-err.txt")
  echo -e "  ${YELLOW}WARNING: Test generation failed — creating template instead${RESET}"
  [[ -n "$TEST_ERR" ]] && echo "  $TEST_ERR"
  # Fallback: copy test template
  TEST_TEMPLATE="$REPO_ROOT/templates/_test.template.sh"
  if [[ -f "$TEST_TEMPLATE" ]]; then
    cp "$TEST_TEMPLATE" "$TEST_PATH"
    sed -i "s/{{SKILL_NAME}}/$SKILL_NAME/g" "$TEST_PATH"
  else
    # Minimal fallback
    cat > "$TEST_PATH" << FALLBACK
#!/usr/bin/env bash
# Tests for ${SKILL_NAME} skill

test_has_frontmatter() {
  assert_frontmatter_field "\$SKILL" "name" "^${SKILL_NAME}\$"
  assert_frontmatter_field "\$SKILL" "version" "^[0-9]"
}

test_has_required_sections() {
  assert_section_exists "\$SKILL" "When to Use"
}

test_no_placeholders() {
  assert_not_contains "\$SKILL" "(TODO|FIXME|XXX)"
}
FALLBACK
  fi
fi

# ── Deliver ──
echo ""
DEST_DIR="$REPO_ROOT/skills/$SKILL_NAME"
mkdir -p "$DEST_DIR"
cp "$DRAFT_DIR/SKILL.md" "$DEST_DIR/SKILL.md"

TEST_DEST="$REPO_ROOT/tests/${SKILL_NAME}.test.sh"
cp "$TEST_PATH" "$TEST_DEST"

# Generate .trust file
bash "$SCRIPT_DIR/skill-verify.sh" --trust "$DEST_DIR" > /dev/null 2>&1 || true

echo -e "${GREEN}Created skill: $SKILL_NAME${RESET}"
echo -e "  Skill: skills/$SKILL_NAME/SKILL.md"
echo -e "  Test:  tests/$SKILL_NAME.test.sh"
echo ""
echo "Next steps:"
echo "  make certify SKILL=$SKILL_NAME"
echo ""
