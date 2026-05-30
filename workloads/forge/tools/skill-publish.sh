#!/usr/bin/env bash
# Gated publish pipeline: lint → scan → test → molthub publish
# Usage: skill-publish.sh <skill_name> <version>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

SKILL_NAME="${1:-}"
VERSION="${2:-}"

if [[ -z "$SKILL_NAME" || -z "$VERSION" ]]; then
  echo "Usage: make publish SKILL=<name> VERSION=<x.y.z>"
  exit 1
fi

SKILL_DIR="$REPO_ROOT/skills/$SKILL_NAME"
SKILL_FILE="$SKILL_DIR/SKILL.md"

if [[ ! -f "$SKILL_FILE" ]]; then
  echo "Error: Skill not found at $SKILL_FILE"
  exit 1
fi

# Validate version format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: Version must be in semver format (e.g., 1.0.0)"
  exit 1
fi

log_header "Publishing $SKILL_NAME@$VERSION"
echo ""

# Gate 1: Lint
echo -e "${BOLD}Step 1/5: Lint${RESET}"
if ! bash "$SCRIPT_DIR/skill-lint.sh" "$SKILL_DIR"; then
  echo ""
  echo -e "${RED}BLOCKED: Lint failed. Fix issues before publishing.${RESET}"
  exit 1
fi

# Gate 2: Scan
echo ""
echo -e "${BOLD}Step 2/5: Security Scan${RESET}"
if ! bash "$SCRIPT_DIR/skill-scan.sh" "$SKILL_DIR"; then
  echo ""
  echo -e "${RED}BLOCKED: Security scan found critical issues. Fix or allowlist before publishing.${RESET}"
  exit 1
fi

# Gate 3: Zero-trust verification
echo ""
echo -e "${BOLD}Step 3/5: Zero-Trust Verification${RESET}"
if ! bash "$SCRIPT_DIR/skill-verify.sh" --strict --trust "$SKILL_DIR"; then
  echo ""
  echo -e "${RED}BLOCKED: Zero-trust verification failed. Every line must be classifiable as safe.${RESET}"
  exit 1
fi

# Gate 4: Test (if test file exists)
echo ""
echo -e "${BOLD}Step 4/5: Tests${RESET}"
if [[ ! -f "$REPO_ROOT/tests/${SKILL_NAME}.test.sh" ]]; then
  echo ""
  echo -e "${RED}BLOCKED: No test file found (tests/${SKILL_NAME}.test.sh).${RESET}"
  echo "  Every skill must have a test file to publish. Run: make new SKILL=$SKILL_NAME"
  exit 1
fi
if ! bash "$SCRIPT_DIR/skill-test.sh" "$SKILL_NAME"; then
  echo ""
  echo -e "${RED}BLOCKED: Tests failed. Fix before publishing.${RESET}"
  exit 1
fi


# Gate 5: Publish
echo ""
echo -e "${BOLD}Step 5/5: Publish${RESET}"

# Extract display name from H1 heading
DISPLAY_NAME=$(get_skill_title "$SKILL_FILE")

echo "  Skill:   $SKILL_NAME"
echo "  Title:   $DISPLAY_NAME"
echo "  Version: $VERSION"
echo ""

# Check if molthub is available
if ! command -v molthub &>/dev/null; then
  echo -e "${RED}Error: molthub not found. Install with: npm install -g molthub${RESET}"
  exit 1
fi

# Run molthub publish
cd "$SKILL_DIR"
molthub publish --version "$VERSION"

echo ""
echo -e "${GREEN}Published $SKILL_NAME@$VERSION${RESET}"
