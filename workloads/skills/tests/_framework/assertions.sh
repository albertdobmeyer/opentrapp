#!/usr/bin/env bash
# Assertion library for skill behavioral tests

ASSERT_PASS=0
ASSERT_FAIL=0

# Assert a markdown heading exists in file
# Usage: assert_section_exists <file> <heading_text>
assert_section_exists() {
  local file="$1" heading="$2"
  if grep -qE "^#{1,6}\s+${heading}" "$file"; then
    ASSERT_PASS=$((ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: Section '${heading}' not found in $(basename "$file")"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert a regex pattern is found in file
# Usage: assert_contains <file> <pattern>
assert_contains() {
  local file="$1" pattern="$2"
  if grep -qE "$pattern" "$file"; then
    ASSERT_PASS=$((ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: Pattern '${pattern}' not found in $(basename "$file")"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert a regex pattern is NOT found in file
# Usage: assert_not_contains <file> <pattern>
assert_not_contains() {
  local file="$1" pattern="$2"
  if ! grep -qE "$pattern" "$file"; then
    ASSERT_PASS=$((ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: Pattern '${pattern}' unexpectedly found in $(basename "$file")"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert code blocks with given language tag have valid syntax (basic check)
# Usage: assert_code_block_valid <file> <language>
assert_code_block_valid() {
  local file="$1" lang="$2"
  local in_block=false
  local block_content=""
  local block_count=0
  local error_count=0

  while IFS= read -r line; do
    if [[ "$line" =~ ^\`\`\`${lang} ]]; then
      in_block=true
      block_content=""
      continue
    fi
    if [[ "$in_block" == true && "$line" == '```' ]]; then
      in_block=false
      block_count=$((block_count + 1))

      case "$lang" in
        json)
          if command -v python3 &>/dev/null; then
            if ! echo "$block_content" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
              error_count=$((error_count + 1))
            fi
          fi
          ;;
        yaml|yml)
          if command -v python3 &>/dev/null; then
            if ! echo "$block_content" | python3 -c "import sys,yaml; yaml.safe_load(sys.stdin)" 2>/dev/null; then
              error_count=$((error_count + 1))
            fi
          fi
          ;;
      esac
      continue
    fi
    if [[ "$in_block" == true ]]; then
      block_content+="$line"$'\n'
    fi
  done < "$file"

  if (( error_count > 0 )); then
    echo "    FAIL: ${error_count}/${block_count} ${lang} code blocks have syntax errors"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi
  ASSERT_PASS=$((ASSERT_PASS + 1))
  return 0
}

# Assert minimum number of code blocks
# Usage: assert_min_code_blocks <file> <n>
assert_min_code_blocks() {
  local file="$1" min="$2"
  local count
  count=$(grep -c '^```' "$file" || true)
  local pairs=$(( count / 2 ))

  if (( pairs >= min )); then
    ASSERT_PASS=$((ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: Only ${pairs} code blocks (minimum ${min}) in $(basename "$file")"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert file line count within range
# Usage: assert_line_count <file> <min> <max>
assert_line_count() {
  local file="$1" min="$2" max="$3"
  local count
  count=$(wc -l < "$file" | tr -d ' ')

  if (( count >= min && count <= max )); then
    ASSERT_PASS=$((ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: Line count ${count} outside range [${min}, ${max}] in $(basename "$file")"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert a frontmatter field matches a pattern
# Usage: assert_frontmatter_field <file> <field> <pattern>
assert_frontmatter_field() {
  local file="$1" field="$2" pattern="$3"
  local value
  value=$(sed -n '/^---$/,/^---$/{ /^---$/d; p; }' "$file" | grep "^${field}:" | sed "s/^${field}:[[:space:]]*//")

  if [[ -z "$value" ]]; then
    echo "    FAIL: Frontmatter field '${field}' not found in $(basename "$file")"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi

  if echo "$value" | grep -qE "$pattern"; then
    ASSERT_PASS=$((ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: Field '${field}' value doesn't match pattern '${pattern}'"
    ASSERT_FAIL=$((ASSERT_FAIL + 1))
    return 1
  fi
}
