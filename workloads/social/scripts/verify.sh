#!/usr/bin/env bash
# OpenAgent Social — Workbench Health Check
# Validates tools, config, patterns, and engagement level.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

ENV_FILE="$PROJECT_ROOT/config/.env"
PATTERNS_FILE="$PROJECT_ROOT/config/injection-patterns.yml"
ALLOWLIST_FILE="$PROJECT_ROOT/config/feed-allowlist.yml"
TOOLS_DIR="$PROJECT_ROOT/tools"

# Colors
if [[ -t 1 ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  BOLD='\033[1m'
  DIM='\033[2m'
  NC='\033[0m'
else
  RED='' GREEN='' YELLOW='' BOLD='' DIM='' NC=''
fi

PASS=0
WARN=0
FAIL=0

check_pass() { echo -e "  ${GREEN}PASS${NC}  $*"; PASS=$((PASS + 1)); }
check_warn() { echo -e "  ${YELLOW}WARN${NC}  $*"; WARN=$((WARN + 1)); }
check_fail() { echo -e "  ${RED}FAIL${NC}  $*"; FAIL=$((FAIL + 1)); }

echo -e "${BOLD}OpenAgent Social — Health Check${NC}"
echo ""

# ── 1. Core Files ────────────────────────────────────────
echo -e "${BOLD}1. Core Files${NC}"

if [[ -f "$ENV_FILE" ]]; then
  check_pass "config/.env exists"
else
  check_fail "config/.env missing (run make setup)"
fi

if [[ -f "$PATTERNS_FILE" ]]; then
  pattern_count=$(grep -c '^\s*- id:' "$PATTERNS_FILE" 2>/dev/null || echo "0")
  check_pass "injection-patterns.yml (${pattern_count} patterns)"
else
  check_fail "injection-patterns.yml missing"
fi

if [[ -f "$ALLOWLIST_FILE" ]]; then
  check_pass "feed-allowlist.yml exists"
else
  check_warn "feed-allowlist.yml missing"
fi

echo ""

# ── 2. Tool Executables ─────────────────────────────────
echo -e "${BOLD}2. Tool Executables${NC}"

for tool in feed-scanner.sh agent-census.sh identity-checklist.sh; do
  if [[ -x "$TOOLS_DIR/$tool" ]]; then
    check_pass "$tool executable"
  else
    check_fail "$tool not executable"
  fi
done

if [[ -x "$PROJECT_ROOT/scripts/engagement-control.sh" ]]; then
  check_pass "engagement-control.sh executable"
else
  check_fail "engagement-control.sh not executable"
fi

echo ""

# ── 3. Preset Files ─────────────────────────────────────
echo -e "${BOLD}3. Engagement Presets${NC}"

for preset in observer researcher participant; do
  if [[ -f "$PROJECT_ROOT/config/${preset}.env" ]]; then
    check_pass "config/${preset}.env exists"
  else
    check_fail "config/${preset}.env missing"
  fi
done

echo ""

# ── 4. Engagement Level ─────────────────────────────────
echo -e "${BOLD}4. Engagement Level${NC}"

if [[ ! -f "$ENV_FILE" ]]; then
  check_warn "Cannot check engagement level (no .env)"
  echo ""
else
  # shellcheck disable=SC1090
  source "$ENV_FILE"

  LEVEL="${ENGAGEMENT_LEVEL:-}"

  if [[ -z "$LEVEL" ]]; then
    check_warn "ENGAGEMENT_LEVEL not set (defaulting to observer)"
    LEVEL="observer"
  elif [[ "$LEVEL" == "observer" || "$LEVEL" == "researcher" || "$LEVEL" == "participant" ]]; then
    check_pass "ENGAGEMENT_LEVEL = ${LEVEL}"
  else
    check_fail "ENGAGEMENT_LEVEL invalid: ${LEVEL} (expected observer|researcher|participant)"
  fi

  # Level-specific checks
  case "$LEVEL" in
    observer)
      if [[ "${RATE_LIMIT_POSTS_PER_HOUR:-0}" == "0" &&
            "${RATE_LIMIT_COMMENTS_PER_HOUR:-0}" == "0" &&
            "${RATE_LIMIT_VOTES_PER_HOUR:-0}" == "0" ]]; then
        check_pass "Observer: all rate limits = 0 (read-only)"
      else
        check_warn "Observer: rate limits should be 0 (read-only mode)"
      fi
      if [[ -n "${MOLTBOOK_API_KEY:-}" ]]; then
        check_warn "Observer: API key set (not needed for read-only)"
      fi
      ;;
    researcher)
      if [[ "${FEED_SCAN_ENABLED:-false}" == "true" ]]; then
        check_pass "Researcher: feed scanning enabled"
      else
        check_fail "Researcher: feed scanning must be enabled"
      fi
      posts="${RATE_LIMIT_POSTS_PER_HOUR:-0}"
      if (( posts > 0 && posts <= 20 )); then
        check_pass "Researcher: post rate limit ${posts}/hr (within bounds)"
      elif (( posts == 0 )); then
        check_warn "Researcher: post rate limit = 0 (read-only, expected > 0)"
      else
        check_warn "Researcher: post rate limit ${posts}/hr (recommended max 20)"
      fi
      if [[ -z "${MOLTBOOK_API_KEY:-}" ]]; then
        check_warn "Researcher: API key not set"
      fi
      ;;
    participant)
      if [[ "${FEED_SCAN_ENABLED:-false}" == "true" ]]; then
        check_pass "Participant: feed scanning enabled"
      else
        check_fail "Participant: feed scanning must be enabled"
      fi
      posts="${RATE_LIMIT_POSTS_PER_HOUR:-0}"
      if (( posts > 0 && posts <= 50 )); then
        check_pass "Participant: post rate limit ${posts}/hr (within bounds)"
      elif (( posts == 0 )); then
        check_warn "Participant: post rate limit = 0 (expected > 0)"
      else
        check_warn "Participant: post rate limit ${posts}/hr (recommended max 50)"
      fi
      if [[ -z "${MOLTBOOK_API_KEY:-}" ]]; then
        check_warn "Participant: API key not set"
      fi
      if [[ -z "${AGENT_HANDLE:-}" ]]; then
        check_warn "Participant: agent handle not set"
      fi
      ;;
  esac

  echo ""
fi

# ── 5. Dependencies ─────────────────────────────────────
echo -e "${BOLD}5. Dependencies${NC}"

for dep in python3 curl bash; do
  if command -v "$dep" &>/dev/null; then
    check_pass "$dep available"
  else
    check_fail "$dep not found"
  fi
done

echo ""

# ── Summary ──────────────────────────────────────────────
echo -e "${BOLD}Results${NC}"
echo -e "  ${GREEN}Passed: ${PASS}${NC}"
if (( WARN > 0 )); then
  echo -e "  ${YELLOW}Warnings: ${WARN}${NC}"
fi
if (( FAIL > 0 )); then
  echo -e "  ${RED}Failed: ${FAIL}${NC}"
fi
echo ""

if (( FAIL > 0 )); then
  echo -e "${RED}HEALTH CHECK FAILED (${FAIL} failure(s))${NC}"
  exit 1
elif (( WARN > 0 )); then
  echo -e "${YELLOW}HEALTH CHECK PASSED with ${WARN} warning(s)${NC}"
  exit 0
else
  echo -e "${GREEN}ALL CHECKS PASSED${NC}"
  exit 0
fi
