"""
Unit tests for vault-proxy's policy logic.

Pins the IP-literal denial defense at vault-proxy.py:_is_allowed() so that future
edits — including the planned server_connect hook in ADR-0009 Tier 2 — cannot
silently regress it. This is the floor for the DNS-rebinding hardening work
(plan: ~/.claude/plans/discuss-which-one-is-buzzing-crab.md, sequence step 1).

The addon imports mitmproxy at module load time. We stub those symbols before
import so the test runs hermetically without installing mitmproxy in CI.

Run from repo root:
    python3 -m unittest discover -s components/opencli-container/proxy -p 'test_*.py' -v
"""

import importlib.util
import os
import sys
import types
import unittest
from pathlib import Path


def _load_vault_proxy_module():
    """Load vault-proxy.py with mitmproxy stubbed so we avoid the runtime dep."""
    # Stub mitmproxy modules — only the names referenced at import time matter.
    mitm = types.ModuleType("mitmproxy")
    ctx = types.ModuleType("mitmproxy.ctx")
    http = types.ModuleType("mitmproxy.http")

    class _Log:
        @staticmethod
        def warn(*_a, **_k):
            pass

    ctx.log = _Log()
    ctx.options = types.SimpleNamespace(flow_detail=0)
    http.HTTPFlow = object  # only used as a type annotation
    http.Response = types.SimpleNamespace(make=lambda *_a, **_k: None)

    sys.modules["mitmproxy"] = mitm
    sys.modules["mitmproxy.ctx"] = ctx
    sys.modules["mitmproxy.http"] = http
    mitm.ctx = ctx
    mitm.http = http

    here = Path(__file__).parent
    src = here / "vault-proxy.py"

    # The addon writes a log file at /var/log/vault-proxy/requests.jsonl on init.
    # Redirect to a temp dir during import so the test doesn't need root.
    import tempfile

    tmp_log_dir = Path(tempfile.mkdtemp(prefix="vault-proxy-test-"))
    # The addon reads LOG_DIR as a module-level constant. Easiest path: monkey-patch
    # after spec_from_file_location but before exec_module — do it via env-driven
    # redirect by injecting a sentinel attribute on the loaded module.
    spec = importlib.util.spec_from_file_location("vault_proxy_under_test", src)
    module = importlib.util.module_from_spec(spec)
    # Override LOG_DIR before exec by patching the source's constant via a wrapper.
    # Simplest: set the constant on the module dict before exec_module — but exec_module
    # re-runs the file. Instead, point ALLOWLIST_PATH at a temp file and let the addon
    # create its log dir under our tmp tree by chdir'ing.
    original_cwd = os.getcwd()
    os.chdir(tmp_log_dir)
    try:
        # Patch module-level paths via a pre-exec hook.
        original_exec = spec.loader.exec_module

        def _patched_exec(m):
            original_exec(m)

        # Override LOG_DIR + ALLOWLIST_PATH by editing module attrs after import.
        # The __init__ runs in module body via `addons = [VaultProxy()]`, which
        # accesses LOG_DIR / ALLOWLIST_PATH already — so we must patch BEFORE exec.
        # Workaround: pre-create the expected dirs under tmp so the real paths work.
        (tmp_log_dir / "var" / "log" / "vault-proxy").mkdir(parents=True, exist_ok=True)
        (tmp_log_dir / "opt" / "vault").mkdir(parents=True, exist_ok=True)
        # Symlink the test allowlist into the expected absolute path requires root.
        # Instead: monkey-patch Path.exists for the allowlist path via a side channel.
        # Cleanest: edit the module's source by string-replacing the constants.
        src_text = src.read_text()
        patched = src_text.replace(
            'LOG_DIR = Path("/var/log/vault-proxy")',
            f'LOG_DIR = Path("{tmp_log_dir}/var/log/vault-proxy")',
        ).replace(
            'ALLOWLIST_PATH = Path("/opt/vault/allowlist.txt")',
            f'ALLOWLIST_PATH = Path("{tmp_log_dir}/opt/vault/allowlist.txt")',
        )
        # Write a tiny allowlist for the test environment.
        (tmp_log_dir / "opt" / "vault" / "allowlist.txt").write_text(
            "api.anthropic.com\napi.openai.com\napi.telegram.org\nraw.githubusercontent.com\n"
        )
        # Exec the patched source as the module body.
        exec(compile(patched, str(src), "exec"), module.__dict__)
    finally:
        os.chdir(original_cwd)

    return module


_VAULT_PROXY_MOD = _load_vault_proxy_module()
_VAULT_PROXY_INSTANCE = _VAULT_PROXY_MOD.VaultProxy()


class IPLiteralDenialTests(unittest.TestCase):
    """Pins vault-proxy.py:_is_allowed() rejection of IP literals.

    These are the destinations a DNS-rebinding attack would try to reach via
    an allowlisted domain whose authoritative server briefly resolves to
    private space. The addon-level defense catches IP-literal _hostnames_;
    the post-resolve defense (ADR-0009 Tier 2) catches the rebinding variant.
    This test guards the floor.
    """

    mod = _VAULT_PROXY_MOD
    proxy = _VAULT_PROXY_INSTANCE

    def test_loopback_ipv4_denied(self):
        self.assertFalse(self.proxy._is_allowed("127.0.0.1"))
        self.assertFalse(self.proxy._is_allowed("127.255.255.254"))

    def test_loopback_ipv6_denied(self):
        # mitmproxy returns IPv6 hosts in bracketed form.
        self.assertFalse(self.proxy._is_allowed("[::1]"))
        self.assertFalse(self.proxy._is_allowed("::1"))

    def test_docker_bridge_gateway_denied(self):
        # Default podman/docker bridge gateway — what an attacker would target
        # to reach the host from inside vault-agent.
        self.assertFalse(self.proxy._is_allowed("172.17.0.1"))

    def test_rfc1918_ranges_denied(self):
        for ip in (
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.1",
            "192.168.255.255",
        ):
            with self.subTest(ip=ip):
                self.assertFalse(self.proxy._is_allowed(ip))

    def test_link_local_denied(self):
        self.assertFalse(self.proxy._is_allowed("169.254.169.254"))  # AWS metadata
        self.assertFalse(self.proxy._is_allowed("[fe80::1]"))

    def test_public_ip_literals_also_denied(self):
        # The allowlist is domain-only; ANY IP literal must be rejected, not
        # just private ones. This is the existing pretty_host=IP-literal defense.
        self.assertFalse(self.proxy._is_allowed("1.1.1.1"))
        self.assertFalse(self.proxy._is_allowed("8.8.8.8"))

    def test_allowlisted_domain_still_allowed(self):
        # Sanity: regression test must not break the happy path.
        self.assertTrue(self.proxy._is_allowed("api.anthropic.com"))
        self.assertTrue(self.proxy._is_allowed("API.ANTHROPIC.COM"))  # case-insensitive
        self.assertTrue(self.proxy._is_allowed("foo.api.anthropic.com"))  # subdomain

    def test_non_allowlisted_domain_denied(self):
        self.assertFalse(self.proxy._is_allowed("evil.com"))
        self.assertFalse(self.proxy._is_allowed("anthropic.com.evil.com"))  # suffix-confusion


class DNSRebindingDefenseTests(unittest.TestCase):
    """Pins the post-resolve destination-IP check (ADR-0009 Tier 2).

    These tests use mocked socket.getaddrinfo so they run hermetically without
    DNS access. The real DNS path is exercised by the integration test in
    tests/test-network-isolation.sh against a live perimeter.
    """

    mod = _VAULT_PROXY_MOD
    proxy = _VAULT_PROXY_INSTANCE

    @staticmethod
    def _addrinfo(*ips):
        """Build a fake socket.getaddrinfo() return value for the given IPs."""
        import socket as _socket

        return [
            (_socket.AF_INET, _socket.SOCK_STREAM, 0, "", (ip, 0))
            if ":" not in ip
            else (_socket.AF_INET6, _socket.SOCK_STREAM, 0, "", (ip, 0, 0, 0))
            for ip in ips
        ]

    def _patch_resolver(self, ips=None, exc=None):
        """Context manager that monkey-patches socket.getaddrinfo on the loaded module."""
        from unittest.mock import patch

        target = "socket.getaddrinfo"
        # The loaded module imported socket at top level; patch through the module's namespace.
        if exc is not None:
            return patch.object(self.mod.socket, "getaddrinfo", side_effect=exc)
        return patch.object(self.mod.socket, "getaddrinfo", return_value=self._addrinfo(*(ips or [])))

    def test_rebind_to_loopback_blocked(self):
        with self._patch_resolver(ips=["127.0.0.1"]):
            self.assertTrue(self.proxy._resolves_to_private("evil.example.com"))

    def test_rebind_to_docker_bridge_blocked(self):
        with self._patch_resolver(ips=["172.17.0.1"]):
            self.assertTrue(self.proxy._resolves_to_private("evil.example.com"))

    def test_rebind_to_cloud_metadata_blocked(self):
        # The classic AWS/GCP metadata SSRF target.
        with self._patch_resolver(ips=["169.254.169.254"]):
            self.assertTrue(self.proxy._resolves_to_private("evil.example.com"))

    def test_rebind_to_rfc1918_blocked(self):
        for ip in ("10.0.0.1", "172.20.0.5", "192.168.1.1"):
            with self.subTest(ip=ip), self._patch_resolver(ips=[ip]):
                self.assertTrue(self.proxy._resolves_to_private("evil.example.com"))

    def test_rebind_to_ipv6_loopback_blocked(self):
        with self._patch_resolver(ips=["::1"]):
            self.assertTrue(self.proxy._resolves_to_private("evil.example.com"))

    def test_rebind_to_ipv6_ula_blocked(self):
        with self._patch_resolver(ips=["fd00::1"]):
            self.assertTrue(self.proxy._resolves_to_private("evil.example.com"))

    def test_mixed_results_with_any_private_blocked(self):
        # Defense-in-depth: if ANY A/AAAA returns private, reject the lookup.
        # An attacker who returns [public_ip, 127.0.0.1] cannot use the public
        # entry to slip through.
        with self._patch_resolver(ips=["8.8.8.8", "127.0.0.1"]):
            self.assertTrue(self.proxy._resolves_to_private("evil.example.com"))

    def test_public_only_results_allowed(self):
        with self._patch_resolver(ips=["8.8.8.8"]):
            self.assertFalse(self.proxy._resolves_to_private("dns.google"))
        with self._patch_resolver(ips=["1.1.1.1", "1.0.0.1"]):
            self.assertFalse(self.proxy._resolves_to_private("one.one.one.one"))

    def test_dns_error_fails_closed(self):
        import socket as _socket

        with self._patch_resolver(exc=_socket.gaierror("name resolution failed")):
            self.assertTrue(self.proxy._resolves_to_private("doesnotexist.invalid"))

    def test_empty_resolver_result_fails_closed(self):
        with self._patch_resolver(ips=[]):
            self.assertTrue(self.proxy._resolves_to_private("empty.example.com"))

    def test_ip_literal_passes_through_to_range_check(self):
        # _is_allowed already rejects raw-IP hostnames, so this method is
        # belt-and-suspenders. A private IP literal should still return True.
        self.assertTrue(self.proxy._resolves_to_private("127.0.0.1"))
        # A public IP literal should return False (no resolver call needed).
        self.assertFalse(self.proxy._resolves_to_private("8.8.8.8"))


class CredentialInjectionTests(unittest.TestCase):
    """Pins the data-driven credential-injection behavior (ADR-0001 + the agent-recipe).

    Behavior must match the pre-refactor hardcoded rules: api.anthropic.com gets
    x-api-key + anthropic-version; api.openai.com gets Authorization: Bearer; a
    matching host with no key set gets nothing (the request goes out
    unauthenticated — a missing key never leaks); a non-provider allowlisted host
    gets nothing. Also pins the optional mounted injection.json extension.
    """

    mod = _VAULT_PROXY_MOD
    proxy = _VAULT_PROXY_INSTANCE

    @staticmethod
    def _mock_flow(host):
        req = types.SimpleNamespace(
            pretty_host=host, method="POST", pretty_url=f"https://{host}/v1/x",
            content=b"{}", headers={},
        )
        return types.SimpleNamespace(request=req, response=None)

    def _inject(self, host, env):
        """Run request() against a public-resolving allowlisted host; return the flow."""
        from unittest.mock import patch
        flow = self._mock_flow(host)
        public = [(2, 1, 0, "", ("8.8.8.8", 0))]  # getaddrinfo → public => not rebinding
        with patch.dict(os.environ, env, clear=False), \
                patch.object(self.mod.socket, "getaddrinfo", return_value=public):
            self.proxy.request(flow)
        return flow

    def test_default_env_vars(self):
        self.assertEqual(self.proxy.injection_env_vars, ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"])

    def test_anthropic_injects_key_and_version(self):
        flow = self._inject("api.anthropic.com", {"ANTHROPIC_API_KEY": "sk-test-123"})
        self.assertEqual(flow.request.headers.get("x-api-key"), "sk-test-123")
        self.assertIn("anthropic-version", flow.request.headers)
        self.assertIsNone(flow.response)  # allowlisted + public => not blocked

    def test_openai_injects_bearer(self):
        flow = self._inject("api.openai.com", {"OPENAI_API_KEY": "sk-oai-9"})
        self.assertEqual(flow.request.headers.get("Authorization"), "Bearer sk-oai-9")

    def test_missing_key_injects_nothing(self):
        flow = self._inject("api.anthropic.com", {"ANTHROPIC_API_KEY": ""})
        self.assertNotIn("x-api-key", flow.request.headers)

    def test_non_provider_host_no_injection(self):
        flow = self._inject("raw.githubusercontent.com", {"ANTHROPIC_API_KEY": "sk-test-123"})
        self.assertNotIn("x-api-key", flow.request.headers)
        self.assertNotIn("Authorization", flow.request.headers)

    def test_host_matches(self):
        self.assertTrue(self.proxy._host_matches("api.anthropic.com", "api.anthropic.com"))
        self.assertTrue(self.proxy._host_matches("foo.api.anthropic.com", "api.anthropic.com"))
        self.assertFalse(self.proxy._host_matches("api.anthropic.com.evil.com", "api.anthropic.com"))

    def test_injection_config_extends_rules(self):
        import json as _json
        import tempfile
        from unittest.mock import patch
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            _json.dump([{
                "match": ["generativelanguage.googleapis.com"],
                "env_var": "GOOGLE_API_KEY",
                "headers": {"x-goog-api-key": "{key}"},
            }], f)
            cfg = f.name
        try:
            with patch.object(self.mod, "INJECTION_CONFIG_PATH", Path(cfg)):
                rules = self.proxy._load_injection_rules()
            self.assertEqual(len(rules), 3)  # 2 in-image defaults + 1 mounted
            self.assertIn("GOOGLE_API_KEY", {r["env_var"] for r in rules})
        finally:
            os.unlink(cfg)

    def test_injection_config_malformed_falls_back(self):
        import tempfile
        from unittest.mock import patch
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            f.write("not valid json {{{")
            cfg = f.name
        try:
            with patch.object(self.mod, "INJECTION_CONFIG_PATH", Path(cfg)):
                rules = self.proxy._load_injection_rules()
            self.assertEqual(len(rules), 2)  # fail-safe: in-image defaults stand
        finally:
            os.unlink(cfg)


if __name__ == "__main__":
    unittest.main(verbosity=2)
