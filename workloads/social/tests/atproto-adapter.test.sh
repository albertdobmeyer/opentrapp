#!/usr/bin/env bash
# AT Protocol adapter tests (v0.6 Item C, SD-C1).
#
# Network-free + Ollama-free: the normaliser is tested against a fixture; the
# adapter's offline-graceful + auth-required behaviours are tested against an
# unreachable AppView (connection refused, fast). A real live read is a separate
# host-only smoke (see the plan), not part of CI.
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCIAL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ADAPTERS_DIR="$SOCIAL_ROOT/tools/lib/adapters"
ADAPTER="$ADAPTERS_DIR/atproto.sh"
NORMALISE="$ADAPTERS_DIR/atproto_normalise.py"
FIREWALL="$SOCIAL_ROOT/tools/semantic-firewall.sh"
FIXTURE="$SCRIPT_DIR/fixtures/atproto-authorfeed.json"

# An unreachable AppView so reads fail fast (connection refused on port 1).
UNREACHABLE="http://127.0.0.1:1"

PASS=0; FAIL=0
pass() { echo -e "  \033[0;32mPASS\033[0m  $1"; PASS=$((PASS+1)); }
fail() { echo -e "  \033[0;31mFAIL\033[0m  $1"; FAIL=$((FAIL+1)); }

echo ""
echo "=== AT Protocol adapter tests (Item C) ==="
echo ""

# ── 1. normaliser: getAuthorFeed → canonical {id,author,content,timestamp} ───
norm=$(python3 "$NORMALISE" < "$FIXTURE" 2>/dev/null || echo '[]')
if echo "$norm" | python3 -c "
import sys, json
posts = json.load(sys.stdin)
assert len(posts) == 2, f'expected 2 posts, got {len(posts)}'
a, b = posts
assert a['id'] == 'at://did:plc:abc123/app.bsky.feed.post/3kxyz1', a['id']
assert a['author'] == 'alice.bsky.social', a['author']
assert 'consensus algorithms' in a['content'], a['content']
assert a['timestamp'] == '2026-05-30T10:00:00.000Z', a['timestamp']
# author falls back to handle even without displayName
assert b['author'] == 'bob.bsky.social', b['author']
assert set(a.keys()) == {'id','author','content','timestamp'}, a.keys()
" 2>/dev/null; then
  pass "normaliser maps a getAuthorFeed response to the canonical post shape"
else
  fail "normaliser did not produce the canonical {id,author,content,timestamp} shape"
fi

# ── 2. name ─────────────────────────────────────────────────────────────────
if [[ "$(bash "$ADAPTER" name 2>/dev/null)" == "atproto" ]]; then
  pass "adapter reports name 'atproto'"
else
  fail "adapter name is not 'atproto'"
fi

# ── 3. dispatch implements every contract verb ──────────────────────────────
verbs_ok=true
for verb in fetch_feed fetch_agent post stats name; do
  grep -qE "^\s*${verb}\)" "$ADAPTER" || { verbs_ok=false; break; }
done
if $verbs_ok; then
  pass "dispatch implements all five contract verbs (fetch_feed/fetch_agent/post/stats/name)"
else
  fail "adapter dispatch is missing a contract verb"
fi

# ── 4. offline-graceful: unreachable AppView → [] + non-zero (no hang) ───────
out=$(ATPROTO_APPVIEW="$UNREACHABLE" bash "$ADAPTER" fetch_agent "alice.bsky.social" 2>/dev/null); rc=$?
if [[ "$(echo "$out" | tr -d '[:space:]')" == "[]" && $rc -ne 0 ]]; then
  pass "fetch_agent against an unreachable AppView returns [] + non-zero (degrades, no crash)"
else
  fail "fetch_agent offline behaviour wrong (out='$out' rc=$rc)"
fi

# ── 5. fetch_feed with neither actor nor feed → error + [] ──────────────────
out=$(ATPROTO_APPVIEW="$UNREACHABLE" bash "$ADAPTER" fetch_feed '{}' 2>/dev/null); rc=$?
if [[ "$(echo "$out" | tr -d '[:space:]')" == "[]" && $rc -ne 0 ]]; then
  pass "fetch_feed without actor/feed returns [] + non-zero (no silent empty success)"
else
  fail "fetch_feed-without-target behaviour wrong (out='$out' rc=$rc)"
fi

# ── 6. post without credentials → error + exit 1 (never a silent no-op) ──────
err=$(ATPROTO_HANDLE="" ATPROTO_APP_PASSWORD="" bash "$ADAPTER" post "hello world" 2>&1); rc=$?
if [[ $rc -eq 1 && "$err" == *"required to post"* ]]; then
  pass "post without credentials fails closed (exit 1 + clear error)"
else
  fail "post-without-creds should exit 1 with an error (rc=$rc err='$err')"
fi

# ── 7. the firewall no longer hard-gates to file-only ───────────────────────
out=$(ATPROTO_APPVIEW="$UNREACHABLE" bash "$FIREWALL" --adapter atproto --actor "alice.bsky.social" 2>&1); rc=$?
if [[ "$out" != *"not yet wired"* && $rc -eq 0 && "$out" == *"No posts fetched"* ]]; then
  pass "semantic-firewall accepts --adapter atproto and degrades cleanly offline"
else
  fail "firewall did not accept the atproto adapter cleanly (rc=$rc out='$out')"
fi

echo ""
echo "atproto-adapter: ${PASS} passed, ${FAIL} failed"
[[ $FAIL -eq 0 ]]
