#!/usr/bin/env bash
# File protocol adapter — reads posts from a local JSON file.
#
# Aligns with the existing --file / --adapter file seam already present in
# semantic-firewall.sh. The scanner core delegates all I/O to this adapter
# when --adapter file is used, so no filesystem logic lives in the core.
#
# Adapter contract (spec 04 §2a):
#   fetch_feed  <opts-json>    -> JSON array of normalised posts
#   fetch_agent <handle>       -> JSON array of normalised posts (filtered)
#   post        <content>      -> exits 1 (file adapter is read-only)
#   stats       <opts-json>    -> JSON object derived from the file
#
# opts-json key: path (string, required for fetch_feed / stats)
#
# Every returned post has the shape: {id, author, content, timestamp}
set -uo pipefail

ADAPTER_NAME="file"

# ── normalise ──────────────────────────────────────────────────────────────
# Accept the two Moltbook-era field names (agent_handle, handle) as well as the
# canonical 'author' so the adapter works with both legacy fixtures and new data.
_normalise_file() {
  local path="$1"
  python3 -c "
import sys, json
with open('${path}') as fh:
    raw = json.load(fh)
posts = raw if isinstance(raw, list) else raw.get('posts', raw.get('data', []))
out = []
for p in posts:
    out.append({
        'id':        p.get('id', ''),
        'author':    p.get('author', p.get('agent_handle', p.get('handle', 'unknown'))),
        'content':   p.get('content', ''),
        'timestamp': p.get('timestamp', p.get('created_at', '')),
    })
print(json.dumps(out))
"
}

# ── fetch_feed ─────────────────────────────────────────────────────────────
_fetch_feed() {
  local opts_json="${1:-}"
  [[ -z "$opts_json" ]] && opts_json='{}'
  local path
  path=$(python3 -c "import json,sys; d=json.loads(r'''${opts_json}'''); print(d.get('path',''))" 2>/dev/null || echo '')

  if [[ -z "$path" || ! -f "$path" ]]; then
    echo "ERROR: file adapter fetch_feed requires opts.path pointing to an existing file (got: '${path}')" >&2
    echo "[]"
    return 1
  fi

  local limit
  limit=$(python3 -c "import json,sys; d=json.loads(r'''${opts_json}'''); print(d.get('limit',9999))" 2>/dev/null || echo '9999')

  _normalise_file "$path" | python3 -c "
import sys, json
posts = json.load(sys.stdin)
limit = int('${limit}')
print(json.dumps(posts[:limit]))
"
}

# ── fetch_agent ────────────────────────────────────────────────────────────
_fetch_agent() {
  local handle="${1:?fetch_agent requires a handle}"
  # Second arg can be path override; fall back to SOCIAL_FEED_FILE env if set.
  local path="${2:-${SOCIAL_FEED_FILE:-}}"

  if [[ -z "$path" || ! -f "$path" ]]; then
    echo "ERROR: file adapter fetch_agent requires a file path (second arg or SOCIAL_FEED_FILE)" >&2
    echo "[]"
    return 1
  fi

  _normalise_file "$path" | python3 -c "
import sys, json
posts = json.load(sys.stdin)
handle = '${handle}'
print(json.dumps([p for p in posts if p.get('author','') == handle]))
"
}

# ── post ───────────────────────────────────────────────────────────────────
_post_content() {
  echo "ERROR: file adapter is read-only — posting is not supported" >&2
  return 1
}

# ── stats ──────────────────────────────────────────────────────────────────
_stats() {
  local opts_json="${1:-}"
  [[ -z "$opts_json" ]] && opts_json='{}'
  local path
  path=$(python3 -c "import json,sys; d=json.loads(r'''${opts_json}'''); print(d.get('path',''))" 2>/dev/null || echo '')

  if [[ -z "$path" || ! -f "$path" ]]; then
    echo '{"agents":null,"posts":null,"comments":null,"source":"file","error":"no_path"}'
    return 1
  fi

  python3 -c "
import json
with open('${path}') as fh:
    raw = json.load(fh)
posts = raw if isinstance(raw, list) else raw.get('posts', raw.get('data', []))
stats_raw = raw.get('stats', {}) if isinstance(raw, dict) else {}
authors = set(p.get('author', p.get('agent_handle', p.get('handle', ''))) for p in posts)
print(json.dumps({
    'agents':   stats_raw.get('agents', len(authors)),
    'posts':    stats_raw.get('posts', len(posts)),
    'comments': stats_raw.get('comments', 0),
    'source':   'file',
    'path':     '${path}',
}))
"
}

# ── dispatch ───────────────────────────────────────────────────────────────
CMD="${1:-}"
_OPTS="${2:-}"; [[ -z "$_OPTS" ]] && _OPTS='{}'
case "$CMD" in
  fetch_feed)  _fetch_feed  "$_OPTS" ;;
  fetch_agent) _fetch_agent "${2:?}" "${3:-}" ;;
  post)        _post_content "${2:-}" ;;
  stats)       _stats "$_OPTS" ;;
  name)        echo "$ADAPTER_NAME" ;;
  *)
    echo "Usage: file.sh <fetch_feed|fetch_agent|post|stats|name> [arg]" >&2
    exit 2
    ;;
esac
