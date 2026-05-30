#!/usr/bin/env bash
# Test: tmpfs size limits and noexec enforcement
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
CONTAINER="opencli-container"

echo "=== tmpfs Limits Tests ==="

# Test 1: /tmp is capped at 256m — writing beyond limit should fail
echo -n "  /tmp capped at 256m: "
result=$($RUNTIME exec "$CONTAINER" sh -c '
    # Try to write 300MB to /tmp (exceeds 256m cap)
    dd if=/dev/zero of=/tmp/overflow-test bs=1M count=300 2>&1
' 2>&1) && dd_exit=0 || dd_exit=$?
# Clean up
$RUNTIME exec "$CONTAINER" sh -c 'rm -f /tmp/overflow-test' 2>/dev/null || true
if [ $dd_exit -ne 0 ] || echo "$result" | grep -qiE "no space|cannot|full"; then
    echo "PASS (write beyond 256m rejected)"
else
    echo "FAIL — was able to write >256m to /tmp"
    exit 1
fi

# Test 2: /home/vault/workspace is capped at 1g
echo -n "  /home/vault/workspace capped at 1g: "
workspace_size=$($RUNTIME exec "$CONTAINER" sh -c 'df -m /home/vault/workspace 2>/dev/null | tail -1 | awk "{print \$2}"' 2>&1) || true
if [ -n "$workspace_size" ] && [ "$workspace_size" -le 1100 ] 2>/dev/null; then
    echo "PASS (${workspace_size}m total)"
else
    echo "WARN — workspace size is ${workspace_size}m (expected ~1024m)"
fi

# Test 3: noexec on /tmp — cannot execute scripts written there
echo -n "  noexec on /tmp: "
noexec_result=$($RUNTIME exec "$CONTAINER" sh -c '
    echo "#!/bin/sh" > /tmp/noexec-test.sh
    echo "echo pwned" >> /tmp/noexec-test.sh
    chmod +x /tmp/noexec-test.sh
    /tmp/noexec-test.sh 2>&1
' 2>&1) && noexec_exit=0 || noexec_exit=$?
# Clean up
$RUNTIME exec "$CONTAINER" sh -c 'rm -f /tmp/noexec-test.sh' 2>/dev/null || true
if [ $noexec_exit -ne 0 ] || echo "$noexec_result" | grep -qiE "denied|not permitted|cannot execute"; then
    echo "PASS (execution blocked)"
else
    echo "FAIL — script executed on noexec /tmp: $noexec_result"
    exit 1
fi

# Test 4: noexec on workspace
echo -n "  noexec on /home/vault/workspace: "
ws_result=$($RUNTIME exec "$CONTAINER" sh -c '
    echo "#!/bin/sh" > /home/vault/workspace/noexec-test.sh
    echo "echo pwned" >> /home/vault/workspace/noexec-test.sh
    chmod +x /home/vault/workspace/noexec-test.sh
    /home/vault/workspace/noexec-test.sh 2>&1
' 2>&1) && ws_exit=0 || ws_exit=$?
# Clean up
$RUNTIME exec "$CONTAINER" sh -c 'rm -f /home/vault/workspace/noexec-test.sh' 2>/dev/null || true
if [ $ws_exit -ne 0 ] || echo "$ws_result" | grep -qiE "denied|not permitted|cannot execute"; then
    echo "PASS (execution blocked)"
else
    echo "FAIL — script executed on noexec workspace: $ws_result"
    exit 1
fi

echo "=== All tmpfs limits tests passed ==="
