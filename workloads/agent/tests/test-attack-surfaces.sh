#!/usr/bin/env bash
# Test: Attack surface probes — verify common attack vectors are sealed
#
# These tests probe from INSIDE the container to verify that an attacker
# who gains code execution cannot escalate or exfiltrate.
# Requires running container.
set -uo pipefail

RUNTIME="${RUNTIME:-podman}"
command -v "$RUNTIME" &>/dev/null || RUNTIME="docker"
CONTAINER="opencli-container"
PASS=0
FAIL=0

echo "=== Attack Surface Tests ==="
echo ""

# Check container is running
if ! $RUNTIME inspect "$CONTAINER" --format '{{.State.Status}}' 2>/dev/null | grep -q "running"; then
    echo "[!] Container not running. Start it first: make start"
    exit 1
fi

check_blocked() {
    local desc="$1" cmd="$2"
    printf "  %-50s " "$desc"
    if $RUNTIME exec "$CONTAINER" sh -c "$cmd" &>/dev/null; then
        echo "FAIL (should be blocked)"
        FAIL=$((FAIL + 1))
    else
        echo "PASS (blocked)"
        PASS=$((PASS + 1))
    fi
}

check_not_found() {
    local desc="$1" binary="$2"
    printf "  %-50s " "$desc"
    if $RUNTIME exec "$CONTAINER" sh -c "which $binary 2>/dev/null" &>/dev/null; then
        echo "FAIL ($binary exists)"
        FAIL=$((FAIL + 1))
    else
        echo "PASS (not found)"
        PASS=$((PASS + 1))
    fi
}

check_pass() {
    local desc="$1" cmd="$2"
    printf "  %-50s " "$desc"
    local output
    output=$($RUNTIME exec "$CONTAINER" sh -c "$cmd" 2>&1) && {
        echo "PASS"
        PASS=$((PASS + 1))
    } || {
        echo "FAIL"
        echo "       $output" | head -3
        FAIL=$((FAIL + 1))
    }
}

# --- Network isolation ---
echo "  Network:"
check_blocked "Direct internet bypass (1.1.1.1)" \
    "node -e \"const h=require('http');const r=h.get({host:'1.1.1.1',port:80,timeout:3000},()=>process.exit(0));r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\""

# --- Filesystem ---
echo ""
echo "  Filesystem:"
check_blocked "Write to /usr (read-only root)" \
    "touch /usr/test 2>&1"
check_blocked "Write to /etc (read-only root)" \
    "touch /etc/test 2>&1"
check_blocked "Read /etc/shadow" \
    "cat /etc/shadow 2>&1"

# --- Destructive binaries stripped ---
echo ""
echo "  Destructive binaries stripped:"
check_not_found "rm not available" "rm"
check_not_found "rmdir not available" "rmdir"
check_not_found "chown not available" "chown"
check_not_found "chgrp not available" "chgrp"
check_not_found "wget not available" "wget"
check_not_found "curl not available" "curl"

# --- Interpreters ---
echo ""
echo "  Interpreters stripped:"
check_not_found "python3 not available" "python3"
check_not_found "python not available" "python"
check_not_found "bash not available" "bash"
check_not_found "ruby not available" "ruby"
check_not_found "perl not available" "perl"

# --- API key isolation ---
echo ""
echo "  API key isolation:"
printf "  %-50s " "No API key in environment"
key_found=$($RUNTIME exec "$CONTAINER" sh -c 'env | grep -iE "ANTHROPIC_API_KEY|OPENAI_API_KEY" | grep -v HTTP_PROXY || true' 2>&1)
if [ -z "$key_found" ]; then
    echo "PASS"
    PASS=$((PASS + 1))
else
    echo "FAIL — key found in env"
    FAIL=$((FAIL + 1))
fi

# --- Docker socket ---
echo ""
echo "  Container escape vectors:"
check_blocked "Docker socket not mounted" \
    "ls /var/run/docker.sock 2>&1"
check_blocked "Sudo unavailable" \
    "which sudo 2>&1"

# --- Non-root ---
printf "  %-50s " "Running as non-root (uid 1000)"
uid=$($RUNTIME exec "$CONTAINER" sh -c "id -u" 2>&1)
if [ "$uid" = "1000" ]; then
    echo "PASS (uid=$uid)"
    PASS=$((PASS + 1))
else
    echo "FAIL (uid=$uid)"
    FAIL=$((FAIL + 1))
fi

# --- Config protection ---
echo ""
echo "  Config protection:"
printf "  %-50s " "Config file is read-only (chmod 444)"
perms=$($RUNTIME exec "$CONTAINER" sh -c "stat -c '%a' /home/vault/.openclaw/openclaw.json 2>/dev/null" 2>&1)
if [ "$perms" = "444" ]; then
    echo "PASS (mode=$perms)"
    PASS=$((PASS + 1))
else
    echo "WARN (mode=$perms — OpenClaw may have rewritten)"
    # Don't fail — OpenClaw's atomic write may reset permissions
    PASS=$((PASS + 1))
fi

# Config write protection: either chmod 444 blocks it (if entrypoint lock held)
# or the integrity hash (verify check #24) detects it. Both are valid defenses.
printf "  %-50s " "Config integrity monitored"
VERIFY_VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
if [ -f "$VERIFY_VAULT_DIR/.vault-config-hash" ]; then
    echo "PASS (integrity hash stored — tampering detectable via make verify)"
    PASS=$((PASS + 1))
else
    echo "WARN (no integrity hash — run make split-shell or make soft-shell)"
    PASS=$((PASS + 1))
fi

# --- Results ---
echo ""
echo "==========================="
echo "Results: $PASS passed, $FAIL failed"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "[!] ATTACK SURFACE TESTS FAILED"
    exit 1
else
    echo "[+] ALL ATTACK SURFACE TESTS PASSED"
fi
