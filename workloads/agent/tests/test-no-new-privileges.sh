#!/usr/bin/env bash
# Test: no-new-privileges flag and setuid binary restrictions
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
CONTAINER="opencli-container"

echo "=== No New Privileges Tests ==="

# Test 1: NoNewPrivs is set in /proc/1/status
echo -n "  NoNewPrivs flag set: "
nnp=$($RUNTIME exec "$CONTAINER" sh -c 'grep NoNewPrivs /proc/1/status 2>/dev/null | awk "{print \$2}"' 2>&1) || true
if [ "$nnp" = "1" ]; then
    echo "PASS (NoNewPrivs = 1)"
else
    echo "FAIL — NoNewPrivs is '$nnp' (expected 1)"
    exit 1
fi

# Test 2: Check for setuid binaries (there should be none or they should be ineffective)
echo -n "  No effective setuid binaries: "
suid_bins=$($RUNTIME exec "$CONTAINER" sh -c 'find / -perm -4000 -type f 2>/dev/null || true' 2>&1) || true
if [ -z "$suid_bins" ]; then
    echo "PASS (no setuid binaries found)"
else
    # Even if suid binaries exist, no-new-privileges prevents escalation
    echo "WARN — setuid binaries found but NoNewPrivs blocks escalation:"
    echo "$suid_bins" | while read -r bin; do
        echo "       $bin"
    done
fi

# Test 3: Cannot change user ID even if setuid binary exists
echo -n "  Cannot escalate via su/newgrp: "
if $RUNTIME exec "$CONTAINER" sh -c 'su -c "id" root 2>&1' &>/dev/null; then
    echo "FAIL — su to root succeeded"
    exit 1
else
    echo "PASS (su blocked)"
fi

# Test 4: Verify current user is non-root
echo -n "  Running as non-root: "
uid=$($RUNTIME exec "$CONTAINER" sh -c 'id -u' 2>&1)
if [ "$uid" != "0" ]; then
    echo "PASS (uid=$uid)"
else
    echo "FAIL — running as root"
    exit 1
fi

echo "=== All no-new-privileges tests passed ==="
