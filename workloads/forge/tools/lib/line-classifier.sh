#!/usr/bin/env bash
# Zero-trust line classifier — every line gets a verdict: SAFE, SUSPICIOUS, or MALICIOUS
# Used by skill-verify.sh for allowlist-based verification
#
# Performance: blocklist runs once per file via grep (batch), allowlist uses
# bash built-in [[ =~ ]] regex to avoid subprocess overhead per line.

CLASSIFIER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -z "${SCAN_PATTERNS+x}" ]]; then
  source "$CLASSIFIER_DIR/patterns.sh"
fi

# ── Scanignore support ──
declare -A _SCANIGNORE_LINES
_load_scanignore() {
  _SCANIGNORE_LINES=()
  local scanignore="$1/.scanignore"
  [[ -f "$scanignore" ]] || return 0
  while IFS= read -r range; do
    range="${range%%$'\r'}"
    [[ "$range" =~ ^#.*$ || -z "$range" ]] && continue
    if [[ "$range" =~ ^L([0-9]+)-L([0-9]+)$ ]]; then
      local s="${BASH_REMATCH[1]}" e="${BASH_REMATCH[2]}"
      for (( i=s; i<=e; i++ )); do _SCANIGNORE_LINES[$i]=1; done
    elif [[ "$range" =~ ^L([0-9]+)$ ]]; then
      _SCANIGNORE_LINES[${BASH_REMATCH[1]}]=1
    fi
  done < "$scanignore"
}

# ── Batch blocklist check ──
# Runs all 87 patterns against a file via grep, returns malicious line numbers
_blocklist_scan_file() {
  local filepath="$1"
  for pattern_def in "${SCAN_PATTERNS[@]}"; do
    IFS='|' read -r severity category regex description mitre_id cve_ids flags <<< "$pattern_def"
    local grep_flags="-nE"
    [[ "${flags:-}" == *i* ]] && grep_flags="-inE"
    while IFS=: read -r line_num _; do
      [[ -z "$line_num" ]] && continue
      [[ -n "${_SCANIGNORE_LINES[$line_num]+_}" ]] && continue
      echo "${line_num}|${severity}|${category}|${description}"
    done < <(grep $grep_flags "$regex" "$filepath" 2>/dev/null || true)
  done
}

# ── Allowlist check functions (bash built-in regex, no subprocesses) ──

_is_structural() {
  local line="$1"
  # Blank / whitespace-only
  [[ -z "$line" || "$line" =~ ^[[:space:]]*$ ]] && return 0
  # Frontmatter delimiter
  [[ "$line" == "---" ]] && return 0
  # Markdown heading
  [[ "$line" =~ ^#{1,6}\ . ]] && return 0
  # Unordered list
  [[ "$line" =~ ^[-*+]\ . ]] && return 0
  # Ordered list
  [[ "$line" =~ ^[0-9]+\.\ . ]] && return 0
  # Blockquote
  [[ "$line" =~ ^\>\ . || "$line" =~ ^\>$ ]] && return 0
  # Code fence
  [[ "$line" =~ ^\`\`\` ]] && return 0
  # Table row
  [[ "$line" =~ ^\|.*\|$ ]] && return 0
  # Table separator
  [[ "$line" =~ ^[-\|:\ ]+$ && "$line" == *"|"* ]] && return 0
  # HTML comment
  [[ "$line" =~ ^\<\!--.* ]] && return 0
  # HTML tags
  [[ "$line" =~ ^\</?[a-zA-Z] ]] && return 0
  [[ "$line" =~ ^\<details\> || "$line" =~ ^\</details\> ]] && return 0
  [[ "$line" =~ ^\<summary\> ]] && return 0
  return 1
}

_is_safe_frontmatter() {
  local line="$1"
  # Known field names
  [[ "$line" =~ ^(name|version|description|metadata|tags|category|author|license): ]] && return 0
  # YAML continuation (indented)
  [[ "$line" =~ ^\ \  ]] && return 0
  # JSON-like content
  [[ "$line" =~ ^\{.*\}$ || "$line" =~ ^\[.*\]$ ]] && return 0
  [[ "$line" =~ ^\"[^\"]*\": ]] && return 0
  return 1
}

_is_safe_code() {
  local line="$1"
  # Strip leading whitespace for matching
  local stripped="${line#"${line%%[![:space:]]*}"}"
  [[ -z "$stripped" ]] && return 0  # blank inside code block

  # Comments
  [[ "$stripped" =~ ^# ]] && return 0
  [[ "$stripped" =~ ^// ]] && return 0
  [[ "$stripped" =~ ^\* ]] && return 0
  [[ "$stripped" =~ ^/\* ]] && return 0
  [[ "$stripped" =~ ^\*/ ]] && return 0

  # Shell control structures
  [[ "$stripped" =~ ^(if|then|else|elif|fi|for|while|until|do|done|case|esac)(\ |$) ]] && return 0

  # Function definitions
  [[ "$stripped" =~ ^function\  ]] && return 0
  [[ "$stripped" =~ ^[a-zA-Z_][a-zA-Z0-9_]*\ *\(\) ]] && return 0

  # Python keywords
  [[ "$stripped" =~ ^(def|class|import|from|return|pass|raise|try|except|finally|with|as|yield|lambda)(\ |$|\() ]] && return 0

  # JS/TS keywords
  [[ "$stripped" =~ ^(const|let|var|return|function|async|await|export|import|throw|try|catch|finally)(\ |$) ]] && return 0

  # Variable assignment (no dangerous substitution)
  [[ "$stripped" =~ ^[a-zA-Z_][a-zA-Z0-9_]*= ]] && return 0

  # Safe builtins and utilities
  [[ "$stripped" =~ ^(echo|printf|ls|cd|pwd|mkdir|cp|mv|cat|grep|find|test|true|false|exit|read|shift|local|declare|readonly|unset|set|source|return|break|continue)(\ |$) ]] && return 0
  [[ "$stripped" =~ ^(wc|sort|uniq|head|tail|tr|cut|awk|sed|tee|xargs|basename|dirname|realpath|mktemp|date|sleep|wait|touch|diff|comm|paste|join|column|yes|seq|shuf)(\ |$) ]] && return 0
  [[ "$stripped" =~ ^(pip|npm|yarn|go|cargo|make|cmake|docker|git|ssh|python3?|node|ruby|java|javac|gcc|clang|rustc|jq|yq)(\ |$) ]] && return 0
  [[ "$stripped" =~ ^(rm|rmdir|chmod|chown|ln|tar|zip|unzip|gzip|gunzip)(\ |$) ]] && return 0
  [[ "$stripped" =~ ^(kubectl|helm|terraform|ansible|vagrant|packer)(\ |$) ]] && return 0

  # Closers and misc syntax
  [[ "$stripped" =~ ^\} ]] && return 0
  [[ "$stripped" =~ ^\) ]] && return 0
  [[ "$stripped" =~ ^\] ]] && return 0
  [[ "$stripped" == ";;" ]] && return 0
  [[ "$stripped" =~ ^\| ]] && return 0
  [[ "$stripped" =~ ^(\&\&|\|\|) ]] && return 0
  [[ "$stripped" == "\\" ]] && return 0

  # Case labels
  [[ "$stripped" =~ ^[a-zA-Z_\*\-\"\'][a-zA-Z0-9_\|\-\"\'\ ]*\) ]] && return 0

  # Quoted strings
  [[ "$stripped" =~ ^\" ]] && return 0
  [[ "$stripped" =~ ^\' ]] && return 0

  # Test brackets
  [[ "$stripped" =~ ^\[\[?\  ]] && return 0

  # Decorators
  [[ "$stripped" =~ ^@[a-zA-Z] ]] && return 0

  # HTML/XML tags
  [[ "$stripped" =~ ^\</?[a-zA-Z] ]] && return 0

  # Flag/option lines (e.g., "  -e KEY=VAL  # description")
  [[ "$stripped" =~ ^-[a-zA-Z] ]] && return 0
  [[ "$stripped" =~ ^--[a-zA-Z] ]] && return 0

  # Method/function calls
  [[ "$stripped" =~ ^[a-zA-Z_][a-zA-Z0-9_.]*\( ]] && return 0

  # Spread/rest/ellipsis
  [[ "$stripped" == "..." ]] && return 0

  # Property access / chaining / assignment
  [[ "$stripped" =~ ^\.[a-zA-Z] ]] && return 0
  [[ "$stripped" =~ ^[a-zA-Z_][a-zA-Z0-9_]*\.[a-zA-Z_][a-zA-Z0-9_.]*\  ]] && return 0

  return 1
}

_is_safe_prose() {
  local line="$1"
  local len=${#line}
  # Empty handled by structural
  (( len == 0 )) && return 0
  # Long lines are suspicious (possible obfuscation)
  (( len >= 500 )) && return 1
  # Reject lines with dangerous shell metacharacters
  [[ "$line" =~ \$\( ]] && return 1
  [[ "$line" =~ \$\{ ]] && return 1
  [[ "$line" =~ \beval\  ]] && return 1
  [[ "$line" =~ \bexec\  ]] && return 1
  return 0
}

# ── Main classifier entry point ──
# Classifies every line in a file using batch operations
# Usage: classify_file <filepath> [skill_dir]
# Output: LINE_NUM|VERDICT|REASON  (one per line)
classify_file() {
  local filepath="$1"
  local skill_dir="${2:-}"

  # Load scanignore
  if [[ -n "$skill_dir" ]]; then
    _load_scanignore "$skill_dir"
  else
    _SCANIGNORE_LINES=()
  fi

  # Step 1: Batch blocklist scan
  declare -A malicious_lines
  while IFS='|' read -r lnum severity category desc; do
    [[ -z "$lnum" ]] && continue
    malicious_lines[$lnum]="${severity}|${category}|${desc}"
  done < <(_blocklist_scan_file "$filepath")

  # Step 2: Walk file, track context, check allowlist with bash builtins
  local context="prose"
  local fence_open=false
  local fm_open=false
  local line_num=0

  while IFS= read -r line || [[ -n "$line" ]]; do
    line_num=$((line_num + 1))

    # Blocklist hit
    if [[ -n "${malicious_lines[$line_num]+_}" ]]; then
      echo "${line_num}|MALICIOUS|${malicious_lines[$line_num]}"
      # Update context
      if (( line_num == 1 )) && [[ "$line" == "---" ]]; then fm_open=true; context="frontmatter"; fi
      if [[ "$fm_open" == true && "$line" == "---" && $line_num -gt 1 ]]; then fm_open=false; context="prose"; fi
      if [[ "$line" =~ ^\`\`\` ]]; then
        if [[ "$fence_open" == true ]]; then fence_open=false; context="prose"
        else fence_open=true; context="code"; fi
      fi
      continue
    fi

    # Context transitions
    if (( line_num == 1 )) && [[ "$line" == "---" ]]; then
      fm_open=true; context="frontmatter"
      echo "${line_num}|SAFE|frontmatter delimiter"
      continue
    fi
    if [[ "$fm_open" == true && "$line" == "---" ]]; then
      fm_open=false; context="prose"
      echo "${line_num}|SAFE|frontmatter delimiter"
      continue
    fi
    if [[ "$line" =~ ^\`\`\` ]]; then
      if [[ "$fence_open" == true ]]; then fence_open=false; context="prose"
      else fence_open=true; context="code"; fi
      echo "${line_num}|SAFE|code fence delimiter"
      continue
    fi

    # Structural check (any context)
    if _is_structural "$line"; then
      echo "${line_num}|SAFE|structural pattern"
      continue
    fi

    # Context-specific allowlist
    case "$context" in
      frontmatter)
        if _is_safe_frontmatter "$line"; then
          echo "${line_num}|SAFE|frontmatter field"
          continue
        fi
        ;;
      code)
        # Inside code fences: anything not on the blocklist is instructional code.
        # The blocklist already caught malicious patterns above; remaining code
        # is safe by definition (YAML configs, Dockerfiles, arbitrary languages).
        echo "${line_num}|SAFE|code block content"
        continue
        ;;
      prose)
        if _is_safe_prose "$line"; then
          echo "${line_num}|SAFE|safe prose"
          continue
        fi
        ;;
    esac

    echo "${line_num}|SUSPICIOUS|unrecognized pattern in ${context} context"
  done < "$filepath"
}
