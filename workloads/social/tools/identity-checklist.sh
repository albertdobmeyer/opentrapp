#!/usr/bin/env bash
# Identity checklist — pre-flight safety check before registering a Moltbook agent
# Usage: identity-checklist.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Config ────────────────────────────────────────────────
ENV_FILE="$PROJECT_ROOT/config/.env"
PATTERNS_FILE="$PROJECT_ROOT/config/injection-patterns.yml"
ALLOWLIST_FILE="$PROJECT_ROOT/config/feed-allowlist.yml"

# Colors
if [[ -t 1 ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  BLUE='\033[0;34m'
  CYAN='\033[0;36m'
  BOLD='\033[1m'
  DIM='\033[2m'
  RESET='\033[0m'
else
  RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' DIM='' RESET=''
fi

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

check_pass() { echo -e "  ${GREEN}PASS${RESET}  $*"; PASS_COUNT=$((PASS_COUNT + 1)); }
check_warn() { echo -e "  ${YELLOW}WARN${RESET}  $*"; WARN_COUNT=$((WARN_COUNT + 1)); }
check_fail() { echo -e "  ${RED}FAIL${RESET}  $*"; FAIL_COUNT=$((FAIL_COUNT + 1)); }
check_info() { echo -e "  ${BLUE}INFO${RESET}  $*"; }

# ── Main ──────────────────────────────────────────────────
echo -e "${BOLD}Moltbook Identity Pre-Flight Checklist${RESET}"
echo -e "${DIM}Run this before registering an agent identity on Moltbook${RESET}"
echo ""

# ── 1. Configuration ─────────────────────────────────────
echo -e "${BOLD}1. Configuration${RESET}"

if [[ -f "$ENV_FILE" ]]; then
  check_pass ".env file exists"
  # shellcheck disable=SC1090
  source "$ENV_FILE"
else
  check_fail ".env file not found — copy config/.env.example to config/.env"
fi

if [[ -n "${MOLTBOOK_API_BASE:-}" ]]; then
  check_pass "API base URL configured: ${MOLTBOOK_API_BASE}"
else
  check_fail "MOLTBOOK_API_BASE not set"
fi

echo ""

# ── 2. API Key Safety ────────────────────────────────────
echo -e "${BOLD}2. API Key Safety${RESET}"

if [[ -n "${MOLTBOOK_API_KEY:-}" ]]; then
  check_pass "API key is set"

  # Check it's not a common placeholder
  case "${MOLTBOOK_API_KEY}" in
    "your-key-here"|"TODO"|"CHANGEME"|"xxx"*)
      check_fail "API key looks like a placeholder — set a real dedicated key"
      ;;
    *)
      check_pass "API key is not a placeholder"
      ;;
  esac
else
  check_info "No API key set (read-only mode — fine for Level 1 Observer)"
fi

# Check .env is gitignored
if [[ -f "$PROJECT_ROOT/.gitignore" ]] && grep -q '\.env$\|\.env\b' "$PROJECT_ROOT/.gitignore" 2>/dev/null; then
  check_pass ".env is gitignored"
elif [[ -f "$PROJECT_ROOT/../.gitignore" ]] && grep -q '\.env$\|\.env\b' "$PROJECT_ROOT/../.gitignore" 2>/dev/null; then
  check_pass ".env is gitignored (parent repo)"
else
  check_warn ".env may not be gitignored — verify before committing"
fi

echo ""

# ── 3. Rate Limits ───────────────────────────────────────
echo -e "${BOLD}3. Rate Limits${RESET}"

POSTS_LIMIT="${RATE_LIMIT_POSTS_PER_HOUR:-0}"
COMMENTS_LIMIT="${RATE_LIMIT_COMMENTS_PER_HOUR:-0}"
VOTES_LIMIT="${RATE_LIMIT_VOTES_PER_HOUR:-0}"

if (( POSTS_LIMIT > 0 && POSTS_LIMIT <= 20 )); then
  check_pass "Post rate limit: ${POSTS_LIMIT}/hour"
elif (( POSTS_LIMIT == 0 )); then
  check_info "Post rate limit: 0 (read-only mode)"
else
  check_warn "Post rate limit is high (${POSTS_LIMIT}/hour) — consider lowering"
fi

if (( COMMENTS_LIMIT > 0 && COMMENTS_LIMIT <= 50 )); then
  check_pass "Comment rate limit: ${COMMENTS_LIMIT}/hour"
elif (( COMMENTS_LIMIT == 0 )); then
  check_info "Comment rate limit: 0 (read-only mode)"
else
  check_warn "Comment rate limit is high (${COMMENTS_LIMIT}/hour) — consider lowering"
fi

if (( VOTES_LIMIT > 0 && VOTES_LIMIT <= 100 )); then
  check_pass "Vote rate limit: ${VOTES_LIMIT}/hour"
elif (( VOTES_LIMIT == 0 )); then
  check_info "Vote rate limit: 0 (read-only mode)"
else
  check_warn "Vote rate limit is high (${VOTES_LIMIT}/hour) — consider lowering"
fi

echo ""

# ── 4. Feed Scanner ──────────────────────────────────────
echo -e "${BOLD}4. Feed Scanner${RESET}"

if [[ -f "$PATTERNS_FILE" ]]; then
  pattern_count=$(grep -c '^\s*- id:' "$PATTERNS_FILE" 2>/dev/null || echo "0")
  check_pass "Injection patterns loaded: ${pattern_count} patterns"
else
  check_fail "Injection patterns file missing: ${PATTERNS_FILE}"
fi

if [[ -f "$ALLOWLIST_FILE" ]]; then
  check_pass "Feed allowlist file exists"
else
  check_warn "Feed allowlist file missing — all agents will be scanned"
fi

FEED_SCAN="${FEED_SCAN_ENABLED:-true}"
if [[ "$FEED_SCAN" == "true" ]]; then
  check_pass "Feed scanning enabled"
else
  check_warn "Feed scanning is DISABLED — enable FEED_SCAN_ENABLED in .env"
fi

echo ""

# ── 5. Agent Identity ────────────────────────────────────
echo -e "${BOLD}5. Agent Identity${RESET}"

if [[ -n "${AGENT_HANDLE:-}" ]]; then
  check_pass "Agent handle configured: @${AGENT_HANDLE}"
else
  check_info "No agent handle set (set AGENT_HANDLE in .env before registering)"
fi

echo ""

# ── 6. Retraction Plan ───────────────────────────────────
echo -e "${BOLD}6. Retraction Plan${RESET}"

check_info "Before registering, document answers to:"
echo -e "    ${DIM}1. Can you delete posts via the API?${RESET}"
echo -e "    ${DIM}2. How quickly can you revoke your API key?${RESET}"
echo -e "    ${DIM}3. What's your escalation path if something goes wrong?${RESET}"
echo -e "    ${DIM}4. Have you tested the kill switch (key revocation)?${RESET}"

echo ""

# ── 7. Tool Availability ─────────────────────────────────
echo -e "${BOLD}7. Tool Availability${RESET}"

for tool in curl python3 jq; do
  if command -v "$tool" &>/dev/null; then
    check_pass "$tool available"
  else
    if [[ "$tool" == "jq" ]]; then
      check_warn "$tool not found (optional, used for JSON formatting)"
    else
      check_fail "$tool not found (required)"
    fi
  fi
done

echo ""

# ── Summary ───────────────────────────────────────────────
echo -e "${BOLD}Pre-Flight Summary${RESET}"
echo -e "  ${GREEN}Passed: ${PASS_COUNT}${RESET}"
echo -e "  ${YELLOW}Warnings: ${WARN_COUNT}${RESET}"
echo -e "  ${RED}Failed: ${FAIL_COUNT}${RESET}"
echo ""

if (( FAIL_COUNT > 0 )); then
  echo -e "${RED}BLOCKED: Fix ${FAIL_COUNT} failing check(s) before registering.${RESET}"
  exit 1
elif (( WARN_COUNT > 0 )); then
  echo -e "${YELLOW}READY with warnings. Review warnings before proceeding.${RESET}"
  exit 0
else
  echo -e "${GREEN}ALL CLEAR. Ready to register agent identity.${RESET}"
  exit 0
fi
