#!/usr/bin/env bash
# Skill linter — validates frontmatter, structure, content quality, metadata consistency
# Usage: skill-lint.sh <path>  (path = skills/ dir or single skill dir)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/frontmatter.sh"

TARGET="${1:-skills}"

log_header "Linting skills in: $TARGET"

skills=()
while IFS= read -r dir; do
  skills+=("$dir")
done < <(discover_skills "$TARGET")

if (( ${#skills[@]} == 0 )); then
  echo "No skills found in $TARGET"
  exit 1
fi

for skill_dir in "${skills[@]}"; do
  file="$skill_dir/SKILL.md"
  slug=$(get_skill_slug "$skill_dir")
  echo -e "\n${CYAN}--- $slug ---${RESET}"

  # === Frontmatter checks ===
  validate_frontmatter "$file" || true

  # Check name matches directory
  fm_name=$(get_frontmatter_field "$file" "name")
  if [[ -n "$fm_name" && "$fm_name" != "$slug" ]]; then
    log_warn "$slug: Frontmatter name '$fm_name' doesn't match directory name"
    count_warn
  fi

  # Check version field
  fm_version=$(get_frontmatter_field "$file" "version" || true)
  if [[ -z "$fm_version" ]]; then
    log_warn "$slug: Missing 'version' field in frontmatter"
    count_warn
  elif ! echo "$fm_version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    log_warn "$slug: Invalid version format '$fm_version' (expected X.Y.Z)"
    count_warn
  fi

  # === Structure checks ===
  # H1 title after frontmatter
  if ! grep -q '^# ' "$file"; then
    log_fail "$slug: No H1 title found"
    count_fail
  else
    log_pass "$slug: H1 title present"
    count_pass
  fi

  # "When to Use" section
  if grep -q '^## When to Use' "$file"; then
    log_pass "$slug: 'When to Use' section present"
    count_pass
  else
    log_fail "$slug: Missing '## When to Use' section"
    count_fail
  fi

  # "Tips" section
  if grep -q '^## Tips' "$file"; then
    log_pass "$slug: 'Tips' section present"
    count_pass
  else
    log_warn "$slug: Missing '## Tips' section"
    count_warn
  fi

  # === Content quality checks ===
  line_count=$(get_line_count "$file")

  if (( line_count < 150 )); then
    log_warn "$slug: Short content (${line_count} lines, min 150)"
    count_warn
  elif (( line_count > 700 )); then
    log_warn "$slug: Long content (${line_count} lines, max 700 recommended)"
    count_warn
  else
    log_pass "$slug: Content length OK (${line_count} lines)"
    count_pass
  fi

  # Code block density
  code_blocks=$(count_code_blocks "$file")
  # Divide by 2 for opening+closing pairs
  block_pairs=$(( code_blocks / 2 ))
  if (( block_pairs < 8 )); then
    log_warn "$slug: Low code block density (${block_pairs} blocks, min 8)"
    count_warn
  else
    log_pass "$slug: Code block density OK (${block_pairs} blocks)"
    count_pass
  fi

  # Language tags on code fences
  untagged=$(grep -cE '^\`\`\`$' "$file" || true)
  if (( untagged > 0 )); then
    log_warn "$slug: ${untagged} code fences without language tags"
    count_warn
  else
    log_pass "$slug: All code fences have language tags"
    count_pass
  fi

  # TODO/FIXME/XXX placeholders (only in prose, not inside code fences)
  placeholders=$(awk '/^```/{c=!c;next} !c{print}' "$file" | grep -ciE '\b(TODO|FIXME|XXX)\b' || true)
  if (( placeholders > 0 )); then
    log_fail "$slug: Found ${placeholders} TODO/FIXME/XXX placeholders"
    count_fail
  else
    log_pass "$slug: No placeholders"
    count_pass
  fi

  # === Metadata consistency ===
  # Check that each binary in requires.anyBins is referenced in content
  meta=$(get_frontmatter_field "$file" "metadata")
  if [[ -n "$meta" ]] && command -v python3 &>/dev/null; then
    bins=$(echo "$meta" | python3 -c "
import sys, json
try:
    m = json.load(sys.stdin)
    cb = m.get('clawdbot', {})
    req = cb.get('requires', {})
    bins = req.get('anyBins', req.get('bins', []))
    if isinstance(bins, list):
        for b in bins:
            print(b)
except: pass
" 2>/dev/null || true)

    if [[ -n "$bins" ]]; then
      # Get content after frontmatter
      content=$(sed -n '/^---$/,/^---$/d; p' "$file" | sed '1{/^$/d}')
      while IFS= read -r bin; do
        if ! echo "$content" | grep -qi "\b${bin}\b"; then
          log_warn "$slug: Binary '${bin}' in metadata but not referenced in content"
          count_warn
        fi
      done <<< "$bins"
    fi
  fi

done

print_summary

if (( FAIL_COUNT > 0 )); then
  exit 1
fi
