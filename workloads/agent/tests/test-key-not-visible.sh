#!/usr/bin/env bash
# Test: API keys not visible inside the OpenClaw container (Path A only)
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
CONTAINER="opencli-container"

echo "=== API Key Isolation Tests ==="

# Test 1: No API key env vars
echo -n "  No API key in env: "
key_found=$($RUNTIME exec "$CONTAINER" sh -c 'env | grep -iE "api_key|api-key|apikey|bearer" | grep -v HTTP_PROXY || true')
if [ -z "$key_found" ]; then
    echo "PASS"
else
    echo "FAIL — found key in environment: $key_found"
    exit 1
fi

# Test 2: No API key in process list
echo -n "  No API key in /proc/*/environ: "
proc_key=$($RUNTIME exec "$CONTAINER" sh -c 'cat /proc/*/environ 2>/dev/null | tr "\0" "\n" | grep -iE "api_key|apikey" | grep -v HTTP_PROXY || true')
if [ -z "$proc_key" ]; then
    echo "PASS"
else
    echo "FAIL — found key in /proc"
    exit 1
fi

# Test 3: No API key in /proc/*/cmdline (command-line argument leak)
# Note: grep -v grep filters out the grep command's own args from /proc/self/cmdline
echo -n "  No API key in /proc/*/cmdline: "
cmdline_key=$($RUNTIME exec "$CONTAINER" sh -c 'cat /proc/[0-9]*/cmdline 2>/dev/null | tr "\0" "\n" | grep -iE "sk-ant-api03" | grep -v grep || true')
if [ -z "$cmdline_key" ]; then
    echo "PASS"
else
    echo "FAIL — found key pattern in cmdline args: $cmdline_key"
    exit 1
fi

# Test 4: No raw API keys in config files
echo -n "  No API key in config files: "
config_key=$($RUNTIME exec "$CONTAINER" sh -c '
    grep -rE "sk-ant-api|sk-[a-zA-Z0-9]{20,}" \
        /home/vault/.config/ \
        /etc/ \
        2>/dev/null || true
' 2>&1)
if [ -z "$config_key" ]; then
    echo "PASS"
else
    echo "FAIL — found key pattern in config files: $config_key"
    exit 1
fi

# Test 5: No API key in shell history
echo -n "  No API key in shell history: "
history_key=$($RUNTIME exec "$CONTAINER" sh -c '
    cat /home/vault/.ash_history /home/vault/.bash_history /home/vault/.sh_history 2>/dev/null | \
    grep -iE "sk-ant-api|sk-[a-zA-Z0-9]{20,}" || true
')
if [ -z "$history_key" ]; then
    echo "PASS"
else
    echo "FAIL — found key pattern in shell history"
    exit 1
fi

echo "=== All API key isolation tests passed ==="
