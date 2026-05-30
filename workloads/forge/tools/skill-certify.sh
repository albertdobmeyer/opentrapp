#!/usr/bin/env bash
# Security certificate generator — 4-gate pipeline producing clearance-report.json
# Usage: skill-certify.sh <skill-name>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found"; exit 1
}

SKILL_NAME="${1:-}"
if [[ -z "$SKILL_NAME" ]]; then
  echo "Usage: make certify SKILL=<name>"
  exit 1
fi

SKILL_DIR="$REPO_ROOT/skills/$SKILL_NAME"
SKILL_FILE="$SKILL_DIR/SKILL.md"

if [[ ! -f "$SKILL_FILE" ]]; then
  echo "Error: Skill not found at $SKILL_FILE"
  exit 1
fi

# Extract and validate version from frontmatter
SKILL_VERSION=$(get_frontmatter_field "$SKILL_FILE" "version" || true)
if [[ -z "$SKILL_VERSION" ]]; then
  echo -e "${RED}BLOCKED: No 'version' field in frontmatter. Add version: X.Y.Z to SKILL.md.${RESET}"
  exit 1
fi
if ! echo "$SKILL_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo -e "${RED}BLOCKED: Invalid version '$SKILL_VERSION'. Must be semver (X.Y.Z).${RESET}"
  exit 1
fi

log_header "Certifying $SKILL_NAME@$SKILL_VERSION"
echo ""

# ── Gate 1: Lint ──
echo -e "${BOLD}Gate 1/4: Lint${RESET}"
if ! bash "$SCRIPT_DIR/skill-lint.sh" "$SKILL_DIR" > /dev/null 2>&1; then
  echo -e "${RED}BLOCKED: Lint failed. Fix issues before certifying.${RESET}"
  exit 1
fi
echo -e "  ${GREEN}PASS${RESET}"

# ── Gate 2: Scan ──
echo -e "${BOLD}Gate 2/4: Security Scan${RESET}"
SCAN_JSON=$(bash "$SCRIPT_DIR/skill-scan.sh" --json "$SKILL_DIR" 2>/dev/null) || true
SCAN_BLOCKED=$("$PY" -c "import sys,json; print(json.load(sys.stdin).get('blocked',1))" <<< "$SCAN_JSON" 2>/dev/null) || SCAN_BLOCKED=1
if [[ "$SCAN_BLOCKED" != "0" ]]; then
  echo -e "${RED}BLOCKED: Security scan has unresolved findings.${RESET}"
  exit 1
fi
echo -e "  ${GREEN}PASS${RESET}"

# ── Gate 3: Verify ──
echo -e "${BOLD}Gate 3/4: Zero-Trust Verification${RESET}"
if ! bash "$SCRIPT_DIR/skill-verify.sh" --trust "$SKILL_DIR" > /dev/null 2>&1; then
  echo -e "${RED}BLOCKED: Zero-trust verification failed.${RESET}"
  exit 1
fi
echo -e "  ${GREEN}PASS${RESET}"

# ── Gate 4: Test ──
echo -e "${BOLD}Gate 4/4: Tests${RESET}"
TEST_FILE="$REPO_ROOT/tests/${SKILL_NAME}.test.sh"
if [[ ! -f "$TEST_FILE" ]]; then
  echo -e "${RED}BLOCKED: No test file (tests/${SKILL_NAME}.test.sh).${RESET}"
  exit 1
fi
TEST_OUTPUT=$(bash "$SCRIPT_DIR/skill-test.sh" "$SKILL_NAME" 2>&1) || {
  echo -e "${RED}BLOCKED: Tests failed.${RESET}"
  echo "$TEST_OUTPUT"
  exit 1
}
echo -e "  ${GREEN}PASS${RESET}"

# ── Generate certificate ──
echo ""
echo -e "${BOLD}Generating clearance report...${RESET}"

# Extract data from scan JSON
SCAN_CRITICAL=$("$PY" -c "import sys,json; s=json.load(sys.stdin)['summary']; print(s['critical'])" <<< "$SCAN_JSON")
SCAN_HIGH=$("$PY" -c "import sys,json; s=json.load(sys.stdin)['summary']; print(s['high'])" <<< "$SCAN_JSON")
SCAN_MEDIUM=$("$PY" -c "import sys,json; s=json.load(sys.stdin)['summary']; print(s['medium'])" <<< "$SCAN_JSON")
SCAN_PATTERNS=$("$PY" -c "import sys,json; print(json.load(sys.stdin)['patternCount'])" <<< "$SCAN_JSON")

# Extract verify data from .trust file
TRUST_FILE="$SKILL_DIR/.trust"
VERIFY_TOTAL=$(grep '^LINES_TOTAL=' "$TRUST_FILE" | cut -d= -f2)
VERIFY_SAFE=$(grep '^LINES_SAFE=' "$TRUST_FILE" | cut -d= -f2)
VERIFY_SUSPICIOUS=$(grep '^LINES_SUSPICIOUS=' "$TRUST_FILE" | cut -d= -f2)
VERIFY_MALICIOUS=0  # Must be 0 if VERIFIED

# Extract test counts from output
TEST_PASSED=$(echo "$TEST_OUTPUT" | grep -oP 'Passed: \K[0-9]+' || echo "0")
TEST_FAILED=$(echo "$TEST_OUTPUT" | grep -oP 'Failed: \K[0-9]+' || echo "0")

# Compute SHA-256 checksum of SKILL.md
CHECKSUM=$("$PY" -c "
import hashlib
with open('$SKILL_FILE', 'rb') as f:
    print('sha256:' + hashlib.sha256(f.read()).hexdigest())
")

# Timestamp
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%Y-%m-%dT%H:%M:%SZ)

# Pattern version (year.month from current date)
PATTERN_VERSION=$(date -u +%Y.%m 2>/dev/null || date +%Y.%m)

# Write certificate
REPORT_FILE="$SKILL_DIR/clearance-report.json"
"$PY" -c "
import json, sys
report = {
    'forge_version': '1.0.0',
    'skill': '$SKILL_NAME',
    'version': '$SKILL_VERSION',
    'certified_at': '$TIMESTAMP',
    'scan': {
        'status': 'PASS',
        'critical': $SCAN_CRITICAL,
        'high': $SCAN_HIGH,
        'medium': $SCAN_MEDIUM,
        'pattern_count': $SCAN_PATTERNS,
        'pattern_version': '$PATTERN_VERSION'
    },
    'verify': {
        'verdict': 'VERIFIED',
        'total_lines': $VERIFY_TOTAL,
        'safe_lines': $VERIFY_SAFE,
        'suspicious_lines': $VERIFY_SUSPICIOUS,
        'malicious_lines': $VERIFY_MALICIOUS
    },
    'test': {
        'status': 'PASS',
        'assertions': $TEST_PASSED,
        'failures': $TEST_FAILED
    },
    'checksum': '$CHECKSUM'
}
with open('$REPORT_FILE', 'w') as f:
    json.dump(report, f, indent=2)
    f.write('\n')
print('OK')
" || {
  echo -e "${RED}Failed to write clearance report.${RESET}"
  exit 1
}

echo -e "  ${GREEN}Certificate: $REPORT_FILE${RESET}"
echo ""
echo -e "${GREEN}$SKILL_NAME@$SKILL_VERSION certified.${RESET}"
