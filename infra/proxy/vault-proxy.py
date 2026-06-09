"""
OpenCli-Container: mitmproxy Addon — API Key Injection + Domain Allowlist

This is the core security component. It runs as a mitmproxy addon in the
vault-proxy sidecar container, intercepting all outbound traffic from the
OpenClaw container.

Key behaviors:
  1. Block requests to domains not on the allowlist → 403
  2. Block raw IP addresses (allowlist is domain-only) → 403
  3. Block large outbound payloads (potential exfiltration) → 413
  4. Inject API keys into LLM provider requests (key never enters OpenClaw container)
  5. Redact API keys if reflected in responses
  6. Log all requests/responses as structured JSON for forensic review
  7. Block oversized responses (>10 MB)

Usage:
  mitmdump --listen-port 8080 --scripts vault-proxy.py
"""

import ipaddress
import json
import logging
import os
import re
import signal
import socket
import time
from pathlib import Path

from mitmproxy import ctx, http

LOG_DIR = Path("/var/log/vault-proxy")
ALLOWLIST_PATH = Path("/opt/vault/allowlist.txt")
EXFIL_THRESHOLD_BYTES = 1 * 1024 * 1024  # 1 MB — block large outbound payloads
EXFIL_RESPONSE_THRESHOLD_BYTES = 10 * 1024 * 1024  # 10 MB — block large responses
ANTHROPIC_API_VERSION = os.environ.get("ANTHROPIC_API_VERSION", "2023-06-01")

# Destination-IP filter for the DNS-rebinding defense (ADR-0009 Tier 2).
# An allowlisted hostname whose authoritative DNS server briefly returns one of
# these ranges would otherwise bypass the allowlist's hostname-only check and
# be proxied to a private/loopback destination. The kernel-level filter in
# ADR-0009 Tier 4 (vault-egress sidecar) closes the residual TOCTOU between
# our getaddrinfo() and mitmproxy's; this layer is the L7 defense.
_PRIVATE_DEST_NETWORKS = tuple(
    ipaddress.ip_network(n)
    for n in (
        "0.0.0.0/8",        # "This network"
        "10.0.0.0/8",       # RFC1918
        "100.64.0.0/10",    # Carrier-grade NAT (RFC6598)
        "127.0.0.0/8",      # IPv4 loopback
        "169.254.0.0/16",   # Link-local + AWS/GCP metadata (169.254.169.254)
        "172.16.0.0/12",    # RFC1918 — also catches default docker/podman bridge
        "192.0.0.0/24",     # IETF protocol assignments
        "192.168.0.0/16",   # RFC1918
        "198.18.0.0/15",    # Benchmark testing
        "224.0.0.0/4",      # IPv4 multicast
        "240.0.0.0/4",      # Reserved for future use
        "::1/128",          # IPv6 loopback
        "fc00::/7",         # IPv6 unique local addresses
        "fe80::/10",        # IPv6 link-local
        "ff00::/8",         # IPv6 multicast
    )
)

# Telegram Bot API embeds the token in the URL path: https://api.telegram.org/bot<id>:<hash>/<method>
# Redact before logging so tokens never hit stdout or the requests.jsonl file.
BOT_TOKEN_PATH_RE = re.compile(r"(/bot)\d+:[A-Za-z0-9_-]{20,}")


class VaultProxy:
    def __init__(self):
        self.allowlist: set[str] = set()
        self.logger = self._setup_logger()
        self._load_allowlist()
        self.logger.info("VaultProxy initialized with %d allowed domains", len(self.allowlist))
        signal.signal(signal.SIGHUP, lambda s, f: self._reload_allowlist())

    def _setup_logger(self) -> logging.Logger:
        logger = logging.getLogger("vault-proxy")
        logger.setLevel(logging.INFO)

        # Structured JSON log to /var/log/vault-proxy/requests.jsonl on the named
        # `vault-proxy-logs` volume. The container entrypoint chowns this dir to
        # the mitmproxy user before privilege-drop (the ZONE 3 fix), so the write
        # should always succeed. If it ever fails we fall back to ephemeral /tmp
        # to keep the perimeter functional — but we make that LOUD, because a
        # silent fallback here is exactly what hid ZONE 3 for weeks: the host-side
        # idle-auto-pause signal reads the *volume*, so a /tmp fallback silently
        # disables idle detection (read_egress_log_last_activity_ms → None).
        log_path = LOG_DIR / "requests.jsonl"
        try:
            LOG_DIR.mkdir(parents=True, exist_ok=True)
            handler = logging.FileHandler(log_path)
        except (PermissionError, OSError) as exc:
            fallback_path = Path("/tmp/vault-proxy-requests.jsonl")
            handler = logging.FileHandler(fallback_path)
            # Scream on BOTH stderr and stdout (podman logs) — this must never be
            # missed again. The logger we're configuring isn't wired up yet.
            import sys as _sys
            banner = (
                "\n"
                "============================================================\n"
                "  [vault-proxy] ZONE-3 ALERT: egress log is NOT persisting\n"
                f"  {log_path} not writable ({type(exc).__name__}: {exc}).\n"
                f"  Falling back to {fallback_path} (EPHEMERAL — lost on restart).\n"
                "  Host-side idle auto-pause is DISABLED while this persists\n"
                "  (read_egress_log sees no volume file → returns None).\n"
                "  The container entrypoint should have chowned the log dir to\n"
                "  the mitmproxy user — check the entrypoint shim / volume perms.\n"
                "============================================================\n"
            )
            print(banner, file=_sys.stderr, flush=True)
            print(banner, file=_sys.stdout, flush=True)
        handler.setFormatter(logging.Formatter("%(message)s"))
        logger.addHandler(handler)

        # Also log to stdout for `podman compose logs`
        stdout = logging.StreamHandler()
        stdout.setFormatter(logging.Formatter("[vault-proxy] %(message)s"))
        logger.addHandler(stdout)

        return logger

    def _load_allowlist(self):
        """Load allowed domains from allowlist.txt, one domain per line."""
        if not ALLOWLIST_PATH.exists():
            ctx.log.warn(f"Allowlist not found at {ALLOWLIST_PATH} — blocking ALL requests")
            return
        with open(ALLOWLIST_PATH) as f:
            for line in f:
                domain = line.strip()
                if domain and not domain.startswith("#"):
                    self.allowlist.add(domain.lower())

    def _reload_allowlist(self):
        """Reload allowlist from disk (triggered by SIGHUP). Atomic swap to avoid empty-set window."""
        old_count = len(self.allowlist)
        new_allowlist: set[str] = set()
        if ALLOWLIST_PATH.exists():
            with open(ALLOWLIST_PATH) as f:
                for line in f:
                    domain = line.strip()
                    if domain and not domain.startswith("#"):
                        new_allowlist.add(domain.lower())
        self.allowlist = new_allowlist
        self.logger.info("Allowlist reloaded: %d → %d domains", old_count, len(self.allowlist))

    def _is_allowed(self, host: str) -> bool:
        """Check if host matches any allowed domain (exact or subdomain)."""
        host = host.lower()
        # Reject raw IP addresses — allowlist is domain-only
        # Strip brackets for IPv6 (mitmproxy returns [::1] form)
        host_for_ip_check = host.strip("[]")
        try:
            ipaddress.ip_address(host_for_ip_check)
            return False
        except ValueError:
            pass
        for allowed in self.allowlist:
            if host == allowed or host.endswith("." + allowed):
                return True
        return False

    def _resolves_to_private(self, host: str) -> bool:
        """DNS-rebinding defense: True if `host` resolves to any private range.

        Called after the allowlist check passes. An allowlisted domain whose
        authoritative DNS briefly points to 127.0.0.1, 172.17.0.1 (default
        docker/podman bridge gateway), an RFC1918 range, or 169.254.169.254
        (cloud metadata) must not be proxied to that destination.

        Fail-closed: any DNS error (NXDOMAIN, timeout, parse failure) is
        treated as private. The legitimate allowlisted hosts always resolve
        to public IPs; a transient DNS failure means we cannot prove the
        destination is safe.

        Residual TOCTOU: mitmproxy will perform its own getaddrinfo() at
        connect time, and a TTL=0 attacker can return different answers
        across consecutive lookups. The kernel-level RFC1918 drop in the
        forthcoming vault-egress sidecar (ADR-0009 Tier 4) closes this gap.
        """
        host = host.strip("[]")
        # If the host is itself an IP literal, _is_allowed already rejected it.
        # This method is only reached after _is_allowed returned True, so a
        # literal IP shouldn't appear here — but stay defensive.
        try:
            ip = ipaddress.ip_address(host)
            return any(ip in net for net in _PRIVATE_DEST_NETWORKS)
        except ValueError:
            pass
        try:
            infos = socket.getaddrinfo(host, None)
        except (socket.gaierror, socket.herror, OSError):
            return True  # fail-closed
        if not infos:
            return True  # fail-closed
        for entry in infos:
            sockaddr = entry[4]
            addr = sockaddr[0]
            # Strip IPv6 zone identifier if present (e.g. "fe80::1%eth0")
            if "%" in addr:
                addr = addr.split("%", 1)[0]
            try:
                ip = ipaddress.ip_address(addr)
            except ValueError:
                return True  # fail-closed on unparseable address
            if any(ip in net for net in _PRIVATE_DEST_NETWORKS):
                return True
        return False

    def _log_event(self, event: dict):
        """Write structured JSON log entry."""
        event["timestamp"] = time.strftime("%Y-%m-%dT%H:%M:%S%z")
        self.logger.info(json.dumps(event, default=str))

    @staticmethod
    def _redact_url(url: str) -> str:
        # Bot-token-in-URL pattern is specific to Telegram; add others here if needed.
        return BOT_TOKEN_PATH_RE.sub(r"\1<REDACTED_BOT_TOKEN>", url)

    def running(self):
        # Silence mitmproxy's built-in per-flow summary prints, which also contain the
        # Telegram token in the URL and bypass our _log_event redaction path.
        ctx.options.flow_detail = 0

    def request(self, flow: http.HTTPFlow):
        """Intercept outbound requests: allowlist check, size check, API key injection."""
        host = flow.request.pretty_host
        method = flow.request.method
        url = flow.request.pretty_url

        # --- 1. Domain allowlist enforcement ---
        if not self._is_allowed(host):
            self._log_event({
                "action": "BLOCKED",
                "method": method,
                "url": self._redact_url(url),
                "host": host,
                "reason": "domain not in allowlist",
            })
            flow.response = http.Response.make(
                403,
                json.dumps({
                    "error": "blocked_by_vault",
                    "message": f"Domain '{host}' is not in the VAULT allowlist. "
                               f"Add it to proxy/allowlist.txt if this is intentional.",
                }).encode(),
                {"Content-Type": "application/json"},
            )
            return

        # --- 1b. DNS-rebinding defense (ADR-0009 Tier 2) ---
        # An allowlisted hostname that resolves to a private/loopback IP is
        # almost certainly a rebinding attempt or a misconfigured upstream.
        # The legitimate allowlisted endpoints (Anthropic, OpenAI, Telegram,
        # raw.githubusercontent.com) all resolve to public IPs.
        if self._resolves_to_private(host):
            self._log_event({
                "action": "BLOCKED",
                "method": method,
                "url": self._redact_url(url),
                "host": host,
                "reason": "host resolves to private/loopback range (DNS-rebinding defense)",
            })
            flow.response = http.Response.make(
                403,
                json.dumps({
                    "error": "blocked_by_vault",
                    "message": f"Host '{host}' resolves to a private or loopback address. "
                               f"This is blocked as a DNS-rebinding defense.",
                }).encode(),
                {"Content-Type": "application/json"},
            )
            return

        # --- 2. Block large outbound payloads (potential exfiltration) ---
        # MUST happen BEFORE API key injection so keys are never attached to blocked requests
        request_size = len(flow.request.content) if flow.request.content else 0
        if request_size > EXFIL_THRESHOLD_BYTES:
            self._log_event({
                "action": "EXFIL_BLOCKED",
                "method": method,
                "url": self._redact_url(url),
                "request_bytes": request_size,
                "reason": f"outbound payload exceeds {EXFIL_THRESHOLD_BYTES} bytes",
            })
            flow.response = http.Response.make(
                413,
                json.dumps({
                    "error": "exfiltration_blocked",
                    "message": f"Outbound payload ({request_size} bytes) exceeds "
                               f"exfiltration threshold ({EXFIL_THRESHOLD_BYTES} bytes).",
                }).encode(),
                {"Content-Type": "application/json"},
            )
            return

        # --- 3. API key injection (the headline feature) ---
        # Keys come from environment variables in the PROXY container only.
        # The OpenClaw container never sees these values.

        if host == "api.anthropic.com" or host.endswith(".api.anthropic.com"):
            api_key = os.environ.get("ANTHROPIC_API_KEY", "")
            if api_key:
                flow.request.headers["x-api-key"] = api_key
                flow.request.headers["anthropic-version"] = ANTHROPIC_API_VERSION
            else:
                ctx.log.warn("ANTHROPIC_API_KEY not set — request will fail auth")

        elif host == "api.openai.com" or host.endswith(".api.openai.com"):
            api_key = os.environ.get("OPENAI_API_KEY", "")
            if api_key:
                flow.request.headers["Authorization"] = f"Bearer {api_key}"
            else:
                ctx.log.warn("OPENAI_API_KEY not set — request will fail auth")

        self._log_event({
            "action": "ALLOWED",
            "method": method,
            "url": self._redact_url(url),
            "host": host,
            "request_bytes": request_size,
        })

    def response(self, flow: http.HTTPFlow):
        """Block oversized responses, redact reflected API keys, log metadata."""
        if flow.response:
            response_size = len(flow.response.content) if flow.response.content else 0

            # --- 1. Block oversized responses FIRST (before logging misleading 200) ---
            if response_size > EXFIL_RESPONSE_THRESHOLD_BYTES:
                self._log_event({
                    "action": "LARGE_RESPONSE_BLOCKED",
                    "url": self._redact_url(flow.request.pretty_url),
                    "response_bytes": response_size,
                    "reason": "response exceeds 10 MB threshold",
                })
                flow.response = http.Response.make(
                    413, b"Response too large", {"Content-Type": "text/plain"}
                )
                return

            # --- 2. Redact API keys if reflected in response (headers + body) ---
            for env_var in ("ANTHROPIC_API_KEY", "OPENAI_API_KEY"):
                key = os.environ.get(env_var, "")
                if not key:
                    continue
                key_bytes = key.encode()
                redacted = False
                # Scan ALL response headers (handles duplicate header names)
                # headers.fields is a tuple of (name, value) byte pairs
                new_fields = []
                for hname, hval in flow.response.headers.fields:
                    if key_bytes in hval:
                        hval = hval.replace(key_bytes, b"[REDACTED_BY_VAULT]")
                        redacted = True
                    new_fields.append((hname, hval))
                if redacted:
                    flow.response.headers.fields = tuple(new_fields)
                # Scan response body
                if flow.response.content and key_bytes in flow.response.content:
                    flow.response.content = flow.response.content.replace(
                        key_bytes, b"[REDACTED_BY_VAULT]"
                    )
                    redacted = True
                if redacted:
                    self._log_event({
                        "action": "KEY_REFLECTED",
                        "url": self._redact_url(flow.request.pretty_url),
                        "env_var": env_var,
                        "reason": "API key found in response — redacted",
                    })

            # --- 3. Log response metadata ---
            self._log_event({
                "action": "RESPONSE",
                "url": self._redact_url(flow.request.pretty_url),
                "status": flow.response.status_code,
                "response_bytes": response_size,
            })


addons = [VaultProxy()]
