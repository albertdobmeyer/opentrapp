#!/usr/bin/env bash
# Read Hum's Telegram conversation from session transcripts.
#
# Usage: bash scripts/read-chat.sh [options]
#   --last N         Show last N messages (default: 20)
#   --tool-calls     Include tool call details (commands, file ops)
#   --since TIME     Show messages after TIME (e.g., "2026-03-27T10:00")
#   --all            Show all messages (no limit)
#
# Security: output is sanitized to prevent terminal injection from
# crafted content in session transcripts.

set -uo pipefail

RUNTIME="podman"
command -v podman &>/dev/null || RUNTIME="docker"
CONTAINER="opencli-container"

# Parse arguments
LAST=20
SHOW_TOOLS=false
SINCE=""
SHOW_ALL=false

while [ $# -gt 0 ]; do
    case "$1" in
        --last)    LAST="${2:-20}"; shift 2 ;;
        --tool-calls) SHOW_TOOLS=true; shift ;;
        --since)   SINCE="${2:-}"; shift 2 ;;
        --all)     SHOW_ALL=true; shift ;;
        *)         shift ;;
    esac
done

$RUNTIME exec "$CONTAINER" sh -c "cat /home/vault/.openclaw/agents/main/sessions/*.jsonl 2>/dev/null" | python3 -c "
import sys, json

last = int(sys.argv[1])
show_tools = sys.argv[2] == 'true'
since = sys.argv[3]
show_all = sys.argv[4] == 'true'

def sanitize(s):
    \"\"\"Strip control characters to prevent terminal injection.\"\"\"
    if not isinstance(s, str):
        return str(s)
    return ''.join(c if (c >= ' ' and c != chr(127)) or c in ('\t', '\n') else '?' for c in s)

def extract_text(content):
    if isinstance(content, str):
        return content.strip()
    if isinstance(content, list):
        parts = []
        for block in content:
            if isinstance(block, dict) and block.get('type') == 'text':
                parts.append(block.get('text', '').strip())
        return '\n'.join(parts)
    return ''

messages = []
for line in sys.stdin:
    try:
        entry = json.loads(line.strip())
        etype = entry.get('type', '')

        if etype != 'message':
            continue

        msg = entry.get('message', {})
        role = msg.get('role', '')
        content = msg.get('content', '')
        ts = entry.get('timestamp', '')[:19]

        # Since filter
        if since and ts < since:
            continue

        # User messages
        if role == 'user':
            text = extract_text(content)
            if not text:
                continue
            # Skip heartbeats
            if 'HEARTBEAT' in text[:50] or text.startswith('Read HEARTBEAT'):
                continue
            # Clean untrusted metadata prefix
            if 'untrusted metadata' in text[:100]:
                parts = text.split('\n\n', 1)
                text = parts[-1] if len(parts) > 1 else text
            messages.append((ts, '\033[94mUSER\033[0m', sanitize(text)))

        # Assistant messages
        elif role == 'assistant':
            text = extract_text(content)
            if text:
                messages.append((ts, '\033[93mHUM\033[0m', sanitize(text)))

            # Show tool calls if requested
            if show_tools and isinstance(content, list):
                for block in content:
                    if isinstance(block, dict) and block.get('type') in ('toolCall', 'tool_use'):
                        name = sanitize(block.get('name', '?'))
                        args = block.get('arguments', {}) or block.get('input', {})
                        if isinstance(args, dict):
                            # Show key details depending on tool type
                            if name in ('exec', 'bash', 'Bash'):
                                detail = sanitize(args.get('command', ''))
                            elif name in ('read', 'write', 'edit', 'apply_patch'):
                                detail = sanitize(args.get('file_path', '') or args.get('path', ''))
                            elif name == 'memory_search':
                                detail = sanitize(args.get('query', ''))
                            else:
                                detail = sanitize(str(args)[:200])
                        else:
                            detail = ''
                        messages.append((ts, '\033[96mTOOL\033[0m', f'{name}: {detail}'))

        # Tool results
        elif role == 'toolResult':
            if show_tools:
                tool_name = sanitize(msg.get('toolName', '?'))
                is_error = msg.get('isError', False)
                result_text = extract_text(msg.get('content', ''))
                status = '\033[31mERROR\033[0m' if is_error else 'ok'
                # Truncate tool results to keep output readable
                display = sanitize(result_text[:200])
                if len(result_text) > 200:
                    display += '...'
                messages.append((ts, '\033[96mRESULT\033[0m', f'{tool_name} ({status}): {display}'))

    except:
        pass

# Apply limit
if show_all:
    display = messages
else:
    display = messages[-last:]

for ts, prefix, text in display:
    print(f'[{ts}] {prefix}: {text}')
    print()
" "$LAST" "$SHOW_TOOLS" "$SINCE" "$SHOW_ALL" 2>&1
