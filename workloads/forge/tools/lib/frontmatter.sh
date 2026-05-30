#!/usr/bin/env bash
# YAML frontmatter parser and validator for SKILL.md files

# Source common if not already loaded
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[[ -z "${RESET:-}" ]] && source "$SCRIPT_DIR/common.sh"

# Check that frontmatter delimiters exist (--- on first and closing line)
validate_frontmatter_delimiters() {
  local file="$1"
  local first_line
  first_line=$(head -1 "$file")

  if [[ "$first_line" != "---" ]]; then
    log_fail "$file: Missing opening frontmatter delimiter (---)"
    count_fail
    return 1
  fi

  # Check for closing delimiter (second ---)
  local delimiter_count
  delimiter_count=$(grep -c '^---$' "$file")
  if (( delimiter_count < 2 )); then
    log_fail "$file: Missing closing frontmatter delimiter (---)"
    count_fail
    return 1
  fi

  return 0
}

# Validate required frontmatter fields: name, description, metadata
validate_frontmatter_fields() {
  local file="$1"
  local fm
  fm=$(extract_frontmatter "$file")
  local errors=0

  # Check name field
  local name
  name=$(echo "$fm" | grep '^name:' | sed 's/^name:[[:space:]]*//')
  if [[ -z "$name" ]]; then
    log_fail "$file: Missing 'name' field in frontmatter"
    count_fail
    errors=$((errors + 1))
  fi

  # Check description field
  local desc
  desc=$(echo "$fm" | grep '^description:' | sed 's/^description:[[:space:]]*//')
  if [[ -z "$desc" ]]; then
    log_fail "$file: Missing 'description' field in frontmatter"
    count_fail
    errors=$((errors + 1))
  fi

  # Check metadata field
  local meta
  meta=$(echo "$fm" | grep '^metadata:' | sed 's/^metadata:[[:space:]]*//')
  if [[ -z "$meta" ]]; then
    log_fail "$file: Missing 'metadata' field in frontmatter"
    count_fail
    errors=$((errors + 1))
  fi

  return $errors
}

# Validate name is a valid slug (lowercase, hyphens, no spaces)
validate_name_slug() {
  local file="$1"
  local name
  name=$(get_frontmatter_field "$file" "name")

  if [[ -z "$name" ]]; then
    return 1  # Already caught by field check
  fi

  if ! echo "$name" | grep -qE '^[a-z0-9]([a-z0-9-]*[a-z0-9])?$'; then
    log_fail "$file: Name '$name' is not a valid slug (use lowercase, hyphens only)"
    count_fail
    return 1
  fi
  return 0
}

# Validate description length (50-200 chars)
validate_description_length() {
  local file="$1"
  local desc
  desc=$(get_frontmatter_field "$file" "description")

  if [[ -z "$desc" ]]; then
    return 1
  fi

  local len=${#desc}
  if (( len < 50 )); then
    log_warn "$file: Description too short (${len} chars, min 50)"
    count_warn
    return 1
  fi
  if (( len > 500 )); then
    log_warn "$file: Description very long (${len} chars, recommended max 200)"
    count_warn
    return 1
  fi
  return 0
}

# Validate metadata is valid JSON
validate_metadata_json() {
  local file="$1"
  local meta
  meta=$(get_frontmatter_field "$file" "metadata")

  if [[ -z "$meta" ]]; then
    return 1
  fi

  if command -v python3 &>/dev/null; then
    if ! echo "$meta" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
      log_fail "$file: metadata field is not valid JSON"
      count_fail
      return 1
    fi
  fi
  return 0
}

# Run all frontmatter validations on a file
validate_frontmatter() {
  local file="$1"
  local errors=0

  validate_frontmatter_delimiters "$file" || errors=$((errors + 1))
  if (( errors > 0 )); then
    return $errors  # Can't parse fields without delimiters
  fi

  validate_frontmatter_fields "$file" || errors=$((errors + 1))
  validate_name_slug "$file" || errors=$((errors + 1))
  validate_description_length "$file" || true  # Warnings don't block
  validate_metadata_json "$file" || errors=$((errors + 1))

  return $errors
}
