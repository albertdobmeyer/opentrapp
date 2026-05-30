#!/bin/sh
# vault-egress entrypoint — ADR-0009 Tier 4 / Tier 5
#
# Boot order:
#   1. sysctl: enable IPv4 + IPv6 forwarding (needed for FORWARD chain to fire)
#   2. nftables: load the RFC1918 drop ruleset; FAIL-CLOSED if load fails
#   3. doh-resolver: start the pinned DNS resolver (Tier 5)
#   4. tinyproxy: start the L7 forwarder vault-proxy chains to
#
# If any step fails, the container exits non-zero. Compose restart-on-failure
# will keep retrying. The perimeter is fail-closed by design: no internet for
# the rest of the perimeter until vault-egress's policy is fully loaded.

set -eu

log() {
    printf '[vault-egress] %s\n' "$1" >&2
}

log "step 1/4: verifying IP forwarding (set declaratively via compose sysctls)"
# Note: /proc/sys is read-only when the container is read_only:true. The compose
# file's `sysctls:` block sets these at container creation; runtime `sysctl -w`
# would fail. We verify the declarative settings took effect instead.
v4_fwd=$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo "0")
v6_fwd=$(cat /proc/sys/net/ipv6/conf/all/forwarding 2>/dev/null || echo "0")
if [ "$v4_fwd" != "1" ] || [ "$v6_fwd" != "1" ]; then
    log "  FATAL: IP forwarding not enabled (v4=$v4_fwd v6=$v6_fwd)"
    log "  Check the 'sysctls:' block in compose.yml; standalone runs need --sysctl net.ipv4.ip_forward=1"
    exit 1
fi
log "  IPv4 + IPv6 forwarding active"

log "step 2/4: loading nftables ruleset"
if ! nft -f /opt/vault/nftables.conf; then
    log "  FATAL: nftables load failed — refusing to start (fail-closed)"
    exit 1
fi
# Verify the marker chain is actually present (defensive: nft -f can succeed
# silently on partial parse in some versions).
if ! nft list ruleset | grep -q 'vault_egress_drop_private'; then
    log "  FATAL: ruleset loaded but marker chain missing — fail-closed"
    exit 1
fi
log "  nftables: forward + output drops loaded, postrouting MASQUERADE active"

log "step 3/4: starting unbound pinned DoT resolver on 127.0.0.1:53"
# unbound provides: DoT to Quad9 + Cloudflare, cache-min-ttl=60 (rebinding
# defense), private-address rejection of upstream answers pointing at RFC1918.
# See unbound.conf for the full policy.
#
# `-d` keeps unbound in the foreground; we background it from the shell.
# Standard DNS port (53) is required so podman's compose `dns: [127.0.0.1]`
# directive routes container DNS through us — `/etc/resolv.conf` is read-only
# under `read_only: true`, so the dns: directive is the only path that works.
unbound -d -c /opt/vault/unbound.conf >/tmp/unbound.log 2>&1 &
UNBOUND_PID=$!

# Wait up to 5 seconds for unbound to actually bind port 53. Port-bound is the
# real readiness signal; `kill -0` returns false negatives during fork/exec.
unbound_ready=false
for _ in 1 2 3 4 5; do
    sleep 1
    if nc -z -w 1 127.0.0.1 53 2>/dev/null; then
        unbound_ready=true
        break
    fi
done

if [ "$unbound_ready" = "true" ]; then
    log "  unbound PID $UNBOUND_PID listening 127.0.0.1:53 (Quad9 primary, Cloudflare secondary)"
    # Verify it actually resolves by querying a known name. Failure here means
    # the upstream DoT path is broken (firewall, transient DNS hiccup, etc.);
    # we continue because Tier 4 (kernel filter) remains the load-bearing defense.
    if getent hosts dns.quad9.net >/dev/null 2>&1; then
        log "  test query through unbound succeeded (Tier 5 active)"
    else
        log "  WARN: unbound is up but test query failed — Tier 5 degraded"
    fi
else
    log "  WARN: unbound failed to bind 127.0.0.1:53 within 5 s"
    log "        Tier 5 degraded; Tier 4 (kernel filter) remains active"
    log "  unbound stderr tail:"
    tail -10 /tmp/unbound.log 2>/dev/null | sed 's/^/    /' || true
fi

log "step 4/4: starting tinyproxy on 0.0.0.0:8888"
# tinyproxy drops to nobody:nobody per its own config; we exec to inherit PID 1.
exec tinyproxy -d -c /opt/vault/tinyproxy.conf
