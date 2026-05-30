#!/usr/bin/env bash
# Test: Container escape vectors — verify common breakout paths are sealed
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
CONTAINER="opencli-container"

echo "=== Container Escape Vector Tests ==="

# Test 1: Docker socket not mounted
echo -n "  Docker socket not mounted: "
if $RUNTIME exec "$CONTAINER" sh -c 'ls /var/run/docker.sock 2>&1' &>/dev/null; then
    echo "FAIL — Docker socket is accessible"
    exit 1
else
    echo "PASS"
fi

# Test 2: /proc/sysrq-trigger not writable
echo -n "  /proc/sysrq-trigger not writable: "
if $RUNTIME exec "$CONTAINER" sh -c 'echo b > /proc/sysrq-trigger 2>&1' &>/dev/null; then
    echo "FAIL — sysrq-trigger is writable"
    exit 1
else
    echo "PASS"
fi

# Test 3: /proc/kcore not readable (real content)
# On Podman rootless, /proc/kcore may exist as a zero-length file.
# We check for actual readable content, not just file existence.
echo -n "  /proc/kcore not readable: "
kcore_size=$($RUNTIME exec "$CONTAINER" sh -c 'stat -c %s /proc/kcore 2>/dev/null || echo 0' 2>&1) || kcore_size="0"
kcore_read=$($RUNTIME exec "$CONTAINER" sh -c 'head -c 4 /proc/kcore 2>/dev/null | wc -c' 2>&1) || kcore_read="0"
if [ "$kcore_read" = "0" ] || [ "$kcore_size" = "0" ]; then
    echo "PASS (empty or unreadable)"
else
    echo "FAIL — /proc/kcore has readable content ($kcore_read bytes read)"
    exit 1
fi

# Test 4: No dangerous symlinks to host paths
echo -n "  No host path symlinks: "
host_links=$($RUNTIME exec "$CONTAINER" sh -c '
    for target in /mnt/c /mnt/d /host /rootfs /hostfs; do
        if [ -e "$target" ] || [ -L "$target" ]; then
            echo "$target"
        fi
    done
' 2>&1) || true
if [ -z "$host_links" ]; then
    echo "PASS"
else
    echo "FAIL — host paths accessible: $host_links"
    exit 1
fi

# Test 5: Cannot manipulate cgroups
echo -n "  Cgroup manipulation blocked: "
cgroup_result=$($RUNTIME exec "$CONTAINER" sh -c '
    # Try to write to cgroup controls
    echo 999999 > /sys/fs/cgroup/pids/pids.max 2>&1 || \
    echo 999999 > /sys/fs/cgroup/pids.max 2>&1 || \
    echo "blocked"
' 2>&1) || true
if echo "$cgroup_result" | grep -qiE "blocked|denied|read-only|permission|no such"; then
    echo "PASS"
else
    echo "FAIL — cgroup write may have succeeded: $cgroup_result"
    exit 1
fi

# Test 6: /proc/acpi not accessible (prevents host hardware interaction)
echo -n "  /proc/acpi not accessible: "
if $RUNTIME exec "$CONTAINER" sh -c 'ls /proc/acpi/ 2>&1' &>/dev/null; then
    echo "WARN — /proc/acpi exists (low risk but unnecessary)"
else
    echo "PASS"
fi

# Test 7: Cannot write to /proc/sys (kernel parameter modification)
echo -n "  /proc/sys not writable: "
if $RUNTIME exec "$CONTAINER" sh -c 'echo 1 > /proc/sys/kernel/panic 2>&1' &>/dev/null; then
    echo "FAIL — /proc/sys is writable"
    exit 1
else
    echo "PASS"
fi

# Test 8: No access to host PID namespace
echo -n "  Host PID namespace isolated: "
pid1_cmdline=$($RUNTIME exec "$CONTAINER" sh -c 'cat /proc/1/cmdline 2>/dev/null | tr "\0" " "' 2>&1) || true
if echo "$pid1_cmdline" | grep -q "tini\|openclaw\|node\|sh"; then
    echo "PASS (PID 1 is container init, not host systemd)"
else
    echo "WARN — PID 1 cmdline: $pid1_cmdline"
fi

echo "=== All container escape vector tests passed ==="
