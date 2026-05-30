#!/usr/bin/env bash
# OpenAgent Social: Engagement Control — Level Presets
#
# Manages which engagement level the user operates at.
# Presets configure rate limits, scanning, and identity requirements.
#
# Usage:
#   bash scripts/engagement-control.sh --level observer --dry-run
#   bash scripts/engagement-control.sh --level researcher --apply
#   bash scripts/engagement-control.sh --status
#
# Levels:
#   observer    — Level 1: read-only, no API key, no interaction
#   researcher  — Level 2: registered identity, controlled interaction
#   participant — Level 3: full interaction with rate limits

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

ENV_FILE="$PROJECT_ROOT/config/.env"
PRESETS_DIR="$PROJECT_ROOT/config"

VALID_LEVELS=("observer" "researcher" "participant")

# Colors
if [[ -t 1 ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  BLUE='\033[0;34m'
  CYAN='\033[0;36m'
  BOLD='\033[1m'
  DIM='\033[2m'
  NC='\033[0m'
else
  RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' DIM='' NC=''
fi

# ── Helpers ──────────────────────────────────────────────

is_valid_level() {
  local level="$1"
  for valid in "${VALID_LEVELS[@]}"; do
    if [[ "$level" == "$valid" ]]; then
      return 0
    fi
  done
  return 1
}

# Read a value from an env file (first match wins)
read_env_value() {
  local file="$1" key="$2" default="${3:-}"
  if [[ -f "$file" ]]; then
    local val
    val=$(grep -E "^${key}=" "$file" 2>/dev/null | head -1 | cut -d= -f2-)
    if [[ -n "$val" ]]; then
      echo "$val"
      return
    fi
  fi
  echo "$default"
}

# Detect current engagement level from .env
detect_level() {
  if [[ ! -f "$ENV_FILE" ]]; then
    echo "unknown"
    return
  fi
  local level
  level=$(read_env_value "$ENV_FILE" "ENGAGEMENT_LEVEL" "")
  if [[ -n "$level" ]] && is_valid_level "$level"; then
    echo "$level"
  else
    echo "unknown"
  fi
}

level_number() {
  case "$1" in
    observer)    echo "1" ;;
    researcher)  echo "2" ;;
    participant) echo "3" ;;
    *)           echo "?" ;;
  esac
}

level_description() {
  case "$1" in
    observer)    echo "Read-only observation. No API key, no interaction." ;;
    researcher)  echo "Registered identity with controlled interaction." ;;
    participant) echo "Full interaction with rate limits and retraction plan." ;;
    *)           echo "Unknown level" ;;
  esac
}

# ── Status ───────────────────────────────────────────────

show_status() {
  echo ""
  echo -e "${BOLD}OpenAgent Social: Engagement Status${NC}"
  echo "===================================="
  echo ""

  if [[ ! -f "$ENV_FILE" ]]; then
    echo -e "  ${YELLOW}No config found.${NC} Run 'make setup' first."
    echo ""
    return
  fi

  # shellcheck disable=SC1090
  source "$ENV_FILE"

  local level
  level=$(detect_level)
  local level_num
  level_num=$(level_number "$level")

  if [[ "$level" == "unknown" ]]; then
    echo -e "  Level:            ${YELLOW}unknown${NC} (ENGAGEMENT_LEVEL not set — defaulting to observer)"
  else
    echo -e "  Level:            ${CYAN}${level}${NC} (Level ${level_num})"
  fi

  echo -e "  Description:      $(level_description "$level")"
  echo ""

  # Config summary
  echo -e "${BOLD}Configuration${NC}"
  local api_key="${MOLTBOOK_API_KEY:-}"
  if [[ -n "$api_key" ]]; then
    echo -e "  API key:          ${GREEN}set${NC} (${#api_key} chars)"
  else
    echo -e "  API key:          ${DIM}not set${NC}"
  fi

  local handle="${AGENT_HANDLE:-}"
  if [[ -n "$handle" ]]; then
    echo -e "  Agent handle:     @${handle}"
  else
    echo -e "  Agent handle:     ${DIM}not set${NC}"
  fi

  local posts="${RATE_LIMIT_POSTS_PER_HOUR:-0}"
  local comments="${RATE_LIMIT_COMMENTS_PER_HOUR:-0}"
  local votes="${RATE_LIMIT_VOTES_PER_HOUR:-0}"
  echo -e "  Rate limits:      ${posts} posts/hr, ${comments} comments/hr, ${votes} votes/hr"

  local scan="${FEED_SCAN_ENABLED:-false}"
  if [[ "$scan" == "true" ]]; then
    echo -e "  Feed scanning:    ${GREEN}enabled${NC}"
  else
    echo -e "  Feed scanning:    ${DIM}disabled${NC}"
  fi

  echo ""

  # Mismatch warnings
  local warnings=0

  if [[ "$level" == "observer" || "$level" == "unknown" ]]; then
    if [[ -n "$api_key" ]]; then
      echo -e "  ${YELLOW}WARN${NC}  API key is set but level is observer (read-only)"
      warnings=$((warnings + 1))
    fi
    if [[ "$posts" != "0" || "$comments" != "0" || "$votes" != "0" ]]; then
      echo -e "  ${YELLOW}WARN${NC}  Rate limits > 0 but level is observer (should be 0)"
      warnings=$((warnings + 1))
    fi
  fi

  if [[ "$level" == "researcher" || "$level" == "participant" ]]; then
    if [[ -z "$api_key" ]]; then
      echo -e "  ${YELLOW}WARN${NC}  Level ${level} requires an API key (MOLTBOOK_API_KEY not set)"
      warnings=$((warnings + 1))
    fi
    if [[ -z "$handle" ]]; then
      echo -e "  ${YELLOW}WARN${NC}  Level ${level} requires an agent handle (AGENT_HANDLE not set)"
      warnings=$((warnings + 1))
    fi
    if [[ "$scan" != "true" ]]; then
      echo -e "  ${YELLOW}WARN${NC}  Feed scanning should be enabled at level ${level}"
      warnings=$((warnings + 1))
    fi
  fi

  if (( warnings > 0 )); then
    echo ""
  fi

  echo "===================================="
  echo ""
}

# ── Dry Run ──────────────────────────────────────────────

do_dry_run() {
  local level="$1"
  local preset_file="$PRESETS_DIR/${level}.env"

  echo ""
  echo -e "${BOLD}OpenAgent Social: Engagement Control — Dry Run${NC}"
  echo "================================================"
  echo ""

  if [[ ! -f "$preset_file" ]]; then
    echo -e "${RED}ERROR: Preset file not found: ${preset_file}${NC}"
    exit 1
  fi

  local current_level
  current_level=$(detect_level)

  echo -e "  Current level:  ${current_level} (Level $(level_number "$current_level"))"
  echo -e "  Target level:   ${CYAN}${level}${NC} (Level $(level_number "$level"))"
  echo ""

  echo -e "${BOLD}$(level_description "$level")${NC}"
  echo ""

  # Show what the config would look like
  echo -e "${BOLD}Preset Configuration${NC}"
  while IFS= read -r line; do
    # Skip comments and empty lines
    [[ "$line" =~ ^#.*$ || -z "$line" ]] && continue
    local key val
    key="${line%%=*}"
    val="${line#*=}"

    # Highlight user-specific fields that will be preserved
    if [[ "$key" == "MOLTBOOK_API_KEY" || "$key" == "AGENT_HANDLE" ]]; then
      local current_val
      current_val=$(read_env_value "$ENV_FILE" "$key" "")
      if [[ -n "$current_val" ]]; then
        echo -e "  ${key}=${GREEN}(preserved from current config)${NC}"
      else
        echo -e "  ${key}=${DIM}(empty — fill in before use)${NC}"
      fi
    else
      echo "  ${key}=${val}"
    fi
  done < "$preset_file"

  echo ""

  # Show level-specific guidelines
  echo -e "${BOLD}Guidelines for Level $(level_number "$level")${NC}"
  case "$level" in
    observer)
      echo "  - Use only unauthenticated API endpoints"
      echo "  - No posts, comments, or votes"
      echo "  - Store data locally for offline review"
      ;;
    researcher)
      echo "  - Use a DEDICATED API key (not your primary)"
      echo "  - Scan all feed content before processing"
      echo "  - Run identity-checklist.sh before registering"
      echo "  - Review feed-allowlist.yml for trusted agents"
      ;;
    participant)
      echo "  - Complete all Level 2 setup first"
      echo "  - Document your retraction plan"
      echo "  - Monitor rate limit compliance"
      echo "  - Have a kill switch ready (API key revocation)"
      ;;
  esac

  echo ""
  echo "================================================"
  echo -e "  ${YELLOW}This is a dry run — no changes applied.${NC}"
  echo "  To apply: use --apply instead of --dry-run"
  echo ""
}

# ── Apply ────────────────────────────────────────────────

do_apply() {
  local level="$1"
  local preset_file="$PRESETS_DIR/${level}.env"

  echo ""
  echo -e "${BOLD}OpenAgent Social: Engagement Control — Apply${NC}"
  echo "=============================================="
  echo ""

  if [[ ! -f "$preset_file" ]]; then
    echo -e "${RED}ERROR: Preset file not found: ${preset_file}${NC}"
    exit 1
  fi

  local current_level
  current_level=$(detect_level)

  echo -e "  Current level:  ${current_level} (Level $(level_number "$current_level"))"
  echo -e "  Target level:   ${CYAN}${level}${NC} (Level $(level_number "$level"))"
  echo ""

  # Step 1: Capture user-specific values from current config
  local saved_api_key="" saved_handle="" saved_api_base=""
  if [[ -f "$ENV_FILE" ]]; then
    saved_api_key=$(read_env_value "$ENV_FILE" "MOLTBOOK_API_KEY" "")
    saved_handle=$(read_env_value "$ENV_FILE" "AGENT_HANDLE" "")
    saved_api_base=$(read_env_value "$ENV_FILE" "MOLTBOOK_API_BASE" "https://api.moltbook.com")
  fi

  # Step 2: Copy preset to .env
  cp "$preset_file" "$ENV_FILE"
  echo -e "  ${GREEN}PASS${NC}  Preset written to config/.env"

  # Step 3: Re-inject user-specific values
  if [[ -n "$saved_api_key" ]]; then
    sed -i "s|^MOLTBOOK_API_KEY=.*|MOLTBOOK_API_KEY=${saved_api_key}|" "$ENV_FILE"
    echo -e "  ${GREEN}PASS${NC}  API key preserved"
  fi
  if [[ -n "$saved_handle" ]]; then
    sed -i "s|^AGENT_HANDLE=.*|AGENT_HANDLE=${saved_handle}|" "$ENV_FILE"
    echo -e "  ${GREEN}PASS${NC}  Agent handle preserved"
  fi
  if [[ "$saved_api_base" != "https://api.moltbook.com" && -n "$saved_api_base" ]]; then
    sed -i "s|^MOLTBOOK_API_BASE=.*|MOLTBOOK_API_BASE=${saved_api_base}|" "$ENV_FILE"
    echo -e "  ${GREEN}PASS${NC}  Custom API base preserved"
  fi

  # Step 4: Verify the result
  echo ""
  echo -e "${BOLD}Verification${NC}"

  # Source the new config and check
  # shellcheck disable=SC1090
  source "$ENV_FILE"

  local checks_pass=0 checks_warn=0 checks_fail=0

  # Level is set correctly
  if [[ "${ENGAGEMENT_LEVEL:-}" == "$level" ]]; then
    echo -e "  ${GREEN}PASS${NC}  ENGAGEMENT_LEVEL = ${level}"
    checks_pass=$((checks_pass + 1))
  else
    echo -e "  ${RED}FAIL${NC}  ENGAGEMENT_LEVEL expected ${level}, got ${ENGAGEMENT_LEVEL:-empty}"
    checks_fail=$((checks_fail + 1))
  fi

  # Level-specific checks
  case "$level" in
    observer)
      if [[ "${RATE_LIMIT_POSTS_PER_HOUR:-0}" == "0" ]]; then
        echo -e "  ${GREEN}PASS${NC}  Rate limits = 0 (read-only)"
        checks_pass=$((checks_pass + 1))
      else
        echo -e "  ${RED}FAIL${NC}  Observer should have rate limits = 0"
        checks_fail=$((checks_fail + 1))
      fi
      if [[ "${FEED_SCAN_ENABLED:-false}" == "false" ]]; then
        echo -e "  ${GREEN}PASS${NC}  Feed scanning disabled (not needed for observation)"
        checks_pass=$((checks_pass + 1))
      else
        echo -e "  ${YELLOW}WARN${NC}  Feed scanning enabled but not required for observer"
        checks_warn=$((checks_warn + 1))
      fi
      ;;
    researcher)
      if (( ${RATE_LIMIT_POSTS_PER_HOUR:-0} > 0 && ${RATE_LIMIT_POSTS_PER_HOUR:-0} <= 20 )); then
        echo -e "  ${GREEN}PASS${NC}  Post rate limit: ${RATE_LIMIT_POSTS_PER_HOUR}/hr (within bounds)"
        checks_pass=$((checks_pass + 1))
      else
        echo -e "  ${RED}FAIL${NC}  Post rate limit out of range for researcher (expected 1-20)"
        checks_fail=$((checks_fail + 1))
      fi
      if [[ "${FEED_SCAN_ENABLED:-false}" == "true" ]]; then
        echo -e "  ${GREEN}PASS${NC}  Feed scanning enabled"
        checks_pass=$((checks_pass + 1))
      else
        echo -e "  ${RED}FAIL${NC}  Feed scanning must be enabled at researcher level"
        checks_fail=$((checks_fail + 1))
      fi
      ;;
    participant)
      if (( ${RATE_LIMIT_POSTS_PER_HOUR:-0} > 0 && ${RATE_LIMIT_POSTS_PER_HOUR:-0} <= 50 )); then
        echo -e "  ${GREEN}PASS${NC}  Post rate limit: ${RATE_LIMIT_POSTS_PER_HOUR}/hr (within bounds)"
        checks_pass=$((checks_pass + 1))
      else
        echo -e "  ${RED}FAIL${NC}  Post rate limit out of range for participant (expected 1-50)"
        checks_fail=$((checks_fail + 1))
      fi
      if [[ "${FEED_SCAN_ENABLED:-false}" == "true" ]]; then
        echo -e "  ${GREEN}PASS${NC}  Feed scanning enabled"
        checks_pass=$((checks_pass + 1))
      else
        echo -e "  ${RED}FAIL${NC}  Feed scanning must be enabled at participant level"
        checks_fail=$((checks_fail + 1))
      fi
      ;;
  esac

  # Warnings for user-specific fields
  if [[ "$level" != "observer" && -z "${MOLTBOOK_API_KEY:-}" ]]; then
    echo -e "  ${YELLOW}WARN${NC}  API key not set — required for ${level} level"
    checks_warn=$((checks_warn + 1))
  fi
  if [[ "$level" != "observer" && -z "${AGENT_HANDLE:-}" ]]; then
    echo -e "  ${YELLOW}WARN${NC}  Agent handle not set — required for ${level} level"
    checks_warn=$((checks_warn + 1))
  fi

  echo ""
  echo -e "${BOLD}Summary${NC}"
  echo -e "  ${GREEN}Passed: ${checks_pass}${NC}"
  if (( checks_warn > 0 )); then
    echo -e "  ${YELLOW}Warnings: ${checks_warn}${NC}"
  fi
  if (( checks_fail > 0 )); then
    echo -e "  ${RED}Failed: ${checks_fail}${NC}"
  fi
  echo ""

  if (( checks_fail > 0 )); then
    echo -e "${RED}Engagement level set with errors. Review above.${NC}"
    exit 1
  elif (( checks_warn > 0 )); then
    echo -e "${GREEN}Engagement level set to ${BOLD}${level}${NC}${GREEN} (Level $(level_number "$level")).${NC}"
    echo -e "${YELLOW}Fix warnings above before using this level.${NC}"
  else
    echo -e "${GREEN}Engagement level set to ${BOLD}${level}${NC}${GREEN} (Level $(level_number "$level")).${NC}"
  fi
  echo ""
}

# ── Parse Arguments ──────────────────────────────────────

MODE=""
LEVEL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --level)
      LEVEL="${2:?--level requires a value (observer, researcher, participant)}"
      if ! is_valid_level "$LEVEL"; then
        echo -e "${RED}ERROR: Invalid level '${LEVEL}'. Valid: observer, researcher, participant${NC}" >&2
        exit 1
      fi
      shift 2
      ;;
    --dry-run)
      MODE="dry-run"
      shift
      ;;
    --apply)
      MODE="apply"
      shift
      ;;
    --status)
      MODE="status"
      shift
      ;;
    -h|--help)
      echo "OpenAgent Social: Engagement Control"
      echo ""
      echo "Usage:"
      echo "  $0 --status                              Show current engagement level"
      echo "  $0 --level <name> --dry-run               Preview a level preset"
      echo "  $0 --level <name> --apply                 Apply a level preset"
      echo ""
      echo "Levels:"
      echo "  observer      Level 1: read-only, no API key"
      echo "  researcher    Level 2: registered identity, controlled interaction"
      echo "  participant   Level 3: full interaction with rate limits"
      exit 0
      ;;
    *)
      echo "Unknown argument: $1. Use --help for usage." >&2
      exit 1
      ;;
  esac
done

# ── Execute ──────────────────────────────────────────────

case "$MODE" in
  status)
    show_status
    ;;
  dry-run)
    if [[ -z "$LEVEL" ]]; then
      echo "ERROR: --level required for dry-run." >&2
      exit 1
    fi
    do_dry_run "$LEVEL"
    ;;
  apply)
    if [[ -z "$LEVEL" ]]; then
      echo "ERROR: --level required for apply." >&2
      exit 1
    fi
    do_apply "$LEVEL"
    ;;
  *)
    echo "ERROR: Specify --status, --dry-run, or --apply. Use --help for usage." >&2
    exit 1
    ;;
esac
