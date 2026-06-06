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
# =============================================================================
set -euo pipefail

WATCH=0
case "${1:-}" in
  --watch) WATCH="${2:-5}" ;;
  --snapshot | "") WATCH=0 ;;
  -h | --help) sed -n '2,17p' "$0"; exit 0 ;;
  *) echo "unknown arg: $1 (use --snapshot or --watch N)" >&2; exit 2 ;;
esac

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

sys_total_mb() { awk '/^MemTotal:/ {printf "%.0f", $2/1024; exit}' /proc/meminfo; }

snapshot() {
  echo "── host memory ─────────────────────────────────────────────"
  free -h | awk 'NR==1 || /^Mem:|^Swap:/'
  echo
  echo "── perimeter containers (resident RSS) ─────────────────────"
  local stats
  stats="$(podman stats --no-stream --format '{{.Name}}|{{.MemUsage}}|{{.MemPerc}}' 2>/dev/null \
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
    pct="$(awk -v t="$total_mb" -v s="$sys" 'BEGIN{printf "%.1f", (s>0)?100*t/s:0}')"
    echo "  ──────────────────────────────────────────────────────"
    printf "  %-28s %7s MB %7s%%\n" "PERIMETER TOTAL" "$total_mb" "$pct"
    echo "  (of ${sys} MB system RAM)"
  fi
  echo
  echo "── on-disk image sizes (vault-*) ───────────────────────────"
  podman images --format '{{.Repository}}:{{.Tag}}  {{.Size}}' 2>/dev/null \
    | grep -iE 'vault-|opentrapp' | sort -u || echo "  (none)"
}

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
