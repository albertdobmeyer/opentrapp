#!/bin/sh
# vault-proxy entrypoint. The mounted log + CA volumes are root-owned on first
# mount, so chown them to the runtime user before dropping privilege — otherwise
# the non-root proxy can't write requests.jsonl (the ZONE-3 bug that silently
# disabled host-side idle auto-pause). Then exec the proxy as the mitmproxy user.
set -e
if [ "$(id -u)" = "0" ]; then
    chown -R mitmproxy:mitmproxy /var/log/vault-proxy /home/mitmproxy/.mitmproxy 2>/dev/null || true
    exec su-exec mitmproxy /usr/local/bin/vault-proxy "$@"
fi
exec /usr/local/bin/vault-proxy "$@"
