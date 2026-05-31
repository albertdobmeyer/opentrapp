#!/usr/bin/env bash
# Shared utilities: colors, logging, skill discovery, frontmatter extraction

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

# Logging
log_pass()  { echo -e "  ${GREEN}PASS${RESET}  $*"; }
log_warn()  { echo -e "  ${YELLOW}WARN${RESET}  $*"; }
log_fail()  { echo -e "  ${RED}FAIL${RESET}  $*"; }
log_info()  { echo -e "  ${BLUE}INFO${RESET}  $*"; }
log_header(){ echo -e "\n${BOLD}$*${RESET}"; }

# Counters
PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

count_pass() { PASS_COUNT=$((PASS_COUNT + 1)); }
count_warn() { WARN_COUNT=$((WARN_COUNT + 1)); }
count_fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); }

print_summary() {
  echo ""
  echo -e "${BOLD}Summary:${RESET} ${GREEN}${PASS_COUNT} passed${RESET}, ${YELLOW}${WARN_COUNT} warnings${RESET}, ${RED}${FAIL_COUNT} failed${RESET}"
  echo ""
}

# Discover all skill directories
# Usage: discover_skills <path>
# If path is a single skill dir (contains SKILL.md), returns just that.
# If path is the skills/ parent, returns all subdirs with SKILL.md.
discover_skills() {
  local path="${1:-.}"

  if [[ -f "$path/SKILL.md" ]]; then
    echo "$path"
    return
  fi

  for dir in "$path"/*/; do
    [[ -f "$dir/SKILL.md" ]] && echo "${dir%/}"
  done
}

# Extract raw frontmatter from SKILL.md (between first pair of --- delimiters only)
# Usage: extract_frontmatter <file>
extract_frontmatter() {
  local file="$1"
  awk 'NR==1 && /^---$/ {found=1; next} found && /^---$/ {exit} found {print}' "$file"
}

# Extract a single frontmatter field value (first match only)
# Usage: get_frontmatter_field <file> <field>
get_frontmatter_field() {
  local file="$1" field="$2"
  extract_frontmatter "$file" | grep -m1 "^${field}:" | sed "s/^${field}:[[:space:]]*//"
}

# Get the skill slug (directory name)
get_skill_slug() {
  basename "$1"
}

# Get the H1 title from the skill
get_skill_title() {
  grep -m1 '^# ' "$1" | sed 's/^# //'
}

# Get line count of file
get_line_count() {
  wc -l < "$1" | tr -d ' '
}

# Count code blocks in file
count_code_blocks() {
  grep -c '^```' "$1" | tr -d ' '
}

# JSON string escaping (shared by scanner, verifier, certifier)
json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

# Resolve repo root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
