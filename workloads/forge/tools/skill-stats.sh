#!/usr/bin/env bash
# Fetch adoption metrics from ClawHub API for published skills
# Usage: skill-stats.sh [--trend] [--rank] [--json]
# Modes:
#   (default)  Current stats table
#   --trend    Compare to previous snapshots, show deltas
#   --rank     Our skills ranked against registry top 50
#   --json     Output raw JSON
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

CACHE_DIR="$REPO_ROOT/.workbench-cache"
mkdir -p "$CACHE_DIR"

API_BASE="https://clawdhub.com/api/v1"
TODAY=$(date +%Y%m%d)
SNAPSHOT_FILE="$CACHE_DIR/stats-${TODAY}.json"

# Parse mode
MODE="default"
JSON_OUTPUT=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --trend) MODE="trend" ;;
    --rank) MODE="rank" ;;
    --json) JSON_OUTPUT=true ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
  shift
done

# Collect our skill slugs
skills=()
while IFS= read -r dir; do
  skills+=("$(get_skill_slug "$dir")")
done < <(discover_skills "$REPO_ROOT/skills")

# Fetch current stats for all our skills
fetch_our_stats() {
  local results="[]"
  for slug in "${skills[@]}"; do
    response=$(curl -sf "${API_BASE}/skills/${slug}" 2>/dev/null || echo '{}')
    downloads=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('downloads','-'))" 2>/dev/null || echo "-")
    stars=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('stars','-'))" 2>/dev/null || echo "-")
    installs=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('installs','-'))" 2>/dev/null || echo "-")
    results=$(echo "$results" | python3 -c "
import sys, json
arr = json.load(sys.stdin)
arr.append({'slug': '$slug', 'downloads': '$downloads', 'stars': '$stars', 'installs': '$installs'})
print(json.dumps(arr))
" 2>/dev/null || echo "$results")
  done
  echo "$results"
}

# Find previous snapshot files for trend comparison
find_previous_snapshot() {
  local days_ago="${1:-7}"
  local target_date
  # Try each day going back
  for i in $(seq 1 "$days_ago"); do
    target_date=$(date -d "-${i} days" +%Y%m%d 2>/dev/null || date -v-${i}d +%Y%m%d 2>/dev/null || echo "")
    [[ -z "$target_date" ]] && continue
    local file="$CACHE_DIR/stats-${target_date}.json"
    if [[ -f "$file" ]]; then
      echo "$file"
      return
    fi
  done
  echo ""
}

# === DEFAULT MODE ===
mode_default() {
  log_header "ClawHub Skill Stats"
  echo "Fetching stats for $((${#skills[@]} - 1)) skills..."
  echo ""

  local results
  results=$(fetch_our_stats)

  # Save snapshot
  echo "$results" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin), indent=2))" > "$SNAPSHOT_FILE" 2>/dev/null || true

  if $JSON_OUTPUT; then
    cat "$SNAPSHOT_FILE"
    return
  fi

  # Print table
  printf "  %-24s %10s %6s %10s\n" "Skill" "Downloads" "Stars" "Installs"
  printf "  %-24s %10s %6s %10s\n" "------------------------" "----------" "------" "----------"

  local total_downloads=0 total_stars=0 total_installs=0
  local count=0

  echo "$results" | python3 -c "
import sys, json
data = json.load(sys.stdin)
total_dl = 0
total_st = 0
total_in = 0
count = 0
for s in data:
    dl = s['downloads']
    st = s['stars']
    ins = s['installs']
    print(f\"  {s['slug']:<24} {dl:>10} {st:>6} {ins:>10}\")
    try: total_dl += int(dl)
    except: pass
    try: total_st += int(st)
    except: pass
    try: total_in += int(ins)
    except: pass
    count += 1
print(f'  {\"========================\":<24} {\"==========\":>10} {\"======\":>6} {\"==========\":>10}')
print(f'  {\"TOTAL (\" + str(count) + \" skills)\":<24} {str(total_dl):>10} {str(total_st):>6} {str(total_in):>10}')
" 2>/dev/null

  echo ""
  echo "Snapshot saved: $SNAPSHOT_FILE"
  echo ""
}

# === TREND MODE ===
mode_trend() {
  log_header "ClawHub Stats — Trend Analysis"

  # Fetch current
  echo "Fetching current stats..."
  local results
  results=$(fetch_our_stats)

  # Save snapshot
  echo "$results" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin), indent=2))" > "$SNAPSHOT_FILE" 2>/dev/null || true

  # Find previous snapshots
  local prev_7d prev_30d
  prev_7d=$(find_previous_snapshot 7)
  prev_30d=$(find_previous_snapshot 30)

  if [[ -z "$prev_7d" && -z "$prev_30d" ]]; then
    echo ""
    echo -e "  ${YELLOW}No previous snapshots found for comparison.${RESET}"
    echo "  Run 'make stats' periodically to build history."
    echo "  Showing current stats instead."
    echo ""
    mode_default
    return
  fi

  echo ""

  python3 -c "
import json, sys

current = json.loads('''$(cat "$SNAPSHOT_FILE")''')
prev_7d_file = '$prev_7d'
prev_30d_file = '$prev_30d'

prev_7d = json.load(open(prev_7d_file)) if prev_7d_file else []
prev_30d = json.load(open(prev_30d_file)) if prev_30d_file else []

# Index by slug
def index_by_slug(data):
    return {s['slug']: s for s in data}

curr_idx = index_by_slug(current)
p7_idx = index_by_slug(prev_7d)
p30_idx = index_by_slug(prev_30d)

def delta(curr_val, prev_val):
    try:
        c = int(curr_val)
        p = int(prev_val)
        d = c - p
        if d > 0: return f'\033[32m↑ +{d}\033[0m'
        elif d < 0: return f'\033[31m↓ {d}\033[0m'
        else: return '  ='
    except:
        return '  ?'

header_7d = '  Δ 7d' if prev_7d_file else ''
header_30d = '  Δ 30d' if prev_30d_file else ''

print(f'  {\"Skill\":<24} {\"Downloads\":>10}{header_7d:>12}{header_30d:>12}')
print(f'  {\"------------------------\":<24} {\"----------\":>10}{\"------------\":>12 if prev_7d_file else \"\"}{\"------------\":>12 if prev_30d_file else \"\"}')

for s in current:
    slug = s['slug']
    dl = s['downloads']
    line = f'  {slug:<24} {dl:>10}'
    if prev_7d_file:
        p = p7_idx.get(slug, {}).get('downloads', '-')
        line += f'{delta(dl, p):>20}'
    if prev_30d_file:
        p = p30_idx.get(slug, {}).get('downloads', '-')
        line += f'{delta(dl, p):>20}'
    print(line)

# Summary
total_dl = sum(int(s['downloads']) for s in current if s['downloads'] not in ['-', ''])
growing = 0
if prev_7d_file:
    for s in current:
        try:
            c = int(s['downloads'])
            p = int(p7_idx.get(s['slug'], {}).get('downloads', 0))
            if c > p: growing += 1
        except: pass

print()
if prev_7d_file:
    prev_file_date = prev_7d_file.split('stats-')[1].split('.')[0]
    print(f'  Compared against: {prev_file_date} (7d window)')
if prev_30d_file:
    prev_file_date = prev_30d_file.split('stats-')[1].split('.')[0]
    print(f'  Compared against: {prev_file_date} (30d window)')
print(f'  Total downloads: {total_dl}')
if prev_7d_file:
    print(f'  Skills growing (7d): {growing}/{len(current)}')
" 2>/dev/null || echo "  (Error computing trends)"

  echo ""
}

# === RANK MODE ===
mode_rank() {
  log_header "ClawHub Stats — Registry Ranking"

  echo "Fetching our stats + registry top 50..."
  echo ""

  # Fetch our stats
  local our_results
  our_results=$(fetch_our_stats)
  echo "$our_results" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin), indent=2))" > "$SNAPSHOT_FILE" 2>/dev/null || true

  # Fetch registry top 50
  local registry_top
  registry_top=$(curl -sf "${API_BASE}/skills?sort=downloads&limit=50" 2>/dev/null || echo '[]')

  python3 -c "
import json, sys

our_data = json.loads('''$(cat "$SNAPSHOT_FILE")''')
registry_raw = json.loads('''$(echo "$registry_top" | sed "s/'/\\\\'/g")''')

# Handle both array and object responses
if isinstance(registry_raw, dict):
    registry = registry_raw.get('results', registry_raw.get('skills', []))
else:
    registry = registry_raw if isinstance(registry_raw, list) else []

our_slugs = {s['slug'] for s in our_data}

# Merge: add our skills that aren't already in registry top 50
registry_slugs = {s.get('slug', s.get('name', '')) for s in registry}
merged = list(registry)
for s in our_data:
    if s['slug'] not in registry_slugs:
        merged.append({'slug': s['slug'], 'downloads': s['downloads'], 'stars': s['stars'], 'installs': s['installs']})

# Sort by downloads
def sort_key(s):
    try: return int(s.get('downloads', 0))
    except: return 0
merged.sort(key=sort_key, reverse=True)

print(f'  {\"Rank\":<6}{\"Skill\":<30} {\"Downloads\":>10} {\"Stars\":>6} {\"Installs\":>10}')
print(f'  {\"------\":<6}{\"------------------------------\":<30} {\"----------\":>10} {\"------\":>6} {\"----------\":>10}')

in_top_50 = 0
for i, s in enumerate(merged[:50], 1):
    slug = s.get('slug', s.get('name', '?'))
    dl = s.get('downloads', '-')
    st = s.get('stars', '-')
    ins = s.get('installs', '-')
    is_ours = slug in our_slugs
    if is_ours:
        in_top_50 += 1
        print(f'  \033[32m{i:<6}{slug:<30} {str(dl):>10} {str(st):>6} {str(ins):>10}  ← ours\033[0m')
    else:
        print(f'  \033[2m{i:<6}{slug:<30} {str(dl):>10} {str(st):>6} {str(ins):>10}\033[0m')

total_dl = sum(int(s['downloads']) for s in our_data if s['downloads'] not in ['-', ''])
print()
print(f'  Your {len(our_data)} skills: {total_dl} total downloads, {in_top_50} in top 50')
" 2>/dev/null || echo "  (Error fetching registry data)"

  echo ""
}

# Dispatch
case "$MODE" in
  default) mode_default ;;
  trend) mode_trend ;;
  rank) mode_rank ;;
esac
