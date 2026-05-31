#!/usr/bin/env bash
# Stage 3: CDR pre-filter — strips dangerous content using line classifier
# Input: structured JSON from cdr-parse.py (file path as arg)
# Output: filtered structured JSON to stdout
# Exit 1 if skill is too dangerous (forbidden patterns or nothing survives)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"
source "$SCRIPT_DIR/line-classifier.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found" >&2; exit 1
}

INPUT_FILE="${1:-}"
if [[ -z "$INPUT_FILE" || ! -f "$INPUT_FILE" ]]; then
  echo "Usage: cdr-prefilter.sh <structured.json>" >&2
  exit 1
fi

# Extract all text content into a temp file for classification
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

"$PY" -c "
import json, sys
data = json.load(open(sys.argv[1]))
for section in data.get('sections', []):
    for line in section.get('prose', []):
        print(line)
    for block in section.get('code_blocks', []):
        for line in block.get('lines', []):
            print(line)
" "$INPUT_FILE" > "$TMPFILE"

TOTAL_LINES=$(wc -l < "$TMPFILE")
if (( TOTAL_LINES == 0 )); then
  echo "REJECTED: No content to filter" >&2
  exit 1
fi

# Check for forbidden terminal patterns that cause FULL rejection
FORBIDDEN_PATTERNS=(
  'curl\s.*\|\s*(ba)?sh'
  'wget\s.*\|\s*(ba)?sh'
  'eval\s*\$\('
)

for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
  if grep -qE "$pattern" "$TMPFILE" 2>/dev/null; then
    echo "REJECTED: Forbidden pattern: $pattern" >&2
    exit 1
  fi
done

# Clear scanignore (no suppressions for untrusted content)
_SCANIGNORE_LINES=()

# Run blocklist scanner on extracted content
declare -A MALICIOUS_LINES=()
while IFS='|' read -r lnum severity category desc; do
  [[ -z "$lnum" ]] && continue
  MALICIOUS_LINES[$lnum]=1
done < <(_blocklist_scan_file "$TMPFILE")

STRIPPED_COUNT=${#MALICIOUS_LINES[@]}

# Build space-separated list of malicious line numbers for Python
MAL_NUMS=""
for k in "${!MALICIOUS_LINES[@]}"; do
  MAL_NUMS="$MAL_NUMS $k"
done

# Filter the structured JSON, removing dangerous lines
"$PY" -c "
import json, sys

data = json.load(open(sys.argv[1]))
mal_nums = set(int(x) for x in sys.argv[2].split() if x)

line_num = 0
for section in data.get('sections', []):
    safe_prose = []
    for line in section.get('prose', []):
        line_num += 1
        if line_num not in mal_nums:
            safe_prose.append(line)
    section['prose'] = safe_prose

    safe_blocks = []
    for block in section.get('code_blocks', []):
        safe_lines = []
        for line in block.get('lines', []):
            line_num += 1
            if line_num not in mal_nums:
                safe_lines.append(line)
        if safe_lines:
            block['lines'] = safe_lines
            safe_blocks.append(block)
    section['code_blocks'] = safe_blocks

# Remove empty sections
data['sections'] = [s for s in data['sections']
                    if s.get('prose') or s.get('code_blocks')]

if not data['sections']:
    print('REJECTED: No safe content survived filtering', file=sys.stderr)
    sys.exit(1)

json.dump(data, sys.stdout, indent=2)
print()
" "$INPUT_FILE" "$MAL_NUMS" || {
  echo "REJECTED: No safe content survived filtering" >&2
  exit 1
}

if (( STRIPPED_COUNT > 0 )); then
  echo "Pre-filter: stripped $STRIPPED_COUNT dangerous line(s)" >&2
fi
