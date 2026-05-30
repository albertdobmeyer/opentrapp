#!/usr/bin/env bash
# AI test generation via Ollama
# Usage: create-tests.sh <skill.md> <output-path>
# Reads a SKILL.md and generates test assertions using the test framework
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found" >&2; exit 1
}

# Load CDR config (shared Ollama settings)
CDR_CONF="$REPO_ROOT/config/cdr.conf"
if [[ -f "$CDR_CONF" ]]; then
  source "$CDR_CONF"
fi
CDR_MODEL="${CDR_MODEL:-qwen2.5-coder:1.5b}"
CDR_ENDPOINT="${CDR_ENDPOINT:-http://localhost:11434/api/generate}"
CDR_TIMEOUT="${CDR_TIMEOUT:-120}"

SKILL_FILE="${1:-}"
OUTPUT_PATH="${2:-}"

if [[ -z "$SKILL_FILE" || ! -f "$SKILL_FILE" || -z "$OUTPUT_PATH" ]]; then
  echo "Usage: create-tests.sh <skill.md> <output-path>" >&2
  exit 1
fi

# Check Ollama is reachable
if ! curl -sf --max-time 5 "http://localhost:11434/api/tags" > /dev/null 2>&1; then
  echo "Error: Ollama is not running at localhost:11434" >&2
  echo "Start it with: ollama serve" >&2
  exit 1
fi

# Extract skill name from frontmatter
SKILL_NAME=$("$PY" -c "
import sys
with open(sys.argv[1]) as f:
    for line in f:
        line = line.strip()
        if line.startswith('name:'):
            print(line.split(':', 1)[1].strip())
            break
" "$SKILL_FILE" 2>/dev/null)

if [[ -z "$SKILL_NAME" ]]; then
  echo "Error: Could not extract skill name from frontmatter" >&2
  exit 1
fi

# System prompt
SYSTEM_PROMPT='You are a test writer for OpenClaw agent skills. Generate test assertions for a SKILL.md file.

Available assertion functions (these are the ONLY functions you may use):

  assert_frontmatter_field "$SKILL" "field_name" "regex_pattern"
    — Checks a YAML frontmatter field matches a regex pattern

  assert_section_exists "$SKILL" "Heading Text"
    — Checks a markdown heading exists (any level)

  assert_contains "$SKILL" "regex_pattern"
    — Checks file contains a regex pattern

  assert_not_contains "$SKILL" "regex_pattern"
    — Checks file does NOT contain a regex pattern

  assert_min_code_blocks "$SKILL" N
    — Checks file has at least N code blocks

  assert_line_count "$SKILL" MIN MAX
    — Checks line count is between MIN and MAX

Rules:
- Output ONLY test function definitions, one per function
- Each function name must start with test_
- Use $SKILL as the file variable (it is already set)
- Write 4-8 test functions covering: frontmatter, required sections, key content, code blocks
- Do NOT include #!/usr/bin/env bash or source statements
- Do NOT include comments
- One blank line between functions
- Check for TODO/FIXME/XXX placeholders with assert_not_contains

Example output format:
test_has_frontmatter() {
  assert_frontmatter_field "$SKILL" "name" "^my-skill$"
  assert_frontmatter_field "$SKILL" "version" "^[0-9]"
  assert_frontmatter_field "$SKILL" "description" "."
}

test_has_required_sections() {
  assert_section_exists "$SKILL" "When to Use"
  assert_section_exists "$SKILL" "Tips"
}

test_no_placeholders() {
  assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
}'

# Read skill content
SKILL_CONTENT=$(cat "$SKILL_FILE")

# Build request and get response
ASSERTIONS=$("$PY" -c "
import json, sys, urllib.request

skill_content = sys.argv[1]
system_prompt = sys.argv[2]
model = sys.argv[3]
endpoint = sys.argv[4]
timeout = int(sys.argv[5])

request = {
    'model': model,
    'system': system_prompt,
    'prompt': 'Generate test functions for this SKILL.md:\n\n' + skill_content,
    'stream': False
}

req = urllib.request.Request(
    endpoint,
    data=json.dumps(request).encode(),
    headers={'Content-Type': 'application/json'}
)

try:
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        result = json.loads(resp.read())
except Exception as e:
    print(f'Error: Ollama request failed: {e}', file=sys.stderr)
    sys.exit(1)

text = result.get('response', '')
if not text.strip():
    print('Error: Ollama returned empty response', file=sys.stderr)
    sys.exit(1)

# Strip markdown fences if present
text = text.strip()
if text.startswith('\`\`\`bash'):
    text = text[len('\`\`\`bash'):].strip()
if text.startswith('\`\`\`'):
    text = text[3:].strip()
if text.endswith('\`\`\`'):
    text = text[:-3].strip()

print(text)
" "$SKILL_CONTENT" "$SYSTEM_PROMPT" "$CDR_MODEL" "$CDR_ENDPOINT" "$CDR_TIMEOUT") || {
  echo "Error: Test generation failed" >&2
  exit 1
}

# Wrap in boilerplate
cat > "$OUTPUT_PATH" << TESTEOF
#!/usr/bin/env bash
# Tests for ${SKILL_NAME} skill (AI-generated)

${ASSERTIONS}
TESTEOF
