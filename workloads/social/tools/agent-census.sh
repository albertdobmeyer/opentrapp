#!/usr/bin/env bash
# Agent census — pull platform stats and post snapshots via a pluggable adapter.
# Protocol-agnostic: all platform I/O is delegated to the adapter.
#
# Usage:
#   agent-census.sh [--adapter <name>]             Pull current stats
#   agent-census.sh [--adapter <name>] --trend     Show trend from saved snapshots
#   agent-census.sh --file <path>                  Load census data from a local JSON file
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ADAPTERS_DIR="$SCRIPT_DIR/lib/adapters"

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

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$ENV_FILE"
fi

mkdir -p "$DATA_DIR"

# ── Parse Args ────────────────────────────────────────────
ADAPTER="mock"   # sensible default; live adapters replace this
MODE="census"
FILE_PATH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --adapter)
      ADAPTER="${2:?--adapter requires a name (file, mock, moltbook)}"
      shift 2
      ;;
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
      echo "Usage: agent-census.sh [--adapter <name>] [--trend] [--file <path>]"
      echo ""
      echo "Pull platform statistics via the configured adapter."
      echo ""
      echo "Options:"
      echo "  --adapter <name>  Protocol adapter: mock (default), file, moltbook"
      echo "  --trend           Show trend from saved census snapshots"
      echo "  --file <path>     Load census data from a local JSON file (implies --adapter file)"
      echo "  -h, --help        Show this help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# --file implies adapter=file
if [[ "$MODE" == "file" ]]; then
  ADAPTER="file"
fi

# ── Resolve adapter ───────────────────────────────────────
ADAPTER_SCRIPT="$ADAPTERS_DIR/${ADAPTER}.sh"
if [[ ! -f "$ADAPTER_SCRIPT" ]]; then
  echo -e "${RED}ERROR${RESET}: Unknown adapter '${ADAPTER}'. Available: file, mock, moltbook" >&2
  exit 1
fi

# ── Census Mode ───────────────────────────────────────────
run_census() {
  local adapter_name
  adapter_name=$(bash "$ADAPTER_SCRIPT" name 2>/dev/null || echo "$ADAPTER")
  echo -e "${BOLD}Agent Census${RESET}"
  echo -e "${DIM}Adapter: ${adapter_name}${RESET}"
  echo ""

  local timestamp
  timestamp=$(date +%Y%m%d-%H%M%S)
  local snapshot_file="$DATA_DIR/census-${timestamp}.json"

  # Get platform stats from adapter
  echo -e "Fetching platform statistics..."
  local stats_json
  stats_json=$(bash "$ADAPTER_SCRIPT" stats '{}' 2>/dev/null || echo '{"agents":null,"posts":null,"comments":null}')

  local agents_count posts_count comments_count
  read -r agents_count posts_count comments_count <<< "$(python3 -c "
import json, sys
d = json.loads(r'''${stats_json}''')
def fmt(v): return str(v) if v is not None else '-'
print(fmt(d.get('agents')), fmt(d.get('posts')), fmt(d.get('comments')))
" 2>/dev/null || echo '- - -')"

  # Get recent posts from adapter for activity analysis
  local opts
  opts=$(python3 -c "import json; print(json.dumps({'limit': 50}))")
  local recent_json
  recent_json=$(bash "$ADAPTER_SCRIPT" fetch_feed "$opts" 2>/dev/null || echo '[]')

  local recent_posts_count=0
  local unique_agents=0
  read -r recent_posts_count unique_agents <<< "$(python3 -c "
import json
posts = json.loads(r'''${recent_json}''')
authors = set(p.get('author', '') for p in posts if p.get('author'))
print(len(posts), len(authors))
" 2>/dev/null || echo '0 0')"

  # Top posts by weight (mock/file have no votes — show first 10)
  local top_posts=""
  top_posts=$(python3 -c "
import json
posts = json.loads(r'''${recent_json}''')
# Sort by votes if present, otherwise preserve order
posts_sorted = sorted(posts, key=lambda p: p.get('votes', p.get('upvotes', 0)), reverse=True)
for p in posts_sorted[:10]:
    author  = p.get('author', 'unknown')
    votes   = p.get('votes', p.get('upvotes', 0))
    content = p.get('content', '')[:60].replace('\n', ' ')
    print(f'  {author:<20} {votes:>8} votes  {content}')
" 2>/dev/null || echo "")

  # Display
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
    'adapter': '$adapter_name',
    'platform': {
        'agents':   '$agents_count',
        'posts':    '$posts_count',
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
  echo -e "${BOLD}Agent Census (offline)${RESET}" >&2
  echo -e "${DIM}Source: ${FILE_PATH}${RESET}" >&2
  echo "" >&2

  if [[ ! -f "$FILE_PATH" ]]; then
    echo -e "${RED}ERROR${RESET}: File not found: $FILE_PATH" >&2
    exit 1
  fi

  local timestamp
  timestamp=$(date +%Y%m%d-%H%M%S)
  local snapshot_file="$DATA_DIR/census-${timestamp}.json"

  # Delegate to file adapter
  local opts
  opts=$(python3 -c "import json; print(json.dumps({'path': '${FILE_PATH}'}))")

  local stats_json
  stats_json=$(bash "$ADAPTER_SCRIPT" stats "$opts" 2>/dev/null || echo '{"agents":null,"posts":null,"comments":null}')

  local agents_count posts_count comments_count
  read -r agents_count posts_count comments_count <<< "$(python3 -c "
import json
d = json.loads(r'''${stats_json}''')
def fmt(v): return str(v) if v is not None else '-'
print(fmt(d.get('agents')), fmt(d.get('posts')), fmt(d.get('comments')))
" 2>/dev/null || echo '- - -')"

  local posts_json
  posts_json=$(bash "$ADAPTER_SCRIPT" fetch_feed "$opts" 2>/dev/null || echo '[]')

  local recent_posts_count=0
  local unique_agents=0
  read -r recent_posts_count unique_agents <<< "$(python3 -c "
import json
posts = json.loads(r'''${posts_json}''')
authors = set(p.get('author','') for p in posts if p.get('author'))
print(len(posts), len(authors))
" 2>/dev/null || echo '0 0')"

  local top_posts=""
  top_posts=$(python3 -c "
import json
posts = json.loads(r'''${posts_json}''')
posts_sorted = sorted(posts, key=lambda p: p.get('votes', p.get('upvotes', 0)), reverse=True)
for p in posts_sorted[:10]:
    author  = p.get('author', 'unknown')
    votes   = p.get('votes', p.get('upvotes', 0))
    content = p.get('content', '')[:60].replace('\n', ' ')
    print(f'  {author:<20} {votes:>8} votes  {content}')
" 2>/dev/null || echo "")

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
    echo "$top_posts"
    echo ""
  fi

  python3 -c "
import json
from datetime import datetime
snapshot = {
    'timestamp': '$timestamp',
    'date': datetime.now().isoformat(),
    'adapter': 'file',
    'platform': {
        'agents':   '$agents_count',
        'posts':    '$posts_count',
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
  echo -e "${BOLD}Census Trend${RESET}"
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
