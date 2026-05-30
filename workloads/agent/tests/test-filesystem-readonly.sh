#!/usr/bin/env bash
# Test: Filesystem is read-only except for designated tmpfs mounts
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
CONTAINER="opencli-container"

echo "=== Filesystem Read-Only Tests ==="

# Test 1: Root filesystem is read-only
echo -n "  Root read-only: "
if $RUNTIME exec "$CONTAINER" sh -c "touch /test-file 2>&1" &>/dev/null; then
    echo "FAIL — root is writable"
    exit 1
else
    echo "PASS"
fi

# Test 2: /tmp is writable (tmpfs)
# Note: rm is stripped from the image (constructive only), so we just test touch
echo -n "  /tmp writable (tmpfs): "
if $RUNTIME exec "$CONTAINER" sh -c "touch /tmp/test-readonly-check 2>&1" &>/dev/null; then
    echo "PASS"
else
    echo "FAIL — /tmp should be writable"
    exit 1
fi

# Test 3: /usr is read-only
echo -n "  /usr read-only: "
if $RUNTIME exec "$CONTAINER" sh -c "touch /usr/test-file 2>&1" &>/dev/null; then
    echo "FAIL — /usr is writable"
    exit 1
else
    echo "PASS"
fi

echo "=== All filesystem tests passed ==="
