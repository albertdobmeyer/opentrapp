#!/usr/bin/env bash
# AT Protocol (atproto / Bluesky) adapter — the first LIVE network adapter
# (v0.6 Item C, SD-C1). All atproto-specific XRPC coupling lives here; the
# scanner core stays protocol-agnostic.
#
# Adapter contract (spec 04 §2a):
#   fetch_feed  <opts-json>   -> JSON array of normalised posts
#   fetch_agent <handle>      -> JSON array of normalised posts
#   post        <content>     -> exits 0 on success
#   stats       <opts-json>   -> JSON object with platform counts
#   name                      -> "atproto"
#
# Every returned post has the shape: {id, author, content, timestamp}
#
# Reads use the PUBLIC AppView — no auth required (a read-only feed shield):
#   ATPROTO_APPVIEW       default: https://public.api.bsky.app
# Posting requires app-password credentials on the PDS:
#   ATPROTO_PDS           default: https://bsky.social
#   ATPROTO_HANDLE        the posting account's handle
#   ATPROTO_APP_PASSWORD  an app password (NOT the account's main password)
#
# opts-json keys for fetch_feed: actor (handle/DID), feed (AT-URI), limit (int, default 50)
set -uo pipefail

ADAPTER_NAME="atproto"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NORMALISE="$SCRIPT_DIR/atproto_normalise.py"

ATPROTO_APPVIEW="${ATPROTO_APPVIEW:-https://public.api.bsky.app}"
ATPROTO_PDS="${ATPROTO_PDS:-https://bsky.social}"
ATPROTO_HANDLE="${ATPROTO_HANDLE:-}"
ATPROTO_APP_PASSWORD="${ATPROTO_APP_PASSWORD:-}"

_normalise() { python3 "$NORMALISE"; }

# URL-encode a value (handles / DIDs / AT-URIs) safely.
_enc() { python3 -c "import sys,urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=''))" "$1"; }

# GET an XRPC endpoint. Fails fast (connection refused / timeout) so offline
# callers degrade quickly rather than hang.
_get() { curl -sf --connect-timeout 5 --max-time 30 "$1" 2>/dev/null; }

# ── fetch_feed ─────────────────────────────────────────────────────────────
_fetch_feed() {
  local opts_json="${1:-}"; [[ -z "$opts_json" ]] && opts_json='{}'
  local actor feed limit
  actor=$(python3 -c "import json; print(json.loads(r'''${opts_json}''').get('actor',''))" 2>/dev/null || echo '')
  feed=$(python3 -c "import json; print(json.loads(r'''${opts_json}''').get('feed',''))" 2>/dev/null || echo '')
  limit=$(python3 -c "import json; print(json.loads(r'''${opts_json}''').get('limit',50))" 2>/dev/null || echo '50')

  local url raw
  if [[ -n "$feed" ]]; then
    url="${ATPROTO_APPVIEW}/xrpc/app.bsky.feed.getFeed?feed=$(_enc "$feed")&limit=${limit}"
  elif [[ -n "$actor" ]]; then
    url="${ATPROTO_APPVIEW}/xrpc/app.bsky.feed.getAuthorFeed?actor=$(_enc "$actor")&limit=${limit}"
  else
    echo "ERROR: fetch_feed needs 'actor' or 'feed' in opts (atproto has no global timeline without auth)" >&2
    echo "[]"; return 1
  fi
  if ! raw=$(_get "$url"); then echo "[]"; return 1; fi
  echo "$raw" | _normalise
}

# ── fetch_agent ────────────────────────────────────────────────────────────
_fetch_agent() {
  local handle="${1:?fetch_agent requires a handle}"
  local raw
  if ! raw=$(_get "${ATPROTO_APPVIEW}/xrpc/app.bsky.feed.getAuthorFeed?actor=$(_enc "$handle")&limit=50"); then
    echo "[]"; return 1
  fi
  echo "$raw" | _normalise
}

# ── stats ──────────────────────────────────────────────────────────────────
# atproto has no global stats endpoint; derive what we can from the configured
# actor's profile, else return nulls (mirrors moltbook's offline shape).
_stats() {
  local opts_json="${1:-}"; [[ -z "$opts_json" ]] && opts_json='{}'
  local actor
  actor=$(python3 -c "import json; print(json.loads(r'''${opts_json}''').get('actor','${ATPROTO_HANDLE}'))" 2>/dev/null || echo '')
  if [[ -z "$actor" ]]; then
    echo '{"agents":null,"posts":null,"comments":null,"source":"atproto"}'; return 0
  fi
  local raw
  if raw=$(_get "${ATPROTO_APPVIEW}/xrpc/app.bsky.actor.getProfile?actor=$(_enc "$actor")"); then
    echo "$raw" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(json.dumps({
    'agents':    None,
    'posts':     d.get('postsCount'),
    'comments':  None,
    'followers': d.get('followersCount'),
    'source':    'atproto',
}))" 2>/dev/null || echo '{"agents":null,"posts":null,"comments":null,"source":"atproto"}'
  else
    echo '{"agents":null,"posts":null,"comments":null,"source":"atproto","error":"appview_unreachable"}'
  fi
}

# ── post ───────────────────────────────────────────────────────────────────
# Auth required. Exchanges the app password for an access JWT, then creates an
# app.bsky.feed.post record. Credentials are passed via argv (never interpolated
# into a JSON string) so content/passwords can't break out of the request body.
_post_content() {
  local content="${1:?post requires content}"
  if [[ -z "$ATPROTO_HANDLE" || -z "$ATPROTO_APP_PASSWORD" ]]; then
    echo "ERROR: ATPROTO_HANDLE + ATPROTO_APP_PASSWORD required to post" >&2
    return 1
  fi
  local session jwt did
  session=$(curl -sf --connect-timeout 5 --max-time 30 -X POST \
    -H "Content-Type: application/json" \
    -d "$(python3 -c 'import json,sys; print(json.dumps({"identifier":sys.argv[1],"password":sys.argv[2]}))' "$ATPROTO_HANDLE" "$ATPROTO_APP_PASSWORD")" \
    "${ATPROTO_PDS}/xrpc/com.atproto.server.createSession" 2>/dev/null) || {
      echo "ERROR: atproto login failed" >&2; return 1; }
  jwt=$(echo "$session" | python3 -c "import sys,json; print(json.load(sys.stdin).get('accessJwt',''))" 2>/dev/null || echo '')
  did=$(echo "$session" | python3 -c "import sys,json; print(json.load(sys.stdin).get('did',''))" 2>/dev/null || echo '')
  [[ -n "$jwt" && -n "$did" ]] || { echo "ERROR: atproto login returned no session" >&2; return 1; }

  local record
  record=$(python3 -c "
import json, sys, datetime
print(json.dumps({
    'repo': sys.argv[1],
    'collection': 'app.bsky.feed.post',
    'record': {
        '\$type': 'app.bsky.feed.post',
        'text': sys.argv[2],
        'createdAt': datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%S.000Z'),
    },
}))" "$did" "$content")
  curl -sf --connect-timeout 5 --max-time 30 -X POST \
    -H "Authorization: Bearer ${jwt}" \
    -H "Content-Type: application/json" \
    -d "$record" \
    "${ATPROTO_PDS}/xrpc/com.atproto.repo.createRecord" > /dev/null 2>&1
}

# ── dispatch ───────────────────────────────────────────────────────────────
CMD="${1:-}"
_OPTS="${2:-}"; [[ -z "$_OPTS" ]] && _OPTS='{}'
case "$CMD" in
  fetch_feed)  _fetch_feed  "$_OPTS" ;;
  fetch_agent) _fetch_agent "${2:?}" ;;
  post)        _post_content "${2:?}" ;;
  stats)       _stats "$_OPTS" ;;
  name)        echo "$ADAPTER_NAME" ;;
  *)
    echo "Usage: atproto.sh <fetch_feed|fetch_agent|post|stats|name> [arg]" >&2
    exit 2
    ;;
esac
