#!/usr/bin/env bash
# Zero-trust skill verifier — guilty until every line proven safe
# Usage: skill-verify.sh [--strict] [--report] [--json] [--trust] <skill_dir|skills_parent>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/line-classifier.sh"
source "$SCRIPT_DIR/lib/trust-manifest.sh"

# Portable python
PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || PY=""

# ── Parse arguments ──
STRICT=false
REPORT=false
JSON_OUT=false
GENERATE_TRUST=false
TARGET=""
for arg in "$@"; do
  case "$arg" in
    --strict)  STRICT=true ;;
    --report)  REPORT=true ;;
    --json)    JSON_OUT=true ;;
    --trust)   GENERATE_TRUST=true ;;
    *)         TARGET="$arg" ;;
  esac
done
TARGET="${TARGET:-skills}"

# Suppress colors for JSON output
if [[ "$JSON_OUT" == true ]]; then
  RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' DIM='' RESET=''
fi

# ── Discover skills ──
skills=()
while IFS= read -r dir; do
  skills+=("$dir")
done < <(discover_skills "$TARGET")

if (( ${#skills[@]} == 0 )); then
  echo "No skills found in $TARGET"
  exit 1
fi

# ── Scannable file extensions (same as scanner) ──
VERIFY_EXTENSIONS=("*.md" "*.sh" "*.py" "*.js" "*.ts" "*.yaml" "*.yml" "*.json")

discover_verify_files() {
  local skill_dir="$1"
  for ext in "${VERIFY_EXTENSIONS[@]}"; do
    find "$skill_dir" -maxdepth 2 -name "$ext" -type f 2>/dev/null
  done | sort -u
}

# ── Verify a single skill ──
ANY_FAILED=false

verify_skill() {
  local skill_dir="$1"
  local slug
  slug=$(get_skill_slug "$skill_dir")

  # Check for .trust file (our own skills fast path)
  if [[ -f "$skill_dir/.trust" ]]; then
    local stored_hash current_hash
    stored_hash=$(grep '^VERIFY_HASH=' "$skill_dir/.trust" 2>/dev/null | cut -d= -f2)
    if [[ -n "$stored_hash" ]]; then
      # Compute current hash of all skill files
      current_hash=$(find "$skill_dir" -maxdepth 2 -type f \
        ! -name '.trust' ! -name '.scanignore' \
        | sort | xargs cat 2>/dev/null | sha256sum | cut -d' ' -f1)
      current_hash="sha256:$current_hash"
      if [[ "$stored_hash" == "$current_hash" ]]; then
        if [[ "$JSON_OUT" != true ]]; then
          echo -e "  ${GREEN}TRUSTED${RESET}  $slug (hash-pinned, skipped verification)"
        fi
        return 0
      fi
      # Hash mismatch — fall through to full verification
      if [[ "$JSON_OUT" != true && "$REPORT" == true ]]; then
        echo -e "  ${YELLOW}WARN${RESET}    $slug: .trust hash mismatch — running full verification"
      fi
    fi
  fi

  # Discover files
  local verify_files=()
  while IFS= read -r f; do
    [[ -n "$f" ]] && verify_files+=("$f")
  done < <(discover_verify_files "$skill_dir")

  if (( ${#verify_files[@]} == 0 )) && [[ -f "$skill_dir/SKILL.md" ]]; then
    verify_files=("$skill_dir/SKILL.md")
  fi

  local total_lines=0 safe_lines=0 suspicious_lines=0 malicious_lines=0
  local file_count=${#verify_files[@]}
  local mal_details=()
  local sus_details=()

  for file in "${verify_files[@]}"; do
    local rel_file="${file#"$skill_dir/"}"

    # Read file content for detail lookup
    local -a file_lines=()
    while IFS= read -r fline || [[ -n "$fline" ]]; do
      file_lines+=("$fline")
    done < "$file"

    # Batch classify entire file
    while IFS='|' read -r line_num verdict reason; do
      [[ -z "$line_num" ]] && continue
      total_lines=$((total_lines + 1))

      # Get the actual line content (0-indexed array)
      local line_content="${file_lines[$((line_num - 1))]:-}"

      case "$verdict" in
        SAFE)
          safe_lines=$((safe_lines + 1))
          ;;
        SUSPICIOUS)
          suspicious_lines=$((suspicious_lines + 1))
          sus_details+=("${rel_file}:L${line_num}|${reason}|${line_content}")
          ;;
        MALICIOUS)
          malicious_lines=$((malicious_lines + 1))
          mal_details+=("${rel_file}:L${line_num}|${reason}|${line_content}")
          ;;
      esac
    done < <(classify_file "$file" "$skill_dir")
  done

  # ── Determine verdict ──
  local final_verdict="VERIFIED"
  local verdict_reason=""
  if (( malicious_lines > 0 )); then
    final_verdict="QUARANTINED"
    verdict_reason="${malicious_lines} malicious"
  fi
  if (( suspicious_lines > 0 )); then
    final_verdict="QUARANTINED"
    if [[ -n "$verdict_reason" ]]; then
      verdict_reason="${verdict_reason}, ${suspicious_lines} suspicious"
    else
      verdict_reason="${suspicious_lines} suspicious"
    fi
  fi

  # ── Generate trust manifest if --trust and skill passed ──
  if [[ "$GENERATE_TRUST" == true && "$final_verdict" == "VERIFIED" ]]; then
    generate_trust_manifest "$skill_dir" "$total_lines" "$safe_lines" "$suspicious_lines"
  fi

  # ── JSON output ──
  if [[ "$JSON_OUT" == true ]]; then
    local sus_pct=0
    if (( total_lines > 0 )); then
      sus_pct=$($PY -c "print(round(($suspicious_lines / $total_lines) * 100, 1))" 2>/dev/null || echo "0")
    fi

    echo '{'
    echo "  \"skill\": \"$slug\","
    echo "  \"verdict\": \"$final_verdict\","
    echo "  \"files\": $file_count,"
    echo '  "lines": {'
    echo "    \"total\": $total_lines,"
    echo "    \"safe\": $safe_lines,"
    echo "    \"suspicious\": $suspicious_lines,"
    echo "    \"malicious\": $malicious_lines"
    echo '  },'
    echo "  \"suspiciousPercent\": $sus_pct,"

    # maliciousLines array
    echo '  "maliciousLines": ['
    local i=0
    for detail in "${mal_details[@]+"${mal_details[@]}"}"; do
      IFS='|' read -r loc reason content <<< "$detail"
      (( i > 0 )) && echo ','
      local esc_content
      esc_content=$(json_escape "$content")
      local esc_reason
      esc_reason=$(json_escape "$reason")
      printf '    {"location":"%s","reason":"%s","content":"%s"}' "$loc" "$esc_reason" "$esc_content"
      i=$((i + 1))
    done
    echo ''
    echo '  ],'

    # suspiciousLines array
    echo '  "suspiciousLines": ['
    i=0
    for detail in "${sus_details[@]+"${sus_details[@]}"}"; do
      IFS='|' read -r loc reason content <<< "$detail"
      (( i > 0 )) && echo ','
      local esc_content
      esc_content=$(json_escape "$content")
      local esc_reason
      esc_reason=$(json_escape "$reason")
      printf '    {"location":"%s","reason":"%s","content":"%s"}' "$loc" "$esc_reason" "$esc_content"
      i=$((i + 1))
    done
    echo ''
    echo '  ]'

    echo '}'
    if [[ "$final_verdict" != "VERIFIED" ]]; then
      ANY_FAILED=true
    fi
    return
  fi

  # ── Default / report output ──
  local safe_pct=0 sus_pct=0 mal_pct=0
  if (( total_lines > 0 )); then
    safe_pct=$($PY -c "print(round(($safe_lines / $total_lines) * 100, 1))" 2>/dev/null || echo "0")
    sus_pct=$($PY -c "print(round(($suspicious_lines / $total_lines) * 100, 1))" 2>/dev/null || echo "0")
    mal_pct=$($PY -c "print(round(($malicious_lines / $total_lines) * 100, 1))" 2>/dev/null || echo "0")
  fi

  echo ""
  echo -e "${BOLD}VERIFY: $slug${RESET}"
  echo "  Files: $file_count"
  echo "  Lines: $total_lines"
  echo -e "  Safe: $safe_lines (${safe_pct}%)"
  echo -e "  Suspicious: $suspicious_lines (${sus_pct}%)"
  echo -e "  Malicious: $malicious_lines (${mal_pct}%)"

  if [[ "$final_verdict" == "VERIFIED" ]]; then
    echo -e "  Verdict: ${GREEN}VERIFIED${RESET} (released from quarantine)"
  else
    echo -e "  Verdict: ${RED}QUARANTINED${RESET} (${verdict_reason})"
  fi

  # Report mode: show per-line details
  if [[ "$REPORT" == true ]]; then
    if (( malicious_lines > 0 )); then
      echo ""
      echo -e "  ${RED}Malicious lines:${RESET}"
      for detail in "${mal_details[@]}"; do
        IFS='|' read -r loc reason content <<< "$detail"
        echo -e "    ${RED}$loc${RESET}  $content"
        echo -e "           ${DIM}[$reason]${RESET}"
      done
    fi

    if (( suspicious_lines > 0 )); then
      echo ""
      echo -e "  ${YELLOW}Suspicious lines:${RESET}"
      local shown=0
      for detail in "${sus_details[@]}"; do
        IFS='|' read -r loc reason content <<< "$detail"
        echo -e "    ${YELLOW}$loc${RESET}  $content"
        echo -e "           ${DIM}[$reason]${RESET}"
        shown=$((shown + 1))
        if (( shown >= 20 )); then
          local remaining=$((suspicious_lines - shown))
          if (( remaining > 0 )); then
            echo "    ... and $remaining more"
          fi
          break
        fi
      done
    fi
  fi

  if [[ "$final_verdict" != "VERIFIED" ]]; then
    ANY_FAILED=true
  fi
}

# ── Main ──
if [[ "$JSON_OUT" != true ]]; then
  log_header "Zero-Trust Verification: $TARGET"
fi

for skill_dir in "${skills[@]}"; do
  verify_skill "$skill_dir"
done

if [[ "$ANY_FAILED" == true ]]; then
  if [[ "$JSON_OUT" != true ]]; then
    echo ""
    echo -e "${RED}BLOCKED: One or more skills failed zero-trust verification.${RESET}"
  fi
  exit 1
fi

if [[ "$JSON_OUT" != true ]]; then
  echo ""
  echo -e "${GREEN}All skills verified.${RESET}"
fi
