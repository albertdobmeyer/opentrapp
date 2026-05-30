#!/usr/bin/env bash
# Test: Seccomp profile enforcement — blocked syscalls actually fail
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
CONTAINER="opencli-container"

echo "=== Seccomp Enforcement Tests ==="

# Test 1: Verify seccomp profile is loaded (mode 2 = filter active)
echo -n "  Seccomp profile loaded: "
seccomp_mode=$($RUNTIME exec "$CONTAINER" sh -c 'grep "^Seccomp:" /proc/1/status 2>/dev/null | head -1 | tr -s "[:space:]" " " | cut -d" " -f2' 2>&1) || true
if [ "$seccomp_mode" = "2" ]; then
    echo "PASS (mode 2 = filter active)"
else
    echo "FAIL — Seccomp mode is '$seccomp_mode' (expected 2)"
    exit 1
fi

# Test 2: mount syscall is blocked
echo -n "  mount() blocked: "
if $RUNTIME exec "$CONTAINER" sh -c 'mount -t tmpfs none /tmp 2>&1' &>/dev/null; then
    echo "FAIL — mount succeeded (should be blocked)"
    exit 1
else
    echo "PASS"
fi

# Test 3: ptrace syscall is blocked (attempt via Python if available, else via /proc)
echo -n "  ptrace() blocked: "
ptrace_result=$($RUNTIME exec "$CONTAINER" sh -c '
    if command -v python3 &>/dev/null; then
        python3 -c "import ctypes; libc=ctypes.CDLL(None); r=libc.ptrace(0,1,0,0); print(r)" 2>&1
    else
        # Fallback: try to read another process memory via /proc (requires ptrace)
        cat /proc/1/mem 2>&1
    fi
' 2>&1) && ptrace_exit=0 || ptrace_exit=$?
if [ $ptrace_exit -ne 0 ] || echo "$ptrace_result" | grep -qiE "denied|not permitted|operation not|error|-1"; then
    echo "PASS"
else
    echo "FAIL — ptrace may not be blocked: $ptrace_result"
    exit 1
fi

# Test 4: unshare syscall is blocked (namespace creation)
echo -n "  unshare() blocked: "
if $RUNTIME exec "$CONTAINER" sh -c 'unshare --mount /bin/true 2>&1' &>/dev/null; then
    echo "FAIL — unshare succeeded (should be blocked)"
    exit 1
else
    echo "PASS"
fi

# Test 5: reboot binary exists but is ineffective (no-new-privileges + non-root)
# We do NOT actually call reboot — it could kill the container.
# Instead we verify the binary can't escalate privileges.
echo -n "  reboot ineffective (non-root + NNP): "
uid=$($RUNTIME exec "$CONTAINER" sh -c 'id -u' 2>&1) || uid="unknown"
nnp=$($RUNTIME exec "$CONTAINER" sh -c 'grep "^NoNewPrivs:" /proc/1/status 2>/dev/null | head -1 | tr -s "[:space:]" " " | cut -d" " -f2' 2>&1) || nnp="unknown"
if [ "$uid" != "0" ] && [ "$nnp" = "1" ]; then
    echo "PASS (uid=$uid, NoNewPrivs=$nnp)"
else
    echo "FAIL — reboot may be effective (uid=$uid, NNP=$nnp)"
    exit 1
fi

echo "=== All seccomp enforcement tests passed ==="
