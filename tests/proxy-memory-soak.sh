#!/usr/bin/env bash
# =============================================================================
# OpenTrApp Proxy Memory Soak  (road-to-recommendable.md §2A, tasks #41/#42)
# =============================================================================
# A days-long security tool cannot leak. This samples vault-proxy (mitmproxy)
# resident RSS over (load × time) so growth can be ATTRIBUTED — steady-state vs.
# a leak — at the consumption end, on real hardware (CLAUDE.md §11).
#
# It does two things:
#   1. Samples vault-proxy RSS at a fixed interval for a fixed duration, printing
#      a time series + a growth attribution (baseline / peak / final / MB-per-hour
#      slope / verdict).
#   2. Optionally drives synthetic request load THROUGH the proxy (from inside the
#      perimeter) so the soak isn't idle. For a realistic soak, prefer leaving a
#      real agent running and use --load off (sample only).
#
# Usage:
#   bash tests/proxy-memory-soak.sh                       # 120 min, 30s interval, light load
#   bash tests/proxy-memory-soak.sh --duration 360 --interval 60
#   bash tests/proxy-memory-soak.sh --load off            # sample only (real agent drives load)
#   bash tests/proxy-memory-soak.sh --load heavy --json
#
# Exit:  0 sampled OK · 1 growth exceeds the leak threshold · 2 cannot assess.
#
# The dev box can author + lint this but CANNOT run a meaningful soak (it
# swap-storms running the perimeter). Run on capable hardware with the perimeter
# up (`make perimeter-up`).
# =============================================================================
set -uo pipefail

PROXY="${OPENTRAPP_PROXY_CTR:-vault-proxy}"
DRIVER="${OPENTRAPP_LOAD_CTR:-vault-agent}"   # container used to drive load through the proxy
DURATION_MIN=120
INTERVAL_SEC=30
LOAD=light            # off | light | heavy
JSON=0
# A leak verdict: net growth above this (MB) over the run, AND a positive slope,
# is flagged. Tune per platform once a real baseline exists.
LEAK_MB_THRESHOLD="${OPENTRAPP_LEAK_MB:-64}"
# An allowlisted host to hit when driving load (must be on infra/proxy/allowlist.txt).
LOAD_HOST="${OPENTRAPP_LOAD_HOST:-api.anthropic.com}"
RUNTIME="${OPENTRAPP_RUNTIME:-}"

while [ $# -gt 0 ]; do
  case "$1" in
    --duration) DURATION_MIN="$2"; shift 2 ;;
    --interval) INTERVAL_SEC="$2"; shift 2 ;;
    --load) LOAD="$2"; shift 2 ;;
    --json) JSON=1; shift ;;
    -h | --help) sed -n '2,34p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

detect_runtime() {
  if [ -n "$RUNTIME" ]; then return 0; fi
  if command -v podman >/dev/null 2>&1; then RUNTIME=podman
  elif command -v docker >/dev/null 2>&1; then RUNTIME=docker
  else echo "ERROR: neither podman nor docker found" >&2; exit 2; fi
}
ctr_running() { $RUNTIME inspect -f '{{.State.Running}}' "$1" 2>/dev/null | grep -qx true; }

# vault-proxy RSS in MB, via podman/docker stats (MemUsage "used / limit").
proxy_rss_mb() {
  local raw used
  raw="$($RUNTIME stats --no-stream --format '{{.MemUsage}}' "$PROXY" 2>/dev/null)"
  used="${raw%%/*}"; used="$(echo "$used" | tr -d ' ')"
  awk -v v="$used" 'BEGIN{
    n=v; sub(/[A-Za-z]+$/,"",n); u=v; sub(/^[0-9.]+/,"",u); f=1;
    if (u=="B") f=1/1048576; else if (u ~ /^[kK]/) f=1/1024;
    else if (u ~ /^M/) f=1; else if (u ~ /^G/) f=1024; else if (u ~ /^T/) f=1048576;
    printf "%.1f", n*f;
  }'
}

# Pick wget/curl inside the driver container.
driver_tool() {
  if $RUNTIME exec "$DRIVER" sh -c 'command -v curl' >/dev/null 2>&1; then echo curl
  elif $RUNTIME exec "$DRIVER" sh -c 'command -v wget' >/dev/null 2>&1; then echo wget
  else echo none; fi
}

# Fire N requests through the proxy (uses the driver's configured HTTP(S)_PROXY).
drive_load() {
  local n="$1" tool
  [ "$LOAD" = off ] && return 0
  tool="$(driver_tool)"
  [ "$tool" = none ] && return 0
  if [ "$tool" = curl ]; then
    $RUNTIME exec "$DRIVER" sh -c \
      "i=0; while [ \$i -lt $n ]; do curl -s -o /dev/null --max-time 5 https://$LOAD_HOST/ 2>/dev/null; i=\$((i+1)); done" \
      >/dev/null 2>&1 || true
  else
    $RUNTIME exec "$DRIVER" sh -c \
      "i=0; while [ \$i -lt $n ]; do wget -T 5 -q -O /dev/null https://$LOAD_HOST/ 2>/dev/null; i=\$((i+1)); done" \
      >/dev/null 2>&1 || true
  fi
}

detect_runtime
if ! ctr_running "$PROXY"; then
  echo "CANNOT ASSESS — $PROXY not running. Bring the perimeter up first. Exit 2."
  exit 2
fi

per_tick_reqs=0
case "$LOAD" in
  off) per_tick_reqs=0 ;;
  light) per_tick_reqs=20 ;;
  heavy) per_tick_reqs=200 ;;
  *) echo "unknown --load: $LOAD (off|light|heavy)" >&2; exit 2 ;;
esac

ticks=$(( DURATION_MIN * 60 / INTERVAL_SEC ))
[ "$ticks" -lt 1 ] && ticks=1

date -u '+%Y-%m-%dT%H:%M:%SZ'
echo "── vault-proxy memory soak ($RUNTIME) ──────────────────────────"
echo "   duration=${DURATION_MIN}m interval=${INTERVAL_SEC}s load=${LOAD} ticks=${ticks} leak-threshold=${LEAK_MB_THRESHOLD}MB"
echo
printf "  %-22s %10s %12s\n" "TIMESTAMP" "RSS_MB" "Δ_BASELINE"

baseline=""; peak=0; last=0; t=0
while [ "$t" -lt "$ticks" ]; do
  [ "$per_tick_reqs" -gt 0 ] && drive_load "$per_tick_reqs"
  rss="$(proxy_rss_mb)"
  [ -z "$baseline" ] && baseline="$rss"
  delta="$(awk -v a="$rss" -v b="$baseline" 'BEGIN{printf "%+.1f", a-b}')"
  printf "  %-22s %10s %12s\n" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$rss" "$delta"
  awk -v a="$rss" -v p="$peak" 'BEGIN{exit !(a>p)}' && peak="$rss"
  last="$rss"
  t=$((t + 1))
  [ "$t" -lt "$ticks" ] && sleep "$INTERVAL_SEC"
done

net="$(awk -v a="$last" -v b="$baseline" 'BEGIN{printf "%.1f", a-b}')"
hours="$(awk -v d="$DURATION_MIN" 'BEGIN{printf "%.4f", d/60}')"
slope="$(awk -v n="$net" -v h="$hours" 'BEGIN{printf "%.2f", (h>0)?n/h:0}')"

echo
echo "── attribution ─────────────────────────────────────────────"
printf "  baseline=%s MB  peak=%s MB  final=%s MB  net=%s MB  slope=%s MB/h\n" \
  "$baseline" "$peak" "$last" "$net" "$slope"
[ "$JSON" = 1 ] && printf '{"baseline":%s,"peak":%s,"final":%s,"net":%s,"slope_mb_per_h":%s}\n' \
  "$baseline" "$peak" "$last" "$net" "$slope"

# Verdict: a leak is net growth past the threshold WITH a positive slope.
leak="$(awk -v n="$net" -v thr="$LEAK_MB_THRESHOLD" -v s="$slope" 'BEGIN{print (n>thr && s>0)?1:0}')"
if [ "$leak" = 1 ]; then
  echo "  LEAK SUSPECTED — net growth ${net}MB > ${LEAK_MB_THRESHOLD}MB with positive slope. See #42."
  exit 1
fi
echo "  Bounded over this run (net ${net}MB ≤ ${LEAK_MB_THRESHOLD}MB). Re-run longer to confirm steady state."
exit 0
