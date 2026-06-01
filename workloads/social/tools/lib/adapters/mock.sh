#!/usr/bin/env bash
# Mock protocol adapter — deterministic in-memory data for tests.
#
# Adapter contract (spec 04 §2a):
#   fetch_feed  <opts-json>    -> JSON array of normalised posts
#   fetch_agent <handle>       -> JSON array of normalised posts (filtered to handle)
#   post        <content>      -> exits 0 (no-op)
#   stats       <opts-json>    -> JSON object with platform counts
#
# Every returned post has the shape: {id, author, content, timestamp}
# No network, no filesystem, no Moltbook strings.
set -uo pipefail

ADAPTER_NAME="mock"

# Embedded fixture posts — protocol-agnostic, no platform-specific fields.
_ALL_POSTS='[
  {"id":"mock-001","author":"agent-alpha","content":"Discussing distributed system tradeoffs in modern architectures.","timestamp":"2026-05-01T10:00:00Z"},
  {"id":"mock-002","author":"agent-beta","content":"Published a weekly summary of consensus algorithm performance benchmarks.","timestamp":"2026-05-01T11:00:00Z"},
  {"id":"mock-003","author":"agent-gamma","content":"Exploring formal verification methods for concurrent programs.","timestamp":"2026-05-01T12:00:00Z"},
  {"id":"mock-004","author":"agent-alpha","content":"Type theory and its practical applications in language design.","timestamp":"2026-05-01T13:00:00Z"}
]'

# ── fetch_feed ─────────────────────────────────────────────────────────────
_fetch_feed() {
  local opts_json="${1:-}"
  [[ -z "$opts_json" ]] && opts_json='{}'
  local limit
  limit=$(python3 -c "import json,sys; d=json.loads(r'''${opts_json}'''); print(d.get('limit',50))" 2>/dev/null || echo '50')

  python3 -c "
import json
posts = json.loads(r'''${_ALL_POSTS}''')
limit = int('${limit}')
print(json.dumps(posts[:limit]))
"
}

# ── fetch_agent ────────────────────────────────────────────────────────────
_fetch_agent() {
  local handle="${1:?fetch_agent requires a handle}"
  python3 -c "
import json
posts = json.loads(r'''${_ALL_POSTS}''')
filtered = [p for p in posts if p.get('author','') == '${handle}']
print(json.dumps(filtered))
"
}

# ── post ───────────────────────────────────────────────────────────────────
_post_content() {
  # Accepts the post but does nothing (test sink).
  return 0
}

# ── stats ──────────────────────────────────────────────────────────────────
_stats() {
  python3 -c "
import json
posts = json.loads(r'''${_ALL_POSTS}''')
authors = set(p['author'] for p in posts)
print(json.dumps({
    'agents':   len(authors),
    'posts':    len(posts),
    'comments': 0,
    'source':   'mock',
}))
"
}

# ── dispatch ───────────────────────────────────────────────────────────────
CMD="${1:-}"
_OPTS="${2:-}"; [[ -z "$_OPTS" ]] && _OPTS='{}'
case "$CMD" in
  fetch_feed)  _fetch_feed  "$_OPTS" ;;
  fetch_agent) _fetch_agent "${2:?}" ;;
  post)        _post_content "${2:-}" ;;
  stats)       _stats ;;
  name)        echo "$ADAPTER_NAME" ;;
  *)
    echo "Usage: mock.sh <fetch_feed|fetch_agent|post|stats|name> [arg]" >&2
    exit 2
    ;;
esac
