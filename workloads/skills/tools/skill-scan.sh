#!/usr/bin/env bash
# Offline security scanner - pattern-based detection without network
# Usage: skill-scan.sh [--format=default|summary|json|sarif] <path>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/patterns.sh"

# ── Parse arguments ──
FORMAT="default"
STRICT=false
TARGET=""
for arg in "$@"; do
  case "$arg" in
    --format=*) FORMAT="${arg#--format=}" ;;
    --summary)  FORMAT="summary" ;;
    --json)     FORMAT="json" ;;
    --sarif)    FORMAT="sarif" ;;
    --strict)   STRICT=true ;;
    *)          TARGET="$arg" ;;
  esac
done
TARGET="${TARGET:-skills}"
DISPLAY_TARGET="$TARGET"

# Standalone / CI convenience (ADR-0025): if TARGET is a single file (for example a
# plugin's SKILL.md, or a loose .md), wrap it in a temporary one-skill directory so the
# directory-oriented discovery scans it. The wrapper is removed on exit.
if [[ -f "$TARGET" ]]; then
  _WRAP="$(mktemp -d)"
  trap 'rm -rf "$_WRAP"' EXIT
  cp "$TARGET" "$_WRAP/SKILL.md"
  TARGET="$_WRAP"
fi

# Suppress colors for machine-readable output
if [[ "$FORMAT" != "default" ]]; then
  RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' DIM='' RESET=''
fi

if [[ "$FORMAT" == "default" ]]; then
  log_header "Security scanning: $DISPLAY_TARGET"
fi

skills=()
while IFS= read -r dir; do
  skills+=("$dir")
done < <(discover_skills "$TARGET")

if (( ${#skills[@]} == 0 )); then
  # No SKILL.md at the given path. "Nothing to scan" is not a security finding, so this
  # exits clean (0) rather than 1, so that a CI gate does not false-fail on an empty or
  # restructured path. The message stays visible so a misconfigured path is noticeable.
  case "$FORMAT" in
    sarif) printf '%s\n' '{"$schema":"https://json.schemastore.org/sarif-2.1.0.json","version":"2.1.0","runs":[{"tool":{"driver":{"name":"OpenTrApp Skill Firewall","informationUri":"https://github.com/albertdobmeyer/opentrapp","rules":[]}},"results":[]}]}' ;;
    json)  printf '%s\n' '{"skills":0,"findings":0,"note":"no SKILL.md found at the given path"}' ;;
    *)     echo "No skill (SKILL.md) found in $DISPLAY_TARGET. Nothing to scan." ;;
  esac
  exit 0
fi

CRITICAL_COUNT=0
HIGH_COUNT=0
MEDIUM_COUNT=0
FINDING_COUNT=0

# ── Findings collector (JSON-lines in memory) ──
# Each finding: slug|severity|category|line_num|description|context|mitre_id|cve_ids
FINDINGS=()

# Check if a line should be ignored via inline comment or .scanignore
is_ignored() {
  local file="$1" line_num="$2" skill_dir="$3"
  local line
  line=$(sed -n "${line_num}p" "$file")

  # Same-line inline suppression ONLY (not ignore-next-line - prevents self-suppression)
  if echo "$line" | grep -q '<!-- scan:ignore -->'; then
    return 0
  fi

  # Previous line ONLY for ignore-next-line (a malicious line cannot suppress itself)
  if (( line_num > 1 )); then
    local prev_line
    prev_line=$(sed -n "$(( line_num - 1 ))p" "$file")
    if echo "$prev_line" | grep -q 'scan:ignore-next-line'; then
      return 0
    fi
  fi

  # Check .scanignore file
  local scanignore="$skill_dir/.scanignore"
  if [[ -f "$scanignore" ]]; then
    while IFS= read -r range; do
      range="${range%%$'\r'}"  # Strip Windows CR
      # Skip comments and empty lines
      [[ "$range" =~ ^#.*$ || -z "$range" ]] && continue
      # Support "L10-L20" range format
      if [[ "$range" =~ ^L([0-9]+)-L([0-9]+)$ ]]; then
        local start="${BASH_REMATCH[1]}" end="${BASH_REMATCH[2]}"
        if (( line_num >= start && line_num <= end )); then
          return 0
        fi
      fi
      # Support single line "L10" format
      if [[ "$range" =~ ^L([0-9]+)$ ]]; then
        if (( line_num == BASH_REMATCH[1] )); then
          return 0
        fi
      fi
    done < "$scanignore"
  fi

  return 1
}

# ── Scanignore audit ──
SCANIGNORE_AUDIT_FAIL=0

audit_scanignore() {
  local scanignore="$1" skill_dir="$2"
  local slug
  slug=$(get_skill_slug "$skill_dir")
  [[ -f "$scanignore" ]] || return 0

  while IFS= read -r range; do
    range="${range%%$'\r'}"
    [[ "$range" =~ ^#.*$ || -z "$range" ]] && continue

    # Validate format
    if [[ "$range" =~ ^L([0-9]+)-L([0-9]+)$ ]]; then
      local start="${BASH_REMATCH[1]}" end="${BASH_REMATCH[2]}"
      local span=$(( end - start + 1 ))
      if (( span > 50 )); then
        echo "  SCANIGNORE_AUDIT_FAIL: $slug/.scanignore - range L${start}-L${end} spans $span lines (max 50)" >&2
        SCANIGNORE_AUDIT_FAIL=$((SCANIGNORE_AUDIT_FAIL + 1))
      fi
    elif [[ "$range" =~ ^L([0-9]+)$ ]]; then
      : # Single line - valid
    else
      echo "  SCANIGNORE_AUDIT_FAIL: $slug/.scanignore - invalid format: $range" >&2
      SCANIGNORE_AUDIT_FAIL=$((SCANIGNORE_AUDIT_FAIL + 1))
    fi
  done < "$scanignore"
}

# ── Scannable file extensions ──
SCAN_EXTENSIONS=("*.md" "*.sh" "*.py" "*.js" "*.ts" "*.yaml" "*.yml" "*.json")

discover_scan_files() {
  local skill_dir="$1"
  for ext in "${SCAN_EXTENSIONS[@]}"; do
    find "$skill_dir" -maxdepth 2 -name "$ext" -type f 2>/dev/null
  done | sort -u
}

# ── Collect phase ──
# Per-skill summary for --summary mode
declare -A SKILL_CRITICAL SKILL_HIGH SKILL_MEDIUM

for skill_dir in "${skills[@]}"; do
  slug=$(get_skill_slug "$skill_dir")
  SKILL_CRITICAL[$slug]=0
  SKILL_HIGH[$slug]=0
  SKILL_MEDIUM[$slug]=0

  # Audit .scanignore
  audit_scanignore "$skill_dir/.scanignore" "$skill_dir"

  # Scan ALL matching files in the skill directory
  scan_files=()
  while IFS= read -r f; do
    [[ -n "$f" ]] && scan_files+=("$f")
  done < <(discover_scan_files "$skill_dir")

  # Fallback: if no files found but SKILL.md exists, scan it
  if (( ${#scan_files[@]} == 0 )) && [[ -f "$skill_dir/SKILL.md" ]]; then
    scan_files=("$skill_dir/SKILL.md")
  fi

  for file in "${scan_files[@]}"; do
    rel_file="${file#"$skill_dir/"}"

    for pattern_def in "${SCAN_PATTERNS[@]}"; do
      IFS='|' read -r severity category regex description mitre_id cve_ids flags <<< "$pattern_def"

      # Determine grep flags based on pattern flags field
      grep_flags="-nE"
      if [[ "${flags:-}" == *i* ]]; then
        grep_flags="-inE"
      fi

      while IFS=: read -r line_num match_line; do
        [[ -z "$line_num" ]] && continue

        if is_ignored "$file" "$line_num" "$skill_dir"; then
          continue
        fi

        FINDING_COUNT=$((FINDING_COUNT + 1))
        context=$(echo "$match_line" | head -c 120)

        # Store finding (includes rel_file for multi-file output)
        FINDINGS+=("${slug}|${severity}|${category}|${line_num}|${description}|${context}|${mitre_id:-}|${cve_ids:-}|${rel_file}")

        case "$severity" in
          CRITICAL)
            CRITICAL_COUNT=$((CRITICAL_COUNT + 1))
            SKILL_CRITICAL[$slug]=$(( ${SKILL_CRITICAL[$slug]} + 1 ))
            ;;
          HIGH)
            HIGH_COUNT=$((HIGH_COUNT + 1))
            SKILL_HIGH[$slug]=$(( ${SKILL_HIGH[$slug]} + 1 ))
            ;;
          *)
            MEDIUM_COUNT=$((MEDIUM_COUNT + 1))
            SKILL_MEDIUM[$slug]=$(( ${SKILL_MEDIUM[$slug]} + 1 ))
            ;;
        esac

      done < <(grep $grep_flags "$regex" "$file" 2>/dev/null || true)
    done
  done

  if (( SKILL_CRITICAL[$slug] + SKILL_HIGH[$slug] + SKILL_MEDIUM[$slug] == 0 )); then
    count_pass
  fi
done

# ── Render phase ──

render_default() {
  local current_slug=""
  for finding in "${FINDINGS[@]+"${FINDINGS[@]}"}"; do
    IFS='|' read -r slug severity category line_num description context mitre_id cve_ids rel_file <<< "$finding"

    if [[ "$slug" != "$current_slug" ]]; then
      echo -e "\n${CYAN}--- $slug ---${RESET}"
      current_slug="$slug"
    fi

    local loc="${rel_file:-SKILL.md}:L${line_num}"
    case "$severity" in
      CRITICAL) echo -e "  ${RED}CRITICAL${RESET} [${category}] ${loc}: ${description}" ;;
      HIGH)     echo -e "  ${YELLOW}HIGH${RESET}     [${category}] ${loc}: ${description}" ;;
      *)        echo -e "  ${BLUE}MEDIUM${RESET}   [${category}] ${loc}: ${description}" ;;
    esac
    echo -e "           ${DIM}${context}${RESET}"
  done

  # Print clean skills
  for skill_dir in "${skills[@]}"; do
    slug=$(get_skill_slug "$skill_dir")
    if (( SKILL_CRITICAL[$slug] + SKILL_HIGH[$slug] + SKILL_MEDIUM[$slug] == 0 )); then
      log_pass "$slug: Clean"
    fi
  done

  echo ""
  echo -e "${BOLD}Scan Results:${RESET}"
  echo -e "  Total findings: ${FINDING_COUNT}"
  echo -e "  ${RED}Critical: ${CRITICAL_COUNT}${RESET}"
  echo -e "  ${YELLOW}High: ${HIGH_COUNT}${RESET}"
  echo -e "  ${BLUE}Medium: ${MEDIUM_COUNT}${RESET}"
  echo -e "  ${GREEN}Clean skills: ${PASS_COUNT}${RESET}"
  if (( SCANIGNORE_AUDIT_FAIL > 0 )); then
    echo -e "  ${YELLOW}Scanignore audit failures: ${SCANIGNORE_AUDIT_FAIL}${RESET}"
  fi
  echo ""
}

render_summary() {
  for skill_dir in "${skills[@]}"; do
    slug=$(get_skill_slug "$skill_dir")
    local c=${SKILL_CRITICAL[$slug]} h=${SKILL_HIGH[$slug]} m=${SKILL_MEDIUM[$slug]}
    if (( c + h + m == 0 )); then
      echo "PASS $slug"
    else
      local parts=()
      (( c > 0 )) && parts+=("${c}C")
      (( h > 0 )) && parts+=("${h}H")
      (( m > 0 )) && parts+=("${m}M")
      echo "FAIL $slug ($(IFS=', '; echo "${parts[*]}"))"
    fi
  done
}

render_json() {
  echo '{'
  echo '  "scanner": "openagent-skills-skill-scan",'
  echo '  "version": "2.0.0",'
  echo "  \"patternCount\": ${#SCAN_PATTERNS[@]},"
  echo "  \"skillsScanned\": ${#skills[@]},"
  echo '  "summary": {'
  echo "    \"total\": ${FINDING_COUNT},"
  echo "    \"critical\": ${CRITICAL_COUNT},"
  echo "    \"high\": ${HIGH_COUNT},"
  echo "    \"medium\": ${MEDIUM_COUNT},"
  echo "    \"clean\": ${PASS_COUNT}"
  echo '  },'
  echo '  "findings": ['

  local i=0
  for finding in "${FINDINGS[@]+"${FINDINGS[@]}"}"; do
    IFS='|' read -r slug severity category line_num description context mitre_id cve_ids rel_file <<< "$finding"
    (( i > 0 )) && echo ','
    local esc_desc esc_ctx
    esc_desc=$(json_escape "$description")
    esc_ctx=$(json_escape "$context")
    printf '    {"skill":"%s","severity":"%s","category":"%s","line":%s,"file":"%s","description":"%s","context":"%s","mitreId":"%s","cveIds":"%s"}' \
      "$slug" "$severity" "$category" "$line_num" "${rel_file:-SKILL.md}" "$esc_desc" "$esc_ctx" "${mitre_id:-}" "${cve_ids:-}"
    i=$((i + 1))
  done

  echo ''
  echo '  ],'
  local blocked=0
  if (( CRITICAL_COUNT > 0 )); then blocked=1; fi
  if [[ "$STRICT" == true ]] && (( HIGH_COUNT > 0 || SCANIGNORE_AUDIT_FAIL > 0 )); then blocked=1; fi
  echo "  \"blocked\": $blocked"
  echo '}'
}

render_sarif() {
  local py
  py=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
    echo "Error: Python not found (needed for SARIF output)" >&2; exit 1
  }
  "$py" "$SCRIPT_DIR/lib/sarif_formatter.py" < <(render_json)
}

case "$FORMAT" in
  default) render_default ;;
  summary) render_summary ;;
  json)    render_json ;;
  sarif)   render_sarif ;;
  *)
    echo "Unknown format: $FORMAT (use default, summary, json, sarif)"
    exit 1
    ;;
esac

# In --strict mode, scanignore audit failures also block
if [[ "$STRICT" == true ]] && (( SCANIGNORE_AUDIT_FAIL > 0 )); then
  if [[ "$FORMAT" == "default" ]]; then
    echo -e "${RED}BLOCKED (strict): ${SCANIGNORE_AUDIT_FAIL} scanignore audit failure(s).${RESET}"
  fi
  exit 1
fi

if (( CRITICAL_COUNT > 0 )); then
  if [[ "$FORMAT" == "default" ]]; then
    echo -e "${RED}BLOCKED: ${CRITICAL_COUNT} critical finding(s). Review and allowlist or fix.${RESET}"
    echo "  Use '# scan:ignore-next-line' or a .scanignore file to allowlist expected patterns."
  fi
  exit 1
fi

# In --strict mode, HIGH findings also block
if [[ "$STRICT" == true ]] && (( HIGH_COUNT > 0 )); then
  if [[ "$FORMAT" == "default" ]]; then
    echo -e "${RED}BLOCKED (strict): ${HIGH_COUNT} high finding(s). Use --strict to enforce HIGH blocking.${RESET}"
  fi
  exit 1
fi
