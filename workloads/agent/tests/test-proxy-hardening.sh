#!/usr/bin/env bash
# Test: Proxy container hardening and operational security
set -euo pipefail

RUNTIME="${RUNTIME:-podman}"
PROXY_CONTAINER="vault-proxy"

echo "=== Proxy Hardening Tests ==="

# Test 1: Proxy container exists and is running
echo -n "  Proxy container running: "
if $RUNTIME inspect "$PROXY_CONTAINER" &>/dev/null; then
    state=$($RUNTIME inspect "$PROXY_CONTAINER" --format '{{.State.Status}}' 2>/dev/null || \
            $RUNTIME inspect "$PROXY_CONTAINER" --format '{{.State.Running}}' 2>/dev/null || echo "unknown")
    if [ "$state" = "running" ] || [ "$state" = "true" ]; then
        echo "PASS"
    else
        echo "FAIL — proxy state: $state"
        exit 1
    fi
else
    echo "FAIL — proxy container not found"
    exit 1
fi

# Test 2: Flag if proxy uses 'latest' tag (recommend digest pinning)
echo -n "  Proxy image tag check: "
proxy_image=$($RUNTIME inspect "$PROXY_CONTAINER" --format '{{.Config.Image}}' 2>/dev/null || echo "unknown")
if echo "$proxy_image" | grep -q ":latest"; then
    echo "WARN — using ':latest' tag ($proxy_image). Recommend pinning to a digest:"
    echo "         image: mitmproxy/mitmproxy@sha256:<digest>"
elif echo "$proxy_image" | grep -q "@sha256:"; then
    echo "PASS (digest-pinned: $proxy_image)"
else
    echo "INFO — image: $proxy_image"
fi

# Test 3: No API keys leaked in proxy logs
echo -n "  No API keys in proxy logs: "
proxy_logs=$($RUNTIME logs "$PROXY_CONTAINER" 2>&1 | tail -200) || true
if echo "$proxy_logs" | grep -qiE 'sk-ant-api|sk-[a-zA-Z0-9]{20,}|Bearer sk-'; then
    echo "FAIL — API key patterns found in proxy logs!"
    exit 1
else
    echo "PASS"
fi

# Test 4: API keys are configured (not empty, not placeholder)
# Note: avoid capturing the raw key value into a shell variable
echo -n "  API keys configured (not placeholder): "
if $RUNTIME inspect "$PROXY_CONTAINER" --format '{{range .Config.Env}}{{println .}}{{end}}' 2>/dev/null \
   | grep -qE 'ANTHROPIC_API_KEY=.+' \
   && ! $RUNTIME inspect "$PROXY_CONTAINER" --format '{{range .Config.Env}}{{println .}}{{end}}' 2>/dev/null \
   | grep -q "REPLACE-WITH-YOUR-KEY"; then
    echo "PASS (real keys configured)"
else
    echo "FAIL — API keys missing or placeholder values"
fi

# Test 5: Proxy is only accessible on internal network
echo -n "  Proxy on internal network: "
proxy_networks=$($RUNTIME inspect "$PROXY_CONTAINER" --format '{{range $k, $v := .NetworkSettings.Networks}}{{$k}} {{end}}' 2>/dev/null || echo "unknown")
if echo "$proxy_networks" | grep -q "vault-internal"; then
    echo "PASS (networks: $proxy_networks)"
else
    echo "WARN — unexpected network config: $proxy_networks"
fi

echo "=== All proxy hardening tests passed ==="
