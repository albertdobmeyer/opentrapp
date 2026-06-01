#!/usr/bin/env bash
# Tests for the egress advisor (adaptive containment, M3).
# Deterministic — no model/network needed (the advisor is heuristic; the
# off-allowlist gray-zone judgment is the judge's job, tested separately).
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ADV="$SCRIPT_DIR/egress-advisor.sh"
PASS=0; FAIL=0
pass() { echo -e "  \033[0;32mPASS\033[0m  $1"; PASS=$((PASS+1)); }
fail() { echo -e "  \033[0;31mFAIL\033[0m  $1"; FAIL=$((FAIL+1)); }

TMP=$(mktemp -d); trap "rm -rf $TMP" EXIT

# Synthetic egress logs.
cat > "$TMP/core-only.jsonl" <<'EOF'
{"action":"ALLOWED","method":"POST","host":"api.anthropic.com","url":"..."}
{"action":"ALLOWED","method":"POST","host":"api.telegram.org","url":"..."}
EOF
cat > "$TMP/with-skills.jsonl" <<'EOF'
{"action":"ALLOWED","method":"POST","host":"api.anthropic.com","url":"..."}
{"action":"ALLOWED","method":"GET","host":"raw.githubusercontent.com","url":"..."}
EOF
cat > "$TMP/with-web.jsonl" <<'EOF'
{"action":"ALLOWED","method":"GET","host":"some-random-site.example.com","url":"..."}
EOF

prop() { "$ADV" --log "$1" --shell "$2" | python3 -c "import sys,json;print(json.load(sys.stdin)['proposal'])"; }

echo ""
echo "=== Egress advisor tests ==="

# 1. soft + only core hosts → propose tighten to hard.
[[ "$(prop "$TMP/core-only.jsonl" soft)" == "tighten_to_hard" ]] \
  && pass "soft shell, only core hosts → tighten_to_hard" \
  || fail "soft + core-only did not propose tighten_to_hard"

# 2. soft + skill-fetch host → propose tighten to split.
[[ "$(prop "$TMP/with-skills.jsonl" soft)" == "tighten_to_split" ]] \
  && pass "soft shell, skill-fetch host → tighten_to_split" \
  || fail "soft + skills did not propose tighten_to_split"

# 3. split + only core → tighten to hard.
[[ "$(prop "$TMP/core-only.jsonl" split)" == "tighten_to_hard" ]] \
  && pass "split shell, only core hosts → tighten_to_hard" \
  || fail "split + core-only did not propose tighten_to_hard"

# 4. THE INVARIANT — hard shell + diverse web traffic (observed needs 'soft',
#    looser than current) must NEVER propose loosening.
p=$(prop "$TMP/with-web.jsonl" hard)
[[ "$p" == "no_change" ]] \
  && pass "INVARIANT: hard shell + web traffic → no_change (never auto-loosen)" \
  || fail "INVARIANT VIOLATED: hard + web proposed '$p' (a loosening!)"

# 5. Across every (current, log) combination, the proposal is never a loosening.
ok=true
for log in core-only with-skills with-web; do
  for sh in hard split soft; do
    p=$(prop "$TMP/$log.jsonl" "$sh")
    case "$p" in
      tighten_to_soft) [[ "$sh" == soft ]] || ok=false ;;  # 'tighten' to soft only valid if already soft (=no-op); never from tighter
      tighten_to_split) [[ "$sh" == soft ]] || ok=false ;; # split is tighter than soft only
      tighten_to_hard)  : ;;                                # hard is the tightest — always a valid tighten
      no_change) : ;;
      *) ok=false ;;
    esac
  done
done
$ok && pass "no (shell,log) combination ever proposes a looser shell" \
    || fail "some combination proposed a loosening"

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]]
