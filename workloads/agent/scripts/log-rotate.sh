#!/usr/bin/env bash
# OpenCli-Container: Log Rotation
#
# Rotates proxy logs and monitors session transcript size.
# Runs from the host. Safe to run while containers are running or stopped.
#
# Usage: bash scripts/log-rotate.sh
#
# Proxy log rotation:
#   - Rotates requests.jsonl when it exceeds 10MB
#   - Keeps 5 rotated copies (requests.jsonl.1 through requests.jsonl.5)
#   - Sends SIGHUP to vault-proxy after rotation (proxy reopens log file)
#
# Session transcript monitoring:
#   - Warns if total session transcripts exceed 100MB
#   - Does NOT delete transcripts (they're forensic evidence)
#
# Security notes:
#   - No data is deleted without user confirmation
#   - Proxy log rotation preserves old logs for forensic review
#   - SIGHUP is the standard signal for log rotation (proxy handles it)

set -uo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME="podman"
command -v podman &>/dev/null || RUNTIME="docker"

# Resolve a compose service name to the actual container name via the
# `com.docker.compose.service` label. Works regardless of project name or
# `container_name:` overrides — see docs/specs/2026-05-10-script-container-resolution.md
resolve_service_container() {
    local service container
    for service in "$@"; do
        container=$($RUNTIME ps -a \
            --filter "label=com.docker.compose.service=$service" \
            --format '{{.Names}}' 2>/dev/null | head -n 1)
        if [ -n "$container" ]; then
            echo "$container"
            return 0
        fi
    done
    return 1
}

PROXY_CONTAINER=$(resolve_service_container vault-proxy) || PROXY_CONTAINER=""
VAULT_CONTAINER=$(resolve_service_container vault) || VAULT_CONTAINER=""
PROXY_LOG_PATH="/var/log/vault-proxy/requests.jsonl"
MAX_LOG_BYTES=$((10 * 1024 * 1024))  # 10MB
MAX_ROTATIONS=5
SESSION_WARN_BYTES=$((100 * 1024 * 1024))  # 100MB

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

echo ""
echo -e "${BOLD}OpenCli-Container: Log Rotation${NC}"
echo "============================"
echo ""

# --- Proxy log rotation ---

echo -e "${BOLD}Proxy Logs${NC}"

proxy_running=false
if $RUNTIME inspect "$PROXY_CONTAINER" --format '{{.State.Status}}' 2>/dev/null | grep -q "running"; then
    proxy_running=true
fi

# Get the volume mount path for proxy logs
vol_path=""
if $proxy_running; then
    # Read log size from inside the running container
    log_size=$($RUNTIME exec "$PROXY_CONTAINER" sh -c "stat -c %s $PROXY_LOG_PATH 2>/dev/null" || echo "0")
    log_lines=$($RUNTIME exec "$PROXY_CONTAINER" sh -c "wc -l < $PROXY_LOG_PATH 2>/dev/null" || echo "0")
else
    # Try to find the volume on the host
    vol_path=$($RUNTIME volume inspect opencli-container_vault-proxy-logs --format '{{.Mountpoint}}' 2>/dev/null || echo "")
    if [ -n "$vol_path" ] && [ -f "$vol_path/requests.jsonl" ]; then
        log_size=$(stat -c %s "$vol_path/requests.jsonl" 2>/dev/null || echo "0")
        log_lines=$(wc -l < "$vol_path/requests.jsonl" 2>/dev/null || echo "0")
    else
        echo "  Proxy log not found (container stopped, volume not accessible)"
        log_size=0
        log_lines=0
    fi
fi

log_size_mb=$(echo "scale=1; $log_size / 1048576" | bc 2>/dev/null || echo "?")
echo "  Current size: ${log_size_mb}MB ($log_lines lines)"

if [ "$log_size" -gt "$MAX_LOG_BYTES" ] 2>/dev/null; then
    echo -e "  ${YELLOW}Log exceeds ${MAX_LOG_BYTES} bytes — rotating...${NC}"

    if $proxy_running; then
        # Rotate inside the container
        # Shift existing rotations: .5 → delete, .4 → .5, .3 → .4, .2 → .3, .1 → .2
        for i in $(seq $MAX_ROTATIONS -1 2); do
            prev=$((i - 1))
            $RUNTIME exec "$PROXY_CONTAINER" sh -c "mv -f ${PROXY_LOG_PATH}.$prev ${PROXY_LOG_PATH}.$i 2>/dev/null" || true
        done
        # Current → .1
        $RUNTIME exec "$PROXY_CONTAINER" sh -c "mv -f $PROXY_LOG_PATH ${PROXY_LOG_PATH}.1 2>/dev/null"
        # Create empty new log
        $RUNTIME exec "$PROXY_CONTAINER" sh -c "touch $PROXY_LOG_PATH"
        # Signal proxy to reopen log file
        $RUNTIME kill --signal SIGHUP "$PROXY_CONTAINER" 2>/dev/null
        echo -e "  ${GREEN}Rotated. SIGHUP sent to proxy.${NC}"

        # Show rotation status
        $RUNTIME exec "$PROXY_CONTAINER" sh -c "ls -lh ${PROXY_LOG_PATH}* 2>/dev/null" | while read -r line; do
            echo "    $line"
        done
    elif [ -n "$vol_path" ]; then
        # Rotate on the host volume
        for i in $(seq $MAX_ROTATIONS -1 2); do
            prev=$((i - 1))
            mv -f "$vol_path/requests.jsonl.$prev" "$vol_path/requests.jsonl.$i" 2>/dev/null || true
        done
        mv -f "$vol_path/requests.jsonl" "$vol_path/requests.jsonl.1" 2>/dev/null
        touch "$vol_path/requests.jsonl"
        echo -e "  ${GREEN}Rotated on volume. Restart proxy to pick up new file.${NC}"
    else
        echo -e "  ${RED}Cannot rotate — container stopped and volume not accessible.${NC}"
    fi
else
    echo -e "  ${GREEN}Within limits (threshold: 10MB). No rotation needed.${NC}"
fi

# --- Session transcript monitoring ---

echo ""
echo -e "${BOLD}Session Transcripts${NC}"

vault_running=false
if $RUNTIME inspect "$VAULT_CONTAINER" --format '{{.State.Status}}' 2>/dev/null | grep -q "running"; then
    vault_running=true
fi

if $vault_running; then
    session_size=$($RUNTIME exec "$VAULT_CONTAINER" sh -c "du -sb /home/vault/.openclaw/agents/main/sessions/ 2>/dev/null | cut -f1" || echo "0")
    session_count=$($RUNTIME exec "$VAULT_CONTAINER" sh -c "find /home/vault/.openclaw/agents/main/sessions/ -name '*.jsonl' -type f 2>/dev/null | wc -l" || echo "0")
else
    echo "  Container not running — checking volume..."
    vault_vol=$($RUNTIME volume inspect opencli-container_vault-data --format '{{.Mountpoint}}' 2>/dev/null || echo "")
    if [ -n "$vault_vol" ] && [ -d "$vault_vol/agents/main/sessions" ]; then
        session_size=$(du -sb "$vault_vol/agents/main/sessions/" 2>/dev/null | cut -f1 || echo "0")
        session_count=$(find "$vault_vol/agents/main/sessions/" -name '*.jsonl' -type f 2>/dev/null | wc -l || echo "0")
    else
        echo "  Session transcripts not accessible."
        session_size=0
        session_count=0
    fi
fi

session_size_mb=$(echo "scale=1; $session_size / 1048576" | bc 2>/dev/null || echo "?")
echo "  Total size: ${session_size_mb}MB across $session_count session files"

if [ "$session_size" -gt "$SESSION_WARN_BYTES" ] 2>/dev/null; then
    echo -e "  ${YELLOW}WARNING: Session transcripts exceed 100MB.${NC}"
    echo "  Consider archiving old sessions with: bash scripts/kill.sh --soft"
    echo "  Then review transcripts before deleting."
else
    echo -e "  ${GREEN}Within limits (threshold: 100MB).${NC}"
fi

echo ""
echo "============================"
echo ""
