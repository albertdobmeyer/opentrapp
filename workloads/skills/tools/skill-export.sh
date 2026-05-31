#!/usr/bin/env bash
# Skill exporter — packages certified skill for vault consumption
# Usage: skill-export.sh <skill-name>
# Output: exports/<skill-name>/SKILL.md + clearance-report.json + .trust
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found"; exit 1
}

SKILL_NAME="${1:-}"
if [[ -z "$SKILL_NAME" ]]; then
  echo "Usage: make export SKILL=<name>"
  exit 1
fi

SKILL_DIR="$REPO_ROOT/skills/$SKILL_NAME"
SKILL_FILE="$SKILL_DIR/SKILL.md"
REPORT_FILE="$SKILL_DIR/clearance-report.json"
TRUST_FILE="$SKILL_DIR/.trust"
EXPORT_DIR="$REPO_ROOT/exports/$SKILL_NAME"

if [[ ! -f "$SKILL_FILE" ]]; then
  echo "Error: Skill not found at $SKILL_FILE"
  exit 1
fi

log_header "Exporting $SKILL_NAME"
echo ""

# Check if certificate exists and is fresh
NEEDS_CERTIFY=true
if [[ -f "$REPORT_FILE" ]]; then
  STORED_CHECKSUM=$("$PY" -c "import json; print(json.load(open('$REPORT_FILE')).get('checksum',''))" 2>/dev/null) || STORED_CHECKSUM=""
  if [[ -n "$STORED_CHECKSUM" ]]; then
    CURRENT_CHECKSUM=$("$PY" -c "import hashlib; print('sha256:'+hashlib.sha256(open('$SKILL_FILE','rb').read()).hexdigest())")
    if [[ "$STORED_CHECKSUM" == "$CURRENT_CHECKSUM" ]]; then
      echo "  Certificate is fresh (checksum matches)."
      NEEDS_CERTIFY=false
    else
      echo "  Certificate is stale (checksum mismatch). Re-certifying..."
    fi
  fi
fi

if [[ "$NEEDS_CERTIFY" == true ]]; then
  echo "  Running certification pipeline..."
  echo ""
  bash "$SCRIPT_DIR/skill-certify.sh" "$SKILL_NAME" || {
    echo -e "${RED}Export failed: certification did not pass.${RESET}"
    exit 1
  }
  echo ""
fi

# Verify required files exist
for f in "$SKILL_FILE" "$REPORT_FILE" "$TRUST_FILE"; do
  if [[ ! -f "$f" ]]; then
    echo -e "${RED}Error: Missing $f${RESET}"
    exit 1
  fi
done

# Package into exports/
rm -rf "$EXPORT_DIR"
mkdir -p "$EXPORT_DIR"
cp "$SKILL_FILE" "$EXPORT_DIR/SKILL.md"
cp "$REPORT_FILE" "$EXPORT_DIR/clearance-report.json"
cp "$TRUST_FILE" "$EXPORT_DIR/.trust"

echo -e "  ${GREEN}Exported to $EXPORT_DIR/${RESET}"
echo ""
echo "  Contents:"
echo "    SKILL.md              ($(wc -c < "$EXPORT_DIR/SKILL.md") bytes)"
echo "    clearance-report.json ($(wc -c < "$EXPORT_DIR/clearance-report.json") bytes)"
echo "    .trust                ($(wc -c < "$EXPORT_DIR/.trust") bytes)"
echo ""
echo "  To install in vault:"
echo "    cd components/opencli-container"
echo "    bash scripts/install-skill.sh ../openagent-skills/exports/$SKILL_NAME/ \\"
echo "      --clearance ../openagent-skills/exports/$SKILL_NAME/clearance-report.json"
