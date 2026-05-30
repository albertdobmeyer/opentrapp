#!/usr/bin/env bash
# Vault Skill Guard — checks installed skills have valid trust files
# Usage: verify-skills.sh
# Warns about skills that bypassed the forge pipeline.
set -uo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME="podman"
command -v podman &>/dev/null || RUNTIME="docker"
CONTAINER="opencli-container"
WORKSPACE_SKILLS="/home/vault/.openclaw/workspace/skills"

BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo ""
echo -e "${BOLD}Vault Skill Guard${NC}"
echo "=================="
echo ""

# Check container is running
if ! $RUNTIME inspect "$CONTAINER" --format '{{.State.Status}}' 2>/dev/null | grep -q "running"; then
  echo -e "${YELLOW}Container not running. No skills to verify.${NC}"
  exit 0
fi

# List installed skills
SKILL_PATHS=$($RUNTIME exec "$CONTAINER" sh -c "find $WORKSPACE_SKILLS -name 'SKILL.md' -type f 2>/dev/null" 2>&1) || SKILL_PATHS=""

if [[ -z "$SKILL_PATHS" ]]; then
  echo "  No skills installed."
  echo ""
  exit 0
fi

VERIFIED=0
UNVERIFIED=0
MODIFIED=0

while IFS= read -r skill_path; do
  [[ -z "$skill_path" ]] && continue
  skill_name=$(echo "$skill_path" | sed "s|$WORKSPACE_SKILLS/||;s|/SKILL.md||")
  trust_path="$WORKSPACE_SKILLS/$skill_name/.trust"

  # Check trust file exists
  if ! $RUNTIME exec "$CONTAINER" sh -c "test -f '$trust_path'" 2>/dev/null; then
    echo -e "  ${RED}UNVERIFIED${NC}  $skill_name (no trust file)"
    UNVERIFIED=$((UNVERIFIED + 1))
    continue
  fi

  # Check hash matches
  STORED_HASH=$($RUNTIME exec "$CONTAINER" sh -c "grep '^VERIFY_HASH=' '$trust_path' 2>/dev/null | cut -d= -f2") || STORED_HASH=""
  CURRENT_HASH=$($RUNTIME exec "$CONTAINER" sh -c "find '$WORKSPACE_SKILLS/$skill_name' -maxdepth 2 -type f ! -name '.trust' ! -name '.scanignore' | sort | xargs cat 2>/dev/null | sha256sum | cut -d' ' -f1") || CURRENT_HASH=""

  if [[ -n "$STORED_HASH" && "sha256:$CURRENT_HASH" == "$STORED_HASH" ]]; then
    echo -e "  ${GREEN}VERIFIED${NC}   $skill_name"
    VERIFIED=$((VERIFIED + 1))
  else
    echo -e "  ${YELLOW}MODIFIED${NC}   $skill_name (hash mismatch since verification)"
    MODIFIED=$((MODIFIED + 1))
  fi

done <<< "$SKILL_PATHS"

echo ""
echo "Summary: $VERIFIED verified, $UNVERIFIED unverified, $MODIFIED modified"

if (( UNVERIFIED > 0 )); then
  echo ""
  echo -e "${YELLOW}WARNING: $UNVERIFIED skill(s) have no trust file.${NC}"
  echo "  These skills bypassed the forge security pipeline."
  echo "  Run: cd components/openskill-forge && make certify SKILL=<name>"
fi

if (( MODIFIED > 0 )); then
  echo ""
  echo -e "${YELLOW}WARNING: $MODIFIED skill(s) modified after verification.${NC}"
  echo "  Re-certify with: cd components/openskill-forge && make certify SKILL=<name>"
fi

echo ""
