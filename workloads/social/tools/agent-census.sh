#!/usr/bin/env bash
# Agent census — pull public stats from Moltbook API and save snapshots
# Usage:
#   agent-census.sh                 Pull current stats
#   agent-census.sh --trend         Show trend from saved snapshots
#   agent-census.sh --file <path>   Load census data from a local JSON file
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Config ────────────────────────────────────────────────
ENV_FILE="$PROJECT_ROOT/config/.env"
DATA_DIR="$PROJECT_ROOT/data"

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

# Load config
MOLTBOOK_API_BASE="${MOLTBOOK_API_BASE:-https://api.moltbook.com}"
if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$ENV_FILE"
fi

mkdir -p "$DATA_DIR"

# ── Parse Args ────────────────────────────────────────────
MODE="census"
FILE_PATH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --trend)
      MODE="trend"
      shift
      ;;
    --file)
      MODE="file"
      FILE_PATH="${2:?--file requires a path}"
      shift 2
      ;;
    -h|--help)
      echo "Usage: agent-census.sh [--trend] [--file <path>]"
      echo ""
      echo "Pull platform statistics from Moltbook API."
      echo ""
      echo "Options:"
      echo "  --trend          Show trend from saved census snapshots"
      echo "  --file <path>    Load census data from a local JSON file"
      echo "  -h, --help       Show this help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# ── Census Mode ───────────────────────────────────────────
run_census() {
  echo -e "${BOLD}Moltbook Agent Census${RESET}"
  echo -e "${DIM}API: ${MOLTBOOK_API_BASE}${RESET}"
  echo ""

  local timestamp
  timestamp=$(date +%Y%m%d-%H%M%S)
  local snapshot_file="$DATA_DIR/census-${timestamp}.json"

  # Fetch platform stats
  echo -e "Fetching platform statistics..."

  # Try to get stats from various endpoints
  local agents_count="-"
  local posts_count="-"
  local comments_count="-"

  # Attempt: stats/overview endpoint
  local stats_response
  if stats_response=$(curl -sf "${MOLTBOOK_API_BASE}/stats" 2>/dev/null); then
    agents_count=$(echo "$stats_response" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(d.get('agents', d.get('total_agents', d.get('agent_count', '-'))))
" 2>/dev/null || echo "-")
    posts_count=$(echo "$stats_response" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(d.get('posts', d.get('total_posts', d.get('post_count', '-'))))
" 2>/dev/null || echo "-")
    comments_count=$(echo "$stats_response" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(d.get('comments', d.get('total_comments', d.get('comment_count', '-'))))
" 2>/dev/null || echo "-")
  fi

  # Fetch recent posts for activity analysis
  local recent_posts_count=0
  local unique_agents=0
  local recent_response

  if recent_response=$(curl -sf "${MOLTBOOK_API_BASE}/posts?limit=50" 2>/dev/null); then
    read -r recent_posts_count unique_agents <<< "$(echo "$recent_response" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    posts = data if isinstance(data, list) else data.get('posts', data.get('data', []))
    handles = set()
    for p in posts:
        h = p.get('agent_handle', p.get('handle', ''))
        if h:
            handles.add(h)
    print(len(posts), len(handles))
except:
    print(0, 0)
" 2>/dev/null || echo "0 0")"
  fi

  # Fetch trending / top posts
  local top_posts=""
  local top_response
  if top_response=$(curl -sf "${MOLTBOOK_API_BASE}/posts?limit=10&sort=votes" 2>/dev/null); then
    top_posts=$(echo "$top_response" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    posts = data if isinstance(data, list) else data.get('posts', data.get('data', []))
    for p in posts[:10]:
        handle = p.get('agent_handle', p.get('handle', 'unknown'))
        votes = p.get('upvotes', p.get('votes', 0))
        content = p.get('content', '')[:60].replace('\n', ' ')
        print(f'  {handle:<20} {votes:>8} votes  {content}')
except:
    pass
" 2>/dev/null || echo "")
  fi

  # Display results
  echo ""
  echo -e "${BOLD}Platform Overview${RESET}"
  echo -e "  Registered agents:    ${CYAN}${agents_count}${RESET}"
  echo -e "  Total posts:          ${CYAN}${posts_count}${RESET}"
  echo -e "  Total comments:       ${CYAN}${comments_count}${RESET}"
  echo ""
  echo -e "${BOLD}Recent Activity (last 50 posts)${RESET}"
  echo -e "  Posts fetched:        ${recent_posts_count}"
  echo -e "  Unique agents:        ${unique_agents}"
  echo ""

  if [[ -n "$top_posts" ]]; then
    echo -e "${BOLD}Top Posts by Votes${RESET}"
    echo -e "${DIM}  (Vote counts are unreliable — race condition in voting API)${RESET}"
    echo "$top_posts"
    echo ""
  fi

  # Save snapshot
  python3 -c "
import json
from datetime import datetime
snapshot = {
    'timestamp': '$timestamp',
    'date': datetime.now().isoformat(),
    'platform': {
        'agents': '$agents_count',
        'posts': '$posts_count',
        'comments': '$comments_count'
    },
    'recent_activity': {
        'posts_sampled': $recent_posts_count,
        'unique_agents': $unique_agents
    }
}
with open('$snapshot_file', 'w') as f:
    json.dump(snapshot, f, indent=2)
" 2>/dev/null

  echo -e "Snapshot saved: ${snapshot_file}"
  echo ""
  echo -e "${DIM}Run with --trend to compare snapshots over time${RESET}"
}

# ── File Mode ─────────────────────────────────────────────
run_census_from_file() {
  echo -e "${BOLD}Moltbook Agent Census (offline)${RESET}" >&2
  echo -e "${DIM}Source: ${FILE_PATH}${RESET}" >&2
  echo "" >&2

  if [[ ! -f "$FILE_PATH" ]]; then
    echo -e "${RED}ERROR${RESET}: File not found: $FILE_PATH" >&2
    exit 1
  fi

  local timestamp
  timestamp=$(date +%Y%m%d-%H%M%S)
  local snapshot_file="$DATA_DIR/census-${timestamp}.json"

  # Parse stats and posts from the file
  local agents_count posts_count comments_count
  read -r agents_count posts_count comments_count <<< "$(python3 -c "
import sys, json
with open('$FILE_PATH') as f:
    data = json.load(f)
stats = data.get('stats', {})
print(stats.get('agents', '-'), stats.get('posts', '-'), stats.get('comments', '-'))
" 2>/dev/null || echo "- - -")"

  local recent_posts_count=0
  local unique_agents=0
  read -r recent_posts_count unique_agents <<< "$(python3 -c "
import sys, json
with open('$FILE_PATH') as f:
    data = json.load(f)
posts = data.get('posts', [])
handles = set()
for p in posts:
    h = p.get('agent_handle', p.get('handle', ''))
    if h:
        handles.add(h)
print(len(posts), len(handles))
" 2>/dev/null || echo "0 0")"

  local top_posts=""
  top_posts=$(python3 -c "
import json
with open('$FILE_PATH') as f:
    data = json.load(f)
posts = sorted(data.get('posts', []), key=lambda p: p.get('upvotes', p.get('votes', 0)), reverse=True)
for p in posts[:10]:
    handle = p.get('agent_handle', p.get('handle', 'unknown'))
    votes = p.get('upvotes', p.get('votes', 0))
    content = p.get('content', '')[:60].replace('\n', ' ')
    print(f'  {handle:<20} {votes:>8} votes  {content}')
" 2>/dev/null || echo "")

  # Display results (same layout as live mode)
  echo ""
  echo -e "${BOLD}Platform Overview${RESET}"
  echo -e "  Registered agents:    ${CYAN}${agents_count}${RESET}"
  echo -e "  Total posts:          ${CYAN}${posts_count}${RESET}"
  echo -e "  Total comments:       ${CYAN}${comments_count}${RESET}"
  echo ""
  echo -e "${BOLD}Recent Activity${RESET}"
  echo -e "  Posts loaded:         ${recent_posts_count}"
  echo -e "  Unique agents:        ${unique_agents}"
  echo ""

  if [[ -n "$top_posts" ]]; then
    echo -e "${BOLD}Top Posts by Votes${RESET}"
    echo -e "${DIM}  (Vote counts are unreliable — race condition in voting API)${RESET}"
    echo "$top_posts"
    echo ""
  fi

  # Save snapshot
  python3 -c "
import json
from datetime import datetime
snapshot = {
    'timestamp': '$timestamp',
    'date': datetime.now().isoformat(),
    'platform': {
        'agents': '$agents_count',
        'posts': '$posts_count',
        'comments': '$comments_count'
    },
    'recent_activity': {
        'posts_sampled': $recent_posts_count,
        'unique_agents': $unique_agents
    }
}
with open('$snapshot_file', 'w') as f:
    json.dump(snapshot, f, indent=2)
" 2>/dev/null

  echo -e "Snapshot saved: ${snapshot_file}"
}

# ── Trend Mode ────────────────────────────────────────────
show_trend() {
  echo -e "${BOLD}Moltbook Census Trend${RESET}"
  echo ""

  local snapshots
  snapshots=$(find "$DATA_DIR" -name "census-*.json" -type f 2>/dev/null | sort)

  if [[ -z "$snapshots" ]]; then
    echo "No census snapshots found. Run agent-census.sh first."
    exit 0
  fi

  local count
  count=$(echo "$snapshots" | wc -l | tr -d ' ')
  echo -e "Found ${count} snapshot(s):"
  echo ""

  printf "  %-20s %15s %12s %12s %10s\n" "Date" "Agents" "Posts" "Comments" "Active"
  printf "  %-20s %15s %12s %12s %10s\n" "--------------------" "---------------" "------------" "------------" "----------"

  while IFS= read -r snapshot_file; do
    python3 -c "
import json
with open('$snapshot_file') as f:
    s = json.load(f)
date = s.get('date', s.get('timestamp', 'unknown'))[:19]
p = s.get('platform', {})
r = s.get('recent_activity', {})
agents = p.get('agents', '-')
posts = p.get('posts', '-')
comments = p.get('comments', '-')
active = r.get('unique_agents', '-')
print(f'  {date:<20} {agents:>15} {posts:>12} {comments:>12} {str(active):>10}')
" 2>/dev/null || echo "  (error reading snapshot)"
  done <<< "$snapshots"

  echo ""
}

# ── Main ──────────────────────────────────────────────────
case "$MODE" in
  census) run_census ;;
  file)   run_census_from_file ;;
  trend)  show_trend ;;
esac
