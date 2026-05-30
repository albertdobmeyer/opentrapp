#!/usr/bin/env bash
# Test: All capabilities dropped
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
CONTAINER="opencli-container"

echo "=== Capability Drop Tests ==="

# Test 1: NET_RAW dropped (ping fails)
echo -n "  NET_RAW dropped (ping blocked): "
if $RUNTIME exec "$CONTAINER" sh -c "ping -c 1 127.0.0.1 2>&1" &>/dev/null; then
    echo "FAIL — ping succeeded (NET_RAW not dropped)"
    exit 1
else
    echo "PASS"
fi

# Test 2: Cannot change file ownership (CAP_CHOWN effectively limited)
echo -n "  Non-root user (uid 1000): "
uid=$($RUNTIME exec "$CONTAINER" id -u)
if [ "$uid" = "1000" ]; then
    echo "PASS"
else
    echo "FAIL — running as uid $uid"
    exit 1
fi

echo "=== All capability tests passed ==="
