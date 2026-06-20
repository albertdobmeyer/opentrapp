#!/usr/bin/env bash
# =============================================================================
# OpenTrApp Memory Profile
# =============================================================================
# Per-container resident memory (RSS) of the running perimeter, plus host
# RAM/swap and on-disk image sizes. This is the before/after gate for the
# memory-optimization work (idle auto-pause, on-demand shields, image slim).
#
# Usage:
#   bash tests/memory-profile.sh            # one snapshot (default)
#   bash tests/memory-profile.sh --watch 5  # re-sample every 5s until Ctrl-C
#
# Read-only: it starts and stops nothing. Bring the perimeter up first
# (`make perimeter-up`) to see live per-container numbers.
#
# Portability: runs on Linux and on Windows via WSL2. /proc/meminfo and free(1)
# are used when available; on platforms without them the host-RAM section is
# omitted gracefully. Container runtime is auto-detected (podman preferred).
# =============================================================================
set -euo pipefail

WATCH=0
case "${1:-}" in
  --watch) WATCH="${2:-5}" ;;
  --snapshot | "") WATCH=0 ;;
  -h | --help) sed -n '2,18p' "$0"; exit 0 ;;
  *) echo "unknown arg: $1 (use --snapshot or --watch N)" >&2; exit 2 ;;
esac

# ── runtime detection ─────────────────────────────────────────────────────────
RUNTIME="${OPENTRAPP_RUNTIME:-}"
detect_runtime() {
  if [ -n "$RUNTIME" ]; then return 0; fi
  if command -v podman >/dev/null 2>&1; then RUNTIME=podman
  elif command -v docker >/dev/null 2>&1; then RUNTIME=docker
  else echo "ERROR: neither podman nor docker found" >&2; exit 2; fi
}

# Convert a podman MemUsage token ("123.4MB", "1.2GiB", "512KiB") to integer MB.
to_mb() {
  awk -v v="$1" 'BEGIN{
    n=v; sub(/[A-Za-z]+$/,"",n);
    u=v; sub(/^[0-9.]+/,"",u);
    f=1;
    if (u=="B") f=1/1048576;
    else if (u ~ /^[kK]/) f=1/1024;
    else if (u ~ /^M/) f=1;
    else if (u ~ /^G/) f=1024;
    else if (u ~ /^T/) f=1048576;
    printf "%.0f", n*f;
  }'
}

# Returns total system RAM in MB, or "N/A" on platforms without /proc/meminfo.
sys_total_mb() {
  if [ -f /proc/meminfo ]; then
    awk '/^MemTotal:/ {printf "%.0f", $2/1024; exit}' /proc/meminfo
  else
    echo "N/A"
  fi
}

snapshot() {
  echo "── host memory ─────────────────────────────────────────────"
  if command -v free >/dev/null 2>&1; then
    free -h | awk 'NR==1 || /^Mem:|^Swap:/'
  else
    echo "  (free(1) not available on this platform — host RAM not shown)"
  fi
  echo
  echo "── perimeter containers (resident RSS) ─────────────────────"
  local stats
  stats="$($RUNTIME stats --no-stream --format '{{.Name}}|{{.MemUsage}}|{{.MemPerc}}' 2>/dev/null \
    | grep -i 'vault-' || true)"
  if [ -z "$stats" ]; then
    echo "  (no vault-* containers running — run 'make perimeter-up' first)"
  else
    printf "  %-28s %12s %8s\n" "CONTAINER" "RSS" "MEM%"
    local total_mb=0 name usage perc used mb
    while IFS='|' read -r name usage perc; do
      [ -n "$name" ] || continue
      used="$(echo "${usage%%/*}" | tr -d ' ')"
      printf "  %-28s %12s %8s\n" "$name" "$used" "$perc"
      mb="$(to_mb "$used")"
      total_mb=$((total_mb + ${mb:-0}))
    done <<< "$stats"
    local sys pct
    sys="$(sys_total_mb)"
    echo "  ──────────────────────────────────────────────────────"
    if [ "$sys" != "N/A" ]; then
      pct="$(awk -v t="$total_mb" -v s="$sys" 'BEGIN{printf "%.1f", (s>0)?100*t/s:0}')"
      printf "  %-28s %7s MB %7s%%\n" "PERIMETER TOTAL" "$total_mb" "$pct"
      echo "  (of ${sys} MB system RAM)"
    else
      printf "  %-28s %7s MB\n" "PERIMETER TOTAL" "$total_mb"
    fi
  fi
  echo
  echo "── on-disk image sizes (vault-*) ───────────────────────────"
  $RUNTIME images --format '{{.Repository}}:{{.Tag}}  {{.Size}}' 2>/dev/null \
    | grep -iE 'vault-|opentrapp' | sort -u || echo "  (none)"
}

detect_runtime
date -u '+%Y-%m-%dT%H:%M:%SZ'
if [ "${WATCH:-0}" -gt 0 ] 2>/dev/null; then
  while true; do
    snapshot
    echo
    sleep "$WATCH"
    date -u '+%Y-%m-%dT%H:%M:%SZ'
  done
else
  snapshot
fi
