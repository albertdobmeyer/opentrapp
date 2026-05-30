#!/usr/bin/env bash
# Feed scanner — detect prompt injection patterns in Moltbook feed content
# Usage:
#   feed-scanner.sh --recent <n>          Scan n most recent posts
#   feed-scanner.sh --agent <handle>      Scan posts by a specific agent
#   feed-scanner.sh --file <path>         Scan a local JSON file of posts
#   feed-scanner.sh --verbose             Show matched content lines
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Config ────────────────────────────────────────────────
ENV_FILE="$PROJECT_ROOT/config/.env"
PATTERNS_FILE="$PROJECT_ROOT/config/injection-patterns.yml"
ALLOWLIST_FILE="$PROJECT_ROOT/config/feed-allowlist.yml"
DATA_DIR="$PROJECT_ROOT/data"

# Colors (disabled if not a terminal)
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

# ── Load Config ───────────────────────────────────────────
MOLTBOOK_API_BASE="${MOLTBOOK_API_BASE:-https://api.moltbook.com}"
MOLTBOOK_API_KEY="${MOLTBOOK_API_KEY:-}"

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$ENV_FILE"
fi

# ── Parse Args ────────────────────────────────────────────
MODE=""
TARGET=""
VERBOSE=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --recent)
      MODE="recent"
      TARGET="${2:-50}"
      shift 2
      ;;
    --agent)
      MODE="agent"
      TARGET="${2:?--agent requires a handle}"
      shift 2
      ;;
    --file)
      MODE="file"
      TARGET="${2:?--file requires a path}"
      shift 2
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    -h|--help)
      echo "Usage: feed-scanner.sh [--recent <n>] [--agent <handle>] [--file <path>] [--verbose]"
      echo ""
      echo "Scan Moltbook feed content for prompt injection patterns."
      echo ""
      echo "Options:"
      echo "  --recent <n>       Scan n most recent posts (default: 50)"
      echo "  --agent <handle>   Scan posts by a specific agent"
      echo "  --file <path>      Scan a local JSON file of posts"
      echo "  --verbose          Show matched content for each finding"
      echo "  -h, --help         Show this help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Default to recent 50
if [[ -z "$MODE" ]]; then
  MODE="recent"
  TARGET="50"
fi

# ── Load Patterns ─────────────────────────────────────────
# Parse injection-patterns.yml into arrays
# Format: SEVERITY<TAB>CATEGORY<TAB>REGEX<TAB>DESCRIPTION
PATTERNS=()

load_patterns() {
  if [[ ! -f "$PATTERNS_FILE" ]]; then
    echo -e "${RED}ERROR${RESET}: Patterns file not found: $PATTERNS_FILE"
    exit 1
  fi

  local severity="" category="" regex="" description="" id=""

  while IFS= read -r line; do
    # Strip leading whitespace for matching
    local trimmed
    trimmed=$(echo "$line" | sed 's/^[[:space:]]*//')

    case "$trimmed" in
      "- id:"*)
        # Save previous pattern if complete
        if [[ -n "$severity" && -n "$regex" ]]; then
          PATTERNS+=("${severity}	${category}	${regex}	${description}")
        fi
        id="${trimmed#*: }"
        severity="" category="" regex="" description=""
        ;;
      "severity:"*)
        severity="${trimmed#*: }"
        ;;
      "category:"*)
        category="${trimmed#*: }"
        ;;
      "regex:"*)
        # Extract regex — everything after "regex: " with surrounding quotes stripped
        regex="${trimmed#*: }"
        regex="${regex#\"}"
        regex="${regex%\"}"
        # Strip (?i) flag — grep -Ei already handles case insensitivity
        regex="${regex#\(\?i\)}"
        ;;
      "description:"*)
        description="${trimmed#*: }"
        description="${description#\"}"
        description="${description%\"}"
        ;;
    esac
  done < "$PATTERNS_FILE"

  # Don't forget the last pattern
  if [[ -n "$severity" && -n "$regex" ]]; then
    PATTERNS+=("${severity}	${category}	${regex}	${description}")
  fi

  if (( ${#PATTERNS[@]} == 0 )); then
    echo -e "${RED}ERROR${RESET}: No patterns loaded from $PATTERNS_FILE"
    exit 1
  fi

  echo -e "${DIM}Loaded ${#PATTERNS[@]} patterns${RESET}"
}

# ── Load Allowlist ────────────────────────────────────────
TRUSTED_AGENTS=()
SAFE_PATTERNS=()

load_allowlist() {
  if [[ ! -f "$ALLOWLIST_FILE" ]]; then
    return
  fi

  local in_section=""
  while IFS= read -r line; do
    local trimmed
    trimmed=$(echo "$line" | sed 's/^[[:space:]]*//')
    # Track which YAML section we're in
    if [[ "$trimmed" == "trusted_agents:"* ]]; then
      in_section="trusted_agents"
      continue
    elif [[ "$trimmed" == "safe_patterns:"* ]]; then
      in_section="safe_patterns"
      continue
    elif [[ "$trimmed" == "skip_categories:"* ]]; then
      in_section=""
      continue
    fi

    case "$in_section" in
      trusted_agents)
        if [[ "$trimmed" == "- handle:"* ]]; then
          local handle="${trimmed#*: }"
          handle="${handle#\"}"
          handle="${handle%\"}"
          TRUSTED_AGENTS+=("$handle")
        fi
        ;;
      safe_patterns)
        if [[ "$trimmed" == "- pattern:"* ]]; then
          local pattern="${trimmed#*: }"
          pattern="${pattern#\"}"
          pattern="${pattern%\"}"
          # Strip (?i) flag — grep -Ei already handles case insensitivity
          pattern="${pattern#\(\?i\)}"
          SAFE_PATTERNS+=("$pattern")
        fi
        ;;
    esac
  done < "$ALLOWLIST_FILE"
}

is_trusted_agent() {
  local handle="$1"
  for trusted in "${TRUSTED_AGENTS[@]}"; do
    if [[ "$handle" == "$trusted" ]]; then
      return 0
    fi
  done
  return 1
}

is_safe_content() {
  local content="$1"
  for safe_regex in "${SAFE_PATTERNS[@]}"; do
    if echo "$content" | grep -qEi "$safe_regex" 2>/dev/null; then
      return 0
    fi
  done
  return 1
}

# ── Fetch Posts ───────────────────────────────────────────
fetch_posts() {
  local tmp_file
  tmp_file=$(mktemp)

  case "$MODE" in
    recent)
      echo -e "${BOLD}Fetching ${TARGET} recent posts...${RESET}" >&2
      local curl_args=(-sf "${MOLTBOOK_API_BASE}/posts?limit=${TARGET}")
      if [[ -n "$MOLTBOOK_API_KEY" ]]; then
        curl_args+=(-H "Authorization: Bearer ${MOLTBOOK_API_KEY}")
      fi
      if ! curl "${curl_args[@]}" > "$tmp_file" 2>/dev/null; then
        echo -e "${RED}ERROR${RESET}: Failed to fetch posts from API" >&2
        echo "Check MOLTBOOK_API_BASE in config/.env" >&2
        rm -f "$tmp_file"
        exit 1
      fi
      ;;
    agent)
      echo -e "${BOLD}Fetching posts by @${TARGET}...${RESET}" >&2
      if ! curl -sf "${MOLTBOOK_API_BASE}/agents/${TARGET}/posts" > "$tmp_file" 2>/dev/null; then
        echo -e "${RED}ERROR${RESET}: Failed to fetch posts for agent @${TARGET}" >&2
        rm -f "$tmp_file"
        exit 1
      fi
      ;;
    file)
      echo -e "${BOLD}Scanning file: ${TARGET}${RESET}" >&2
      if [[ ! -f "$TARGET" ]]; then
        echo -e "${RED}ERROR${RESET}: File not found: $TARGET" >&2
        exit 1
      fi
      cp "$TARGET" "$tmp_file"
      ;;
  esac

  echo "$tmp_file"
}

# ── Scan Content ──────────────────────────────────────────
scan_content() {
  local posts_file="$1"
  local critical_count=0
  local high_count=0
  local medium_count=0
  local clean_count=0
  local total_posts=0
  local skipped_trusted=0

  # Extract posts as individual content blocks
  # Expected JSON format: array of objects with "content" and "agent_handle" fields
  local post_count
  post_count=$(python3 -c "
import sys, json
try:
    data = json.load(open('$posts_file'))
    if isinstance(data, list):
        print(len(data))
    elif isinstance(data, dict) and 'posts' in data:
        print(len(data['posts']))
    elif isinstance(data, dict) and 'data' in data:
        print(len(data['data']))
    else:
        print(0)
except:
    print(0)
" 2>/dev/null || echo "0")

  if [[ "$post_count" == "0" ]]; then
    echo -e "${YELLOW}No posts found to scan${RESET}"
    echo "If scanning from API, the response format may have changed."
    return
  fi

  echo -e "Scanning ${post_count} posts...\n"

  # Process each post
  for (( i=0; i<post_count; i++ )); do
    local content handle
    content=$(python3 -c "
import sys, json
data = json.load(open('$posts_file'))
posts = data if isinstance(data, list) else data.get('posts', data.get('data', []))
if $i < len(posts):
    print(posts[$i].get('content', ''))
" 2>/dev/null || echo "")

    handle=$(python3 -c "
import sys, json
data = json.load(open('$posts_file'))
posts = data if isinstance(data, list) else data.get('posts', data.get('data', []))
if $i < len(posts):
    print(posts[$i].get('agent_handle', posts[$i].get('handle', 'unknown')))
" 2>/dev/null || echo "unknown")

    total_posts=$((total_posts + 1))

    # Check trusted agent allowlist
    if [[ ${#TRUSTED_AGENTS[@]} -gt 0 ]] && is_trusted_agent "$handle"; then
      skipped_trusted=$((skipped_trusted + 1))
      continue
    fi

    # Check safe content patterns (suppress findings for benign content)
    if [[ ${#SAFE_PATTERNS[@]} -gt 0 ]] && is_safe_content "$content"; then
      clean_count=$((clean_count + 1))
      continue
    fi

    # Scan against each pattern
    local post_findings=0
    for pattern_def in "${PATTERNS[@]}"; do
      IFS=$'\t' read -r severity category regex description <<< "$pattern_def"

      if echo "$content" | grep -qEi "$regex" 2>/dev/null; then
        if (( post_findings == 0 )); then
          echo -e "${CYAN}--- Post #$((i+1)) by @${handle} ---${RESET}"
        fi
        post_findings=$((post_findings + 1))

        case "$severity" in
          CRITICAL)
            echo -e "  ${RED}CRITICAL${RESET} [${category}]: ${description}"
            critical_count=$((critical_count + 1))
            ;;
          HIGH)
            echo -e "  ${YELLOW}HIGH${RESET}     [${category}]: ${description}"
            high_count=$((high_count + 1))
            ;;
          *)
            echo -e "  ${BLUE}MEDIUM${RESET}   [${category}]: ${description}"
            medium_count=$((medium_count + 1))
            ;;
        esac

        if $VERBOSE; then
          local match
          match=$(echo "$content" | grep -oEi "$regex" 2>/dev/null | head -1 | head -c 100)
          echo -e "           ${DIM}${match}${RESET}"
        fi
      fi
    done

    if (( post_findings == 0 )); then
      clean_count=$((clean_count + 1))
    fi
  done

  # ── Summary ───────────────────────────────────────────
  echo ""
  echo -e "${BOLD}Scan Results:${RESET}"
  echo -e "  Posts scanned: ${total_posts}"
  if (( skipped_trusted > 0 )); then
    echo -e "  Skipped (trusted): ${skipped_trusted}"
  fi
  echo -e "  ${RED}Critical findings: ${critical_count}${RESET}"
  echo -e "  ${YELLOW}High findings: ${high_count}${RESET}"
  echo -e "  ${BLUE}Medium findings: ${medium_count}${RESET}"
  echo -e "  ${GREEN}Clean posts: ${clean_count}${RESET}"
  echo ""

  # Save results
  mkdir -p "$DATA_DIR"
  local timestamp
  timestamp=$(date +%Y%m%d-%H%M%S)
  local results_file="$DATA_DIR/scan-${timestamp}.json"

  python3 -c "
import json
results = {
    'timestamp': '$timestamp',
    'mode': '$MODE',
    'target': '$TARGET',
    'total_posts': $total_posts,
    'skipped_trusted': $skipped_trusted,
    'critical': $critical_count,
    'high': $high_count,
    'medium': $medium_count,
    'clean': $clean_count
}
with open('$results_file', 'w') as f:
    json.dump(results, f, indent=2)
" 2>/dev/null && echo -e "Results saved: ${results_file}" || true

  if (( critical_count > 0 )); then
    echo -e "${RED}WARNING: ${critical_count} critical finding(s). Review before processing this feed content.${RESET}"
    exit 1
  fi
}

# ── Main ──────────────────────────────────────────────────
echo -e "${BOLD}Moltbook Feed Scanner${RESET}"
echo ""

load_patterns
load_allowlist

posts_file=$(fetch_posts)
scan_content "$posts_file"

rm -f "$posts_file"
