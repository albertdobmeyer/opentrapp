#!/usr/bin/env bash
# Test: Network isolation — vault container can only reach proxy, not internet directly
#
# All HTTP requests use Node.js http module (wget/curl are not in the image).
# Proxy-routed requests are sent directly to vault-proxy:8080 in proxy-request
# format (path = full URL, Host header = target domain) so mitmproxy intercepts.
set -uo pipefail

RUNTIME="podman"
command -v podman &>/dev/null || RUNTIME="docker"
CONTAINER="opencli-container"
PASS=0
FAIL=0
INFO=0

check() {
    local desc="$1" cmd="$2" expect_fail="${3:-false}"

    printf "  %-55s " "$desc"

    output=$($RUNTIME exec "$CONTAINER" sh -c "$cmd" 2>&1) && exit_code=0 || exit_code=$?

    if [ "$expect_fail" = "true" ]; then
        if [ $exit_code -ne 0 ]; then
            echo "PASS (blocked as expected)"
            PASS=$((PASS + 1))
        else
            echo "FAIL (should have been blocked)"
            echo "       Output: $output"
            FAIL=$((FAIL + 1))
        fi
    else
        if [ $exit_code -eq 0 ]; then
            echo "PASS"
            PASS=$((PASS + 1))
        else
            echo "FAIL"
            echo "       Output: $output"
            FAIL=$((FAIL + 1))
        fi
    fi
}

echo ""
echo "=== Network Isolation Tests ==="
echo ""

# Check container is running
if ! $RUNTIME inspect "$CONTAINER" &>/dev/null; then
    echo "[!] Container '$CONTAINER' is not running."
    echo "    Start it first: $RUNTIME compose up -d"
    exit 1
fi

# Test 1: Container can reach proxy
check "Proxy reachable" \
    "node -e \"const h=require('http');const r=h.get({host:'vault-proxy',port:8080,timeout:5000},()=>{process.exit(0)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\""

# Test 2: Blocked domain returns 403 via proxy
check "evil.com blocked by proxy (403)" \
    "node -e \"const h=require('http');const r=h.get({host:'vault-proxy',port:8080,path:'http://evil.com/',headers:{Host:'evil.com'},timeout:5000},res=>{process.exit(res.statusCode===403?0:1)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\""

# Test 3: Direct internet access bypassing proxy should fail (no default gateway)
check "Direct internet blocked (no gateway)" \
    "node -e \"const h=require('http');const r=h.get({host:'1.1.1.1',port:80,timeout:5000},()=>{process.exit(0)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\"" true

# Test 4: IP-based request blocked via proxy (no domain allowlist bypass)
check "IP-based request blocked via proxy" \
    "node -e \"const h=require('http');const r=h.get({host:'vault-proxy',port:8080,path:'http://1.1.1.1/',headers:{Host:'1.1.1.1'},timeout:5000},res=>{process.exit(res.statusCode===403?0:1)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\""

# Test 4a: Loopback IP-literal blocked (DNS-rebinding floor — guards against
# an allowlisted domain being trivially swapped for a loopback request).
# Companion unit test: components/opencli-container/proxy/test_vault_proxy.py
check "Loopback IP literal blocked (127.0.0.1)" \
    "node -e \"const h=require('http');const r=h.get({host:'vault-proxy',port:8080,path:'http://127.0.0.1/',headers:{Host:'127.0.0.1'},timeout:5000},res=>{process.exit(res.statusCode===403?0:1)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\""

# Test 5: Case-insensitive blocking — EVIL.COM should also be blocked
check "Case insensitive blocking (EVIL.COM)" \
    "node -e \"const h=require('http');const r=h.get({host:'vault-proxy',port:8080,path:'http://EVIL.COM/',headers:{Host:'EVIL.COM'},timeout:5000},res=>{process.exit(res.statusCode===403?0:1)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\""

# Test 6: Long subdomain blocked (data exfiltration vector)
check "Long subdomain blocked (exfiltrated-data-payload.evil.com)" \
    "node -e \"const h=require('http');const r=h.get({host:'vault-proxy',port:8080,path:'http://exfiltrated-data-payload.evil.com/',headers:{Host:'exfiltrated-data-payload.evil.com'},timeout:5000},res=>{process.exit(res.statusCode===403?0:1)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\""

# Test 7: Subdomain edge case — evil.api.anthropic.com (informational only)
printf "  %-55s " "Subdomain edge case (evil.api.anthropic.com)"
output=$($RUNTIME exec "$CONTAINER" sh -c "node -e \"const h=require('http');const r=h.get({host:'vault-proxy',port:8080,path:'http://evil.api.anthropic.com/',headers:{Host:'evil.api.anthropic.com'},timeout:5000},res=>{process.exit(res.statusCode===403?1:0)});r.on('error',()=>process.exit(1));r.on('timeout',()=>{r.destroy();process.exit(1)})\"" 2>&1) && exit_code=0 || exit_code=$?
if [ $exit_code -ne 0 ]; then
    echo "INFO (blocked or unreachable — check subdomain matching policy)"
    INFO=$((INFO + 1))
else
    echo "INFO (reachable — subdomain of allowed domain)"
    INFO=$((INFO + 1))
fi

echo ""
echo "====================================="
echo "Results: $PASS passed, $FAIL failed, $INFO informational"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "[!] NETWORK ISOLATION TESTS FAILED"
    echo "    $FAIL test(s) did not pass. Review output above."
    exit 1
else
    echo "[+] All network isolation tests passed."
fi
