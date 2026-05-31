#!/usr/bin/env bash
# Download a skill from ClawHub to the quarantine directory
# Usage: skill-download.sh <skill-name>
# Output: prints quarantine path on success
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

SKILL_NAME="${1:-}"
if [[ -z "$SKILL_NAME" ]]; then
  echo "Usage: skill-download.sh <skill-name>"
  exit 1
fi

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
QUARANTINE_DIR="$REPO_ROOT/quarantine/${SKILL_NAME}-${TIMESTAMP}"

log_header "Downloading $SKILL_NAME from ClawHub"
echo ""

# Create quarantine directory
mkdir -p "$QUARANTINE_DIR"

# Download from ClawHub API
API_URL="https://clawdhub.com/api/v1/skills/${SKILL_NAME}/raw"
echo "  Fetching from $API_URL..."

HTTP_CODE=$(curl -sf -o "$QUARANTINE_DIR/SKILL.md" -w "%{http_code}" "$API_URL" 2>/dev/null) || {
  rm -rf "$QUARANTINE_DIR"
  echo -e "${RED}Error: Failed to download from ClawHub API.${RESET}"
  echo "  The API may be unavailable. Use local file instead:"
  echo "  make cdr FILE=path/to/SKILL.md"
  exit 1
}

if [[ "$HTTP_CODE" != "200" ]]; then
  rm -rf "$QUARANTINE_DIR"
  echo -e "${RED}Error: ClawHub returned HTTP $HTTP_CODE for skill '$SKILL_NAME'.${RESET}"
  exit 1
fi

# Basic sanity check — must have frontmatter
if ! head -1 "$QUARANTINE_DIR/SKILL.md" | grep -q '^---$'; then
  rm -rf "$QUARANTINE_DIR"
  echo -e "${RED}Error: Downloaded file has no YAML frontmatter. Not a valid SKILL.md.${RESET}"
  exit 1
fi

FILE_SIZE=$(wc -c < "$QUARANTINE_DIR/SKILL.md")
echo -e "  ${GREEN}Downloaded to quarantine ($FILE_SIZE bytes)${RESET}"
echo "  Path: $QUARANTINE_DIR/SKILL.md"
echo ""

# Output the quarantine path (for piping to skill-cdr.sh)
echo "$QUARANTINE_DIR/SKILL.md"
