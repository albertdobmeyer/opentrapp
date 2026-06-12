#!/usr/bin/env bash
# =============================================================================
# OpenTrApp Boundary Self-Test  (road-to-recommendable.md §1A / §1B)
# =============================================================================
# Proves that the live five-container perimeter actually HOLDS its security
# boundaries — at the consumption end, on real hardware (CLAUDE.md §11). This
# is the gate for calling OpenTrApp a security tool, and the contract the
# daemon re-runs on every (re)start so a *resumed* boundary must pass the SAME
# checks as a cold one, fail-closed (ADR-0018 / ADR-0019, tasks #39/#45).
#
# It asserts, against the running perimeter:
#   B1  Network isolation     — vault-agent has NO direct internet route
#   B2  L7 allowlist          — off-allowlist host blocked (403), on-list allowed
#   B3  Credential injection   — the Anthropic/OpenAI key is NOT in the agent
#   B4  L3 egress filter       — vault-egress nftables drop-private set is loaded
#   B5  Proxy CA pinned        — the agent-trusted CA fingerprint is unchanged
#   B6  No host-side untrusted — skill delivery is read-only into the agent;
#                                untrusted work runs inside containers
#
# Exit status (fail-closed — the daemon gates on this):
#   0  all boundaries PASS
#   1  one or more boundaries FAILED  → the perimeter must hold closed + alert
#   2  could not assess (perimeter not running / tool missing) → NOT "pass".
#      Unverifiable is not verified.
#
# Usage:
#   bash tests/boundary-selftest.sh                 # assess (resume/default mode)
#   bash tests/boundary-selftest.sh --record-baseline  # cold start: pin the CA fingerprint
#   bash tests/boundary-selftest.sh --json          # machine-readable summary line
#
# This box can author + lint this script but CANNOT run it (it swap-storms
# running the full perimeter). Run it on capable hardware with the perimeter up
# (`make perimeter-up`). The first real run shakes out per-tool availability in
# the alpine-based agent image; commands below auto-detect wget/curl.
# =============================================================================
set -uo pipefail

# ── configuration ────────────────────────────────────────────────────────────
AGENT="${OPENTRAPP_AGENT_CTR:-vault-agent}"
PROXY="${OPENTRAPP_PROXY_CTR:-vault-proxy}"
EGRESS="${OPENTRAPP_EGRESS_CTR:-vault-egress}"
DATA_DIR="${OPENTRAPP_DATA_DIR:-$HOME/.opentrapp}"
BASELINE_DIR="$DATA_DIR/boundary"
CA_BASELINE="$BASELINE_DIR/ca-fingerprint.expected"
# A public IP with no DNS dependency — the no-route test must fail to reach it.
PROBE_IP="1.1.1.1"
# An on-allowlist host (the agent's vendor API) and a host that must NOT be on it.
ONLIST_HOST="${OPENTRAPP_ONLIST_HOST:-api.anthropic.com}"
OFFLIST_HOST="${OPENTRAPP_OFFLIST_HOST:-example.org}"
# Paths inside the containers (from compose.yml).
AGENT_CA="/opt/proxy-ca/mitmproxy-ca-cert.pem"
RUNTIME="${OPENTRAPP_RUNTIME:-}"   # podman|docker; auto-detected if empty

JSON=0
RECORD_BASELINE=0
for arg in "$@"; do
  case "$arg" in
    --json) JSON=1 ;;
    --record-baseline) RECORD_BASELINE=1 ;;
    -h | --help) sed -n '2,40p' "$0"; exit 0 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

# ── output helpers ───────────────────────────────────────────────────────────
PASS=0; FAIL=0; SKIP=0
FAILED_NAMES=""
c_ok=$'\033[32m'; c_no=$'\033[31m'; c_sk=$'\033[33m'; c_z=$'\033[0m'
[ -t 1 ] || { c_ok=""; c_no=""; c_sk=""; c_z=""; }

pass() { PASS=$((PASS + 1)); printf "  ${c_ok}PASS${c_z}  %-22s %s\n" "$1" "${2:-}"; }
fail() { FAIL=$((FAIL + 1)); FAILED_NAMES="$FAILED_NAMES $1"; printf "  ${c_no}FAIL${c_z}  %-22s %s\n" "$1" "${2:-}"; }
skip() { SKIP=$((SKIP + 1)); printf "  ${c_sk}SKIP${c_z}  %-22s %s\n" "$1" "${2:-}"; }

# ── runtime + liveness ───────────────────────────────────────────────────────
detect_runtime() {
  if [ -n "$RUNTIME" ]; then return 0; fi
  if command -v podman >/dev/null 2>&1; then RUNTIME=podman
  elif command -v docker >/dev/null 2>&1; then RUNTIME=docker
  else echo "ERROR: neither podman nor docker found" >&2; exit 2; fi
}
ctr_running() { $RUNTIME inspect -f '{{.State.Running}}' "$1" 2>/dev/null | grep -qx true; }
xa() { $RUNTIME exec "$AGENT" "$@"; }     # exec in agent
xe() { $RUNTIME exec "$EGRESS" "$@"; }    # exec in egress
xp() { $RUNTIME exec "$PROXY" "$@"; }     # exec in proxy

# Pick an available HTTP client inside the agent (alpine busybox wget / curl).
agent_http_tool() {
  if xa sh -c 'command -v curl' >/dev/null 2>&1; then echo curl
  elif xa sh -c 'command -v wget' >/dev/null 2>&1; then echo wget
  else echo none; fi
}

# ── the checks ───────────────────────────────────────────────────────────────

# B1 — Network isolation: the agent network is internal:true (no gateway), so a
# DIRECT (proxy-bypassed) connection to a public IP must fail to route.
check_isolation() {
  local tool out
  tool="$(agent_http_tool)"
  if [ "$tool" = none ]; then skip "B1-isolation" "no http tool in $AGENT"; return; fi
  # Unset proxy env so we test the raw route, not the proxied path.
  if [ "$tool" = curl ]; then
    out="$(xa sh -c "env -u HTTP_PROXY -u HTTPS_PROXY -u http_proxy -u https_proxy \
      curl -sS --max-time 6 http://$PROBE_IP/ -o /dev/null -w '%{http_code}' 2>&1; echo \" exit=\$?\"")"
  else
    out="$(xa sh -c "env -u HTTP_PROXY -u HTTPS_PROXY -u http_proxy -u https_proxy \
      wget -T 6 -q -O /dev/null http://$PROBE_IP/ 2>&1; echo \"exit=\$?\"")"
  fi
  # Success (exit=0 / a real HTTP code) would mean the agent reached the internet
  # directly — a boundary breach. Any failure (no route / timeout) is the pass.
  if echo "$out" | grep -qE 'exit=0|^200| 200 '; then
    fail "B1-isolation" "agent reached $PROBE_IP directly → NOT isolated [$out]"
  else
    pass "B1-isolation" "no direct route to $PROBE_IP (proxy-bypass fails)"
  fi
}

# B2 — L7 allowlist: through the proxy, an off-allowlist host returns 403; an
# on-allowlist host is NOT 403 (any other response means the proxy permitted it).
check_allowlist() {
  local tool off on
  tool="$(agent_http_tool)"
  if [ "$tool" = none ]; then skip "B2-allowlist" "no http tool in $AGENT"; return; fi
  status_for() {
    # Uses the agent's configured HTTP_PROXY. Plain http to avoid TLS-trust noise;
    # the proxy applies the allowlist before any upstream connect.
    if [ "$tool" = curl ]; then
      xa sh -c "curl -sS --max-time 8 -o /dev/null -w '%{http_code}' http://$1/ 2>/dev/null"
    else
      xa sh -c "wget -T 8 -S -q -O /dev/null http://$1/ 2>&1 | awk '/HTTP\\//{print \$2; exit}'"
    fi
  }
  off="$(status_for "$OFFLIST_HOST")"
  on="$(status_for "$ONLIST_HOST")"
  if [ "$off" = 403 ]; then
    pass "B2-allowlist-deny" "$OFFLIST_HOST → 403 (blocked)"
  else
    fail "B2-allowlist-deny" "$OFFLIST_HOST → '${off:-no-response}' (expected 403)"
  fi
  if [ -n "$on" ] && [ "$on" != 403 ]; then
    pass "B2-allowlist-allow" "$ONLIST_HOST → '$on' (not blocked)"
  else
    fail "B2-allowlist-allow" "$ONLIST_HOST → '${on:-no-response}' (allowlisted host blocked)"
  fi
}

# B3 — Credential injection: the high-value vendor API key is injected by the
# proxy and must NEVER be present in the agent. NOTE: TELEGRAM_BOT_TOKEN *is*
# legitimately in the agent (OpenClaw polls Telegram itself, compose:69) — so we
# assert ONLY on the Anthropic/OpenAI key, not a blanket token grep.
check_credentials() {
  local hit
  hit="$(xa env 2>/dev/null | grep -iE '^(ANTHROPIC_API_KEY|OPENAI_API_KEY)=' | grep -vE '=$' || true)"
  if [ -z "$hit" ]; then
    pass "B3-credential" "no Anthropic/OpenAI key in $AGENT (proxy-injected)"
  else
    fail "B3-credential" "vendor API key present in $AGENT env (ADR-0001 breach)"
  fi
}

# B4 — L3 egress filter: vault-egress nftables ruleset has the drop-private set
# loaded (same marker the container healthcheck uses).
check_egress_filter() {
  if xe nft list ruleset 2>/dev/null | grep -q 'vault_egress_drop_private'; then
    pass "B4-l3-egress" "nftables drop-private set loaded in $EGRESS"
  else
    fail "B4-l3-egress" "vault_egress_drop_private NOT in $EGRESS ruleset"
  fi
}

# B5 — Proxy CA pinned: the CA the agent trusts must be stable across restarts.
# Cold start (--record-baseline) pins it; resume compares. A silent CA swap is a
# MITM red flag.
check_ca_pinned() {
  local fp
  fp="$(xa sh -c "
    if command -v openssl >/dev/null 2>&1; then
      openssl x509 -in '$AGENT_CA' -noout -fingerprint -sha256 2>/dev/null
    else
      sha256sum '$AGENT_CA' 2>/dev/null
    fi" | tr -d ' ' )"
  if [ -z "$fp" ]; then skip "B5-ca-pinned" "could not read $AGENT_CA in $AGENT"; return; fi
  if [ "$RECORD_BASELINE" = 1 ] || [ ! -f "$CA_BASELINE" ]; then
    mkdir -p "$BASELINE_DIR"; printf '%s\n' "$fp" > "$CA_BASELINE"; chmod 600 "$CA_BASELINE"
    pass "B5-ca-pinned" "baseline recorded ($(printf '%s' "$fp" | tail -c 20))"
    return
  fi
  if [ "$fp" = "$(cat "$CA_BASELINE")" ]; then
    pass "B5-ca-pinned" "CA fingerprint unchanged"
  else
    fail "B5-ca-pinned" "CA fingerprint CHANGED since baseline — possible CA swap"
  fi
}

# B6 — No host-side untrusted content: skills are delivered to the agent
# read-only (compose: skills-deliveries :ro), and untrusted work runs inside
# containers, not host bash. Structural assertion against the live mount.
check_no_host_untrusted() {
  local mode
  mode="$($RUNTIME inspect -f '{{range .Mounts}}{{if eq .Destination "/home/vault/workspace/skills"}}{{.RW}}{{end}}{{end}}' "$AGENT" 2>/dev/null)"
  if [ "$mode" = false ]; then
    pass "B6-no-host-content" "skill delivery is read-only into $AGENT"
  elif [ -z "$mode" ]; then
    skip "B6-no-host-content" "skills-delivery mount not present (skills on-demand?)"
  else
    fail "B6-no-host-content" "skill delivery is writable by $AGENT (RW=$mode)"
  fi
}

# ── run ──────────────────────────────────────────────────────────────────────
detect_runtime
date -u '+%Y-%m-%dT%H:%M:%SZ'
echo "── OpenTrApp boundary self-test ($RUNTIME) ─────────────────────"
[ "$RECORD_BASELINE" = 1 ] && echo "   mode: COLD START (recording CA baseline)" \
                           || echo "   mode: assess (resume / default)"

# Liveness gate: an un-running perimeter is "cannot assess", never "pass".
missing=""
for c in "$AGENT" "$PROXY" "$EGRESS"; do
  ctr_running "$c" || missing="$missing $c"
done
if [ -n "$missing" ]; then
  echo "  ${c_sk}CANNOT ASSESS${c_z} — not running:$missing"
  echo "  Bring the perimeter up first (make perimeter-up). Exit 2."
  exit 2
fi

echo
check_isolation
check_allowlist
check_credentials
check_egress_filter
check_ca_pinned
check_no_host_untrusted
echo

echo "── result ──────────────────────────────────────────────────"
printf "  pass=%d  fail=%d  skip=%d\n" "$PASS" "$FAIL" "$SKIP"
[ "$JSON" = 1 ] && printf '{"pass":%d,"fail":%d,"skip":%d,"failed":"%s"}\n' \
  "$PASS" "$FAIL" "$SKIP" "$(echo "$FAILED_NAMES" | xargs)"

if [ "$FAIL" -gt 0 ]; then
  echo "  ${c_no}BOUNDARY FAILED — perimeter must hold closed + alert (fail-closed).${c_z}"
  exit 1
fi
if [ "$SKIP" -gt 0 ]; then
  echo "  ${c_sk}Some checks could not run — assess unverified, not green.${c_z}"
  exit 2
fi
echo "  ${c_ok}All boundaries hold.${c_z}"
exit 0
