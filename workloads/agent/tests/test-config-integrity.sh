#!/usr/bin/env bash
# Test: Config integrity — verify security-critical settings in openclaw.json
#
# Checks the running OpenClaw config (JSON5 at ~/.openclaw/openclaw.json)
# for security-critical values. Uses node for JSON parsing (no python3
# in the container).
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
command -v "$RUNTIME" &>/dev/null || RUNTIME="docker"
CONTAINER="opencli-container"
CONFIG="/home/vault/.openclaw/openclaw.json"

echo "=== Config Integrity Tests ==="

# Test 1: Config file exists
echo -n "  Config file present: "
if $RUNTIME exec "$CONTAINER" sh -c "test -f $CONFIG" 2>/dev/null; then
    echo "PASS"
else
    echo "FAIL — config not found at $CONFIG"
    exit 1
fi

# Helper: check a config value using node JSON parsing.
# The config is plain JSON after OpenClaw processes it on startup.
# If the config is still JSON5 (pre-startup), these checks will fail —
# that's correct because we should only verify a running, initialized system.
check_config() {
    local desc="$1" js_expr="$2"
    printf "  %-45s " "$desc:"
    result=$($RUNTIME exec "$CONTAINER" sh -c "node -e \"const c=JSON.parse(require('fs').readFileSync('$CONFIG','utf8')); $js_expr\"" 2>&1)
    if [ $? -eq 0 ]; then
        echo "PASS"
    else
        echo "FAIL"
        echo "       $result" | head -3
        return 1
    fi
}

# Test 2: Elevated access disabled
check_config "Elevated access disabled" \
    "process.exit(c.tools.elevated.enabled===false?0:1)"

# Test 3: Sandbox mode is off (container IS the sandbox)
check_config "Sandbox mode = off" \
    "process.exit(c.agents.defaults.sandbox.mode==='off'?0:1)"

# Test 4: Gateway mode is local
check_config "Gateway mode = local" \
    "process.exit(c.gateway.mode==='local'?0:1)"

# Test 5: Telegram DM policy is pairing
check_config "Telegram DM policy = pairing" \
    "process.exit(c.channels.telegram.dmPolicy==='pairing'?0:1)"

# Test 6: Tool deny list is non-empty
check_config "Tool deny list present" \
    "process.exit(Array.isArray(c.tools.deny)&&c.tools.deny.length>0?0:1)"

# Test 7: Profile is known (minimal or coding, not full)
check_config "Profile is restricted (not full)" \
    "const p=c.tools.profile; process.exit(p==='minimal'||p==='coding'?0:1)"

# Test 8: WhatsApp disabled (prevents health-monitor spam)
check_config "WhatsApp channel disabled" \
    "const w=c.channels&&c.channels.whatsapp; process.exit(w&&w.enabled===false?0:1)"

echo "=== All config integrity tests passed ==="
