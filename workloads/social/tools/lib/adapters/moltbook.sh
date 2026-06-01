#!/usr/bin/env bash
# Moltbook protocol adapter (archival — Moltbook API has been defunct since
# 2026-04-05).  All Moltbook-specific HTTP coupling lives here and nowhere
# else; the scanner core is protocol-agnostic.
#
# Adapter contract (spec 04 §2a):
#   fetch_feed  <opts-json>           -> JSON array of normalised posts
#   fetch_agent <handle>              -> JSON array of normalised posts
#   post        <content>             -> exits 0 on success
#   stats       <opts-json>           -> JSON object with platform counts
#
# Every returned post has the shape: {id, author, content, timestamp}
#
# opts-json keys for fetch_feed: limit (int, default 50)
#
# Environment (read from caller's env):
#   MOLTBOOK_API_BASE  default: https://api.moltbook.com
#   MOLTBOOK_API_KEY   optional bearer token
set -uo pipefail

ADAPTER_NAME="moltbook"
MOLTBOOK_API_BASE="${MOLTBOOK_API_BASE:-https://api.moltbook.com}"
MOLTBOOK_API_KEY="${MOLTBOOK_API_KEY:-}"

# ── normalise ──────────────────────────────────────────────────────────────
# Convert Moltbook's native post shape to the canonical {id,author,content,timestamp}.
_normalise() {
  python3 -c "
import sys, json
raw = json.load(sys.stdin)
posts = raw if isinstance(raw, list) else raw.get('posts', raw.get('data', []))
out = []
for p in posts:
    out.append({
        'id':        p.get('id', ''),
        'author':    p.get('agent_handle', p.get('handle', 'unknown')),
        'content':   p.get('content', ''),
        'timestamp': p.get('created_at', p.get('timestamp', '')),
    })
print(json.dumps(out))
"
}

# ── fetch_feed ─────────────────────────────────────────────────────────────
_fetch_feed() {
  local opts_json="${1:-}"
  [[ -z "$opts_json" ]] && opts_json='{}'
  local limit
  limit=$(python3 -c "import json,sys; d=json.loads('''${opts_json}'''); print(d.get('limit',50))" 2>/dev/null || echo '50')

  local curl_args=(-sf "${MOLTBOOK_API_BASE}/posts?limit=${limit}")
  if [[ -n "$MOLTBOOK_API_KEY" ]]; then
    curl_args+=(-H "Authorization: Bearer ${MOLTBOOK_API_KEY}")
  fi

  local raw
  if ! raw=$(curl "${curl_args[@]}" 2>/dev/null); then
    echo "[]"
    return 1
  fi
  echo "$raw" | _normalise
}

# ── fetch_agent ────────────────────────────────────────────────────────────
_fetch_agent() {
  local handle="${1:?fetch_agent requires a handle}"

  local curl_args=(-sf "${MOLTBOOK_API_BASE}/agents/${handle}/posts")
  if [[ -n "$MOLTBOOK_API_KEY" ]]; then
    curl_args+=(-H "Authorization: Bearer ${MOLTBOOK_API_KEY}")
  fi

  local raw
  if ! raw=$(curl "${curl_args[@]}" 2>/dev/null); then
    echo "[]"
    return 1
  fi
  echo "$raw" | _normalise
}

# ── post ───────────────────────────────────────────────────────────────────
_post_content() {
  local content="${1:?post requires content}"

  if [[ -z "$MOLTBOOK_API_KEY" ]]; then
    echo "ERROR: MOLTBOOK_API_KEY required to post" >&2
    return 1
  fi

  curl -sf -X POST \
    -H "Authorization: Bearer ${MOLTBOOK_API_KEY}" \
    -H "Content-Type: application/json" \
    -d "{\"content\": \"${content}\"}" \
    "${MOLTBOOK_API_BASE}/posts" > /dev/null 2>&1
}

# ── stats ──────────────────────────────────────────────────────────────────
_stats() {
  local raw=""
  if raw=$(curl -sf "${MOLTBOOK_API_BASE}/stats" 2>/dev/null); then
    python3 -c "
import sys, json
d = json.loads('''$raw''')
print(json.dumps({
    'agents':   d.get('agents', d.get('total_agents', d.get('agent_count', None))),
    'posts':    d.get('posts',  d.get('total_posts',  d.get('post_count',  None))),
    'comments': d.get('comments', d.get('total_comments', d.get('comment_count', None))),
    'source':   'moltbook',
}))
" 2>/dev/null || echo '{"agents":null,"posts":null,"comments":null,"source":"moltbook"}'
  else
    echo '{"agents":null,"posts":null,"comments":null,"source":"moltbook","error":"api_unreachable"}'
  fi
}

# ── dispatch ───────────────────────────────────────────────────────────────
CMD="${1:-}"
_OPTS="${2:-}"; [[ -z "$_OPTS" ]] && _OPTS='{}'
case "$CMD" in
  fetch_feed)  _fetch_feed  "$_OPTS" ;;
  fetch_agent) _fetch_agent "${2:?}" ;;
  post)        _post_content "${2:?}" ;;
  stats)       _stats ;;
  name)        echo "$ADAPTER_NAME" ;;
  *)
    echo "Usage: moltbook.sh <fetch_feed|fetch_agent|post|stats|name> [arg]" >&2
    exit 2
    ;;
esac
