#!/usr/bin/env bash
# CDR Orchestrator — Content Disarm & Reconstruction pipeline
# Usage:
#   skill-cdr.sh <path-to-SKILL.md>          Local file CDR
#   skill-cdr.sh --download <skill-name>      Download + CDR
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found"; exit 1
}

# Parse arguments
DOWNLOAD_MODE=false
INPUT_PATH=""
SKILL_NAME=""

if [[ "${1:-}" == "--download" ]]; then
  DOWNLOAD_MODE=true
  SKILL_NAME="${2:-}"
  if [[ -z "$SKILL_NAME" ]]; then
    echo "Usage: skill-cdr.sh --download <skill-name>"
    exit 1
  fi
else
  INPUT_PATH="${1:-}"
  if [[ -z "$INPUT_PATH" || ! -f "$INPUT_PATH" ]]; then
    echo "Usage: skill-cdr.sh <path-to-SKILL.md>"
    echo "   or: skill-cdr.sh --download <skill-name>"
    exit 1
  fi
fi

log_header "CDR Pipeline: Content Disarm & Reconstruction"
echo ""

# ── Stage 1: Quarantine ──
echo -e "${BOLD}[1/8] Quarantine${RESET}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

if [[ "$DOWNLOAD_MODE" == true ]]; then
  echo "  Downloading $SKILL_NAME from ClawHub..."
  QUARANTINE_OUTPUT=$(bash "$SCRIPT_DIR/skill-download.sh" "$SKILL_NAME" 2>&1) || {
    echo -e "${RED}  Download failed.${RESET}"
    echo "$QUARANTINE_OUTPUT" | tail -3
    exit 1
  }
  INPUT_PATH=$(echo "$QUARANTINE_OUTPUT" | tail -1)
  QUARANTINE_DIR=$(dirname "$INPUT_PATH")
else
  # Local file — copy to quarantine
  TEMP_NAME=$(basename "$INPUT_PATH" .md | tr '[:upper:]' '[:lower:]' | tr ' ' '-')
  QUARANTINE_DIR="$REPO_ROOT/quarantine/${TEMP_NAME}-${TIMESTAMP}"
  mkdir -p "$QUARANTINE_DIR"
  cp "$INPUT_PATH" "$QUARANTINE_DIR/SKILL.md"
  INPUT_PATH="$QUARANTINE_DIR/SKILL.md"
fi

# Ensure cleanup on exit (success or failure)
cleanup() {
  if [[ -d "${QUARANTINE_DIR:-}" ]]; then
    rm -rf "$QUARANTINE_DIR"
  fi
}
trap cleanup EXIT

echo -e "  ${GREEN}PASS${RESET} — quarantined at $QUARANTINE_DIR/"

# ── Stage 2: Structural Parse ──
echo -e "${BOLD}[2/8] Structural Parse${RESET}"
STRUCTURED_JSON="$QUARANTINE_DIR/structured.json"

if ! "$PY" "$SCRIPT_DIR/lib/cdr-parse.py" "$INPUT_PATH" > "$STRUCTURED_JSON" 2>&1; then
  echo -e "  ${RED}FAIL — could not parse markdown structure${RESET}"
  exit 1
fi
SECTION_COUNT=$("$PY" -c "import json; print(len(json.load(open('$STRUCTURED_JSON')).get('sections',[])))")
echo -e "  ${GREEN}PASS${RESET} — $SECTION_COUNT sections extracted"

# ── Stage 3: Pre-Filter ──
echo -e "${BOLD}[3/8] Pre-Filter${RESET}"
FILTERED_JSON="$QUARANTINE_DIR/filtered.json"

PREFILTER_STDERR=""
if ! bash "$SCRIPT_DIR/lib/cdr-prefilter.sh" "$STRUCTURED_JSON" > "$FILTERED_JSON" 2>/tmp/cdr-prefilter-err.txt; then
  PREFILTER_STDERR=$(cat /tmp/cdr-prefilter-err.txt)
  rm -f /tmp/cdr-prefilter-err.txt
  echo -e "  ${RED}REJECTED — dangerous content detected${RESET}"
  echo "  $PREFILTER_STDERR"
  exit 1
fi
PREFILTER_STDERR=$(cat /tmp/cdr-prefilter-err.txt 2>/dev/null || true)
rm -f /tmp/cdr-prefilter-err.txt
if [[ -n "$PREFILTER_STDERR" ]]; then
  echo "  $PREFILTER_STDERR"
fi
echo -e "  ${GREEN}PASS${RESET} — safe content extracted"

# ── Stage 4: Intent Extraction ──
echo -e "${BOLD}[4/8] Intent Extraction (Ollama)${RESET}"
INTENT_JSON="$QUARANTINE_DIR/intent.json"

if ! bash "$SCRIPT_DIR/lib/cdr-intent.sh" "$FILTERED_JSON" > "$INTENT_JSON" 2>&1; then
  echo -e "  ${RED}FAIL — intent extraction failed${RESET}"
  cat "$INTENT_JSON" >&2
  exit 1
fi
echo -e "  ${GREEN}PASS${RESET} — intent extracted"

# ── Stage 5: Intent Validation ──
echo -e "${BOLD}[5/8] Intent Validation${RESET}"

if ! "$PY" "$SCRIPT_DIR/lib/cdr-validate.py" "$INTENT_JSON" > /dev/null 2>&1; then
  VALIDATION_ERRORS=$("$PY" "$SCRIPT_DIR/lib/cdr-validate.py" "$INTENT_JSON" 2>&1 || true)
  echo -e "  ${RED}FAIL — intent does not match schema${RESET}"
  echo "  $VALIDATION_ERRORS"
  exit 1
fi
echo -e "  ${GREEN}PASS${RESET} — schema valid"

# ── Stage 6: Reconstruction ──
echo -e "${BOLD}[6/8] Reconstruction${RESET}"
RECON_DIR="$QUARANTINE_DIR/reconstructed"
mkdir -p "$RECON_DIR"

if ! "$PY" "$SCRIPT_DIR/lib/cdr-reconstruct.py" "$INTENT_JSON" "$RECON_DIR/SKILL.md" > /dev/null 2>&1; then
  echo -e "  ${RED}FAIL — reconstruction failed${RESET}"
  exit 1
fi
RECON_LINES=$(wc -l < "$RECON_DIR/SKILL.md")
echo -e "  ${GREEN}PASS${RESET} — reconstructed ($RECON_LINES lines)"

# ── Stage 7: Post-Verify ──
echo -e "${BOLD}[7/8] Post-Verification${RESET}"

# Lint
if ! bash "$SCRIPT_DIR/skill-lint.sh" "$RECON_DIR" > /dev/null 2>&1; then
  echo -e "  ${RED}FAIL — reconstruction failed lint${RESET}"
  exit 1
fi

# Scan
SCAN_RESULT=$(bash "$SCRIPT_DIR/skill-scan.sh" --json "$RECON_DIR" 2>/dev/null) || true
SCAN_BLOCKED=$("$PY" -c "import sys,json; print(json.load(sys.stdin).get('blocked',1))" <<< "$SCAN_RESULT" 2>/dev/null) || SCAN_BLOCKED=1
if [[ "$SCAN_BLOCKED" != "0" ]]; then
  echo -e "  ${RED}FAIL — reconstruction has scanner findings${RESET}"
  exit 1
fi

# Verify
if ! bash "$SCRIPT_DIR/skill-verify.sh" "$RECON_DIR" > /dev/null 2>&1; then
  echo -e "  ${RED}FAIL — reconstruction failed zero-trust verification${RESET}"
  exit 1
fi

echo -e "  ${GREEN}PASS${RESET} — lint + scan + verify all clean"

# ── Stage 8: Deliver + Cleanup ──
echo -e "${BOLD}[8/8] Deliver${RESET}"

# Extract skill name from reconstructed frontmatter
CDR_SKILL_NAME=$("$PY" -c "
import sys
with open('$RECON_DIR/SKILL.md') as f:
    for line in f:
        line = line.strip()
        if line.startswith('name:'):
            print(line.split(':', 1)[1].strip())
            break
" 2>/dev/null)

if [[ -z "$CDR_SKILL_NAME" ]]; then
  echo -e "  ${RED}FAIL — could not extract skill name from reconstruction${RESET}"
  exit 1
fi

DEST_DIR="$REPO_ROOT/skills/$CDR_SKILL_NAME"
if [[ -d "$DEST_DIR" ]]; then
  echo "  Skill '$CDR_SKILL_NAME' already exists — overwriting with CDR'd version."
fi

mkdir -p "$DEST_DIR"
cp "$RECON_DIR/SKILL.md" "$DEST_DIR/SKILL.md"

# Generate trust file
bash "$SCRIPT_DIR/skill-verify.sh" --trust "$DEST_DIR" > /dev/null 2>&1 || true

echo -e "  ${GREEN}PASS${RESET} — delivered to skills/$CDR_SKILL_NAME/"

# Cleanup happens via trap
echo ""
echo -e "${GREEN}CDR complete: $CDR_SKILL_NAME${RESET}"
echo "  Run 'make certify SKILL=$CDR_SKILL_NAME' to generate security certificate."
