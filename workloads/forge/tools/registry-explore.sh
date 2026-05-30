#!/usr/bin/env bash
# Browse and search the ClawHub skill registry for competitive intelligence
# Usage: registry-explore.sh [QUERY] [--sort=<sort>] [--limit=<n>]
# Sorts: downloads (default), trending, installs
# Saves snapshots to .workbench-cache/explore-<sort>-YYYYMMDD.json
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

CACHE_DIR="$REPO_ROOT/.workbench-cache"
mkdir -p "$CACHE_DIR"

API_BASE="https://clawdhub.com/api/v1"
TODAY=$(date +%Y%m%d)

# Defaults
QUERY=""
SORT="downloads"
LIMIT=20

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --sort=*) SORT="${1#--sort=}" ;;
    --limit=*) LIMIT="${1#--limit=}" ;;
    --*) echo "Unknown option: $1"; exit 1 ;;
    *) QUERY="$1" ;;
  esac
  shift
done

# Collect our skill slugs for highlighting
declare -A OUR_SKILLS
while IFS= read -r dir; do
  slug=$(get_skill_slug "$dir")
  OUR_SKILLS["$slug"]=1
done < <(discover_skills "$REPO_ROOT/skills")

if [[ -n "$QUERY" ]]; then
  log_header "Registry Search: \"$QUERY\""
  echo ""
  ENDPOINT="${API_BASE}/search?q=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$QUERY'))")&limit=${LIMIT}"
  SNAPSHOT_FILE="$CACHE_DIR/explore-search-${TODAY}.json"
else
  log_header "Registry Browse: top ${LIMIT} by ${SORT}"
  echo ""
  ENDPOINT="${API_BASE}/skills?sort=${SORT}&limit=${LIMIT}"
  SNAPSHOT_FILE="$CACHE_DIR/explore-${SORT}-${TODAY}.json"
fi

# Fetch from API
response=$(curl -sf "$ENDPOINT" 2>/dev/null || echo '[]')

# Parse and display results
python3 -c "
import sys, json

data = json.loads('''$( echo "$response" | sed "s/'/\\\\'/g" )''')

# Handle both array and object-with-results responses
if isinstance(data, dict):
    skills = data.get('results', data.get('skills', []))
else:
    skills = data if isinstance(data, list) else []

our_slugs = set('''${!OUR_SKILLS[*]}'''.split())

# Print table
print(f'  {\"Rank\":<5} {\"Skill\":<30} {\"Downloads\":>10} {\"Stars\":>6} {\"Installs\":>10}  Description')
print(f'  {\"-----\":<5} {\"------------------------------\":<30} {\"----------\":>10} {\"------\":>6} {\"----------\":>10}  -----------')

for i, skill in enumerate(skills[:$LIMIT], 1):
    slug = skill.get('slug', skill.get('name', '?'))
    downloads = skill.get('downloads', '-')
    stars = skill.get('stars', '-')
    installs = skill.get('installs', '-')
    desc = skill.get('description', '')[:50]

    marker = ' *' if slug in our_slugs else ''
    # Use ANSI green for ours
    if slug in our_slugs:
        line = f'  \033[32m{i:<5} {slug:<30} {str(downloads):>10} {str(stars):>6} {str(installs):>10}  {desc}{marker}\033[0m'
    else:
        line = f'  \033[2m{i:<5} {slug:<30} {str(downloads):>10} {str(stars):>6} {str(installs):>10}  {desc}\033[0m'
    print(line)

# Save raw JSON
with open('$SNAPSHOT_FILE', 'w') as f:
    json.dump(data, f, indent=2)

print()
total = len(skills[:$LIMIT])
ours = sum(1 for s in skills[:$LIMIT] if s.get('slug', s.get('name', '')) in our_slugs)
print(f'  {total} skills shown, {ours} are ours (marked with *)')
" 2>/dev/null || {
  echo "  (API returned no results or is unavailable)"
}

echo ""
echo "Snapshot saved: $SNAPSHOT_FILE"
echo ""
