#!/usr/bin/env bash
# AI skill draft generation via Ollama
# Usage: create-draft.sh <input.json> <output-path> [error-feedback]
# Input JSON: {"name","type","description","commands","tips"}
# Output: raw SKILL.md written to output-path
# Optional third arg: error feedback for retry mode
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
# Backend protocol (shared with CDR): "ollama" (/api/generate) or "openai"
# (/v1/chat/completions). Lets skill creation reuse a model you already run.
CDR_API_FORMAT="${CDR_API_FORMAT:-ollama}"
CDR_API_KEY="${CDR_API_KEY:-}"

INPUT_FILE="${1:-}"
OUTPUT_PATH="${2:-}"
ERROR_FEEDBACK="${3:-}"

if [[ -z "$INPUT_FILE" || ! -f "$INPUT_FILE" || -z "$OUTPUT_PATH" ]]; then
  echo "Usage: create-draft.sh <input.json> <output-path> [error-feedback]" >&2
  exit 1
fi

# Check the model endpoint is reachable. Use the configured endpoint's host
# (not a hardcoded localhost) and a protocol-appropriate probe path.
CDR_HOST=$(echo "$CDR_ENDPOINT" | sed -E 's|(https?://[^/]+).*|\1|')
if [[ "$CDR_API_FORMAT" == "openai" ]]; then
  PROBE_URL="${CDR_HOST}/v1/models"
else
  PROBE_URL="${CDR_HOST}/api/tags"
fi
PROBE_AUTH=()
[[ -n "$CDR_API_KEY" ]] && PROBE_AUTH=(-H "Authorization: Bearer ${CDR_API_KEY}")
if ! curl -sf --max-time 5 "${PROBE_AUTH[@]}" "$PROBE_URL" > /dev/null 2>&1; then
  echo "Error: the model endpoint is not reachable at ${CDR_HOST} (format: ${CDR_API_FORMAT})" >&2
  echo "Start a local model server (ollama serve), or point config/cdr.conf at a" >&2
  echo "model you already run (CDR_API_FORMAT=openai, CDR_ENDPOINT=.../v1/chat/completions)." >&2
  exit 1
fi

# Read the template structure for the chosen type
SKILL_TYPE=$("$PY" -c "import json,sys; print(json.load(open(sys.argv[1]))['type'])" "$INPUT_FILE")
TEMPLATE_FILE="$REPO_ROOT/templates/$SKILL_TYPE/SKILL.md"

if [[ ! -f "$TEMPLATE_FILE" ]]; then
  echo "Error: Template not found: $TEMPLATE_FILE" >&2
  exit 1
fi

# Build template structure description from actual template
TEMPLATE_STRUCTURE=$(sed 's/{{SKILL_NAME}}/[skill-name]/g; s/{{SKILL_TITLE}}/[Skill Title]/g; s/TODO — //g; s/TODO//g' "$TEMPLATE_FILE")

# System prompt
SYSTEM_PROMPT="You are a technical documentation writer for OpenClaw agent skills.
You write clear, practical reference documents that help AI agents perform specific tasks.

You will receive a JSON object with:
- name: the skill identifier (lowercase, hyphens)
- type: the template type (cli-tool, workflow, or language-ref)
- description: what the skill should teach
- commands: specific commands to include (may be empty — generate appropriate ones)
- tips: specific tips to include (may be empty — generate appropriate ones)

Generate a complete SKILL.md following this exact structure:

${TEMPLATE_STRUCTURE}

Rules:
- Start with valid YAML frontmatter: name, version: 1.0.0, description (one sentence, 50-200 chars, starts with action verb), metadata: {}
- The frontmatter must be between --- delimiters on their own lines
- Include a \"When to Use\" section with 3-5 trigger scenarios as bullet points
- All code blocks must use bash fencing (or appropriate language)
- Only include commands and syntax you are confident are correct
- If unsure about exact flags, describe the operation conceptually instead
- Every section must have real content, not placeholder text
- Output ONLY the complete SKILL.md content — no surrounding explanation, no markdown fences around the whole output"

# Build the Ollama request and get response
RESPONSE=$("$PY" -c "
import json, sys, urllib.request

# Read input
with open(sys.argv[1]) as f:
    user_input = json.load(f)

system_prompt = sys.argv[2]
model = sys.argv[3]
endpoint = sys.argv[4]
timeout = int(sys.argv[5])
error_feedback = sys.argv[6] if len(sys.argv) > 6 and sys.argv[6] else ''

# Build user prompt
if error_feedback:
    user_prompt = json.dumps(user_input, indent=2)
    user_prompt += '\n\nThe previous draft failed verification with these errors:\n'
    user_prompt += error_feedback
    user_prompt += '\n\nRewrite the complete SKILL.md fixing these issues. Keep the same structure and content, only fix what is broken.'
else:
    user_prompt = json.dumps(user_input, indent=2)

# Build the request per protocol. Output is markdown (a SKILL.md), so the
# OpenAI branch does NOT request json mode.
api_format = sys.argv[7] if len(sys.argv) > 7 else 'ollama'
api_key = sys.argv[8] if len(sys.argv) > 8 else ''

if api_format == 'openai':
    request = {
        'model': model,
        'messages': [
            {'role': 'system', 'content': system_prompt},
            {'role': 'user', 'content': user_prompt},
        ],
        'stream': False,
    }
else:
    request = {
        'model': model,
        'system': system_prompt,
        'prompt': user_prompt,
        'stream': False,
    }

headers = {'Content-Type': 'application/json'}
if api_key:
    headers['Authorization'] = 'Bearer ' + api_key
req = urllib.request.Request(endpoint, data=json.dumps(request).encode(), headers=headers)

try:
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        result = json.loads(resp.read())
except Exception as e:
    print(f'Error: model request failed: {e}', file=sys.stderr)
    sys.exit(1)

# Extract response text per protocol
if api_format == 'openai':
    try:
        text = result['choices'][0]['message']['content']
    except (KeyError, IndexError, TypeError):
        print('Error: OpenAI-compatible response missing choices[0].message.content', file=sys.stderr)
        sys.exit(1)
else:
    text = result.get('response', '')
if not text.strip():
    print('Error: model returned empty response', file=sys.stderr)
    sys.exit(1)

# Strip markdown fences if the LLM wrapped the output
text = text.strip()
if text.startswith('\`\`\`markdown'):
    text = text[len('\`\`\`markdown'):].strip()
if text.startswith('\`\`\`'):
    text = text[3:].strip()
if text.endswith('\`\`\`'):
    text = text[:-3].strip()

print(text)
" "$INPUT_FILE" "$SYSTEM_PROMPT" "$CDR_MODEL" "$CDR_ENDPOINT" "$CDR_TIMEOUT" "$ERROR_FEEDBACK" "$CDR_API_FORMAT" "$CDR_API_KEY") || {
  echo "Error: Draft generation failed" >&2
  exit 1
}

# Write to output path
echo "$RESPONSE" > "$OUTPUT_PATH"
