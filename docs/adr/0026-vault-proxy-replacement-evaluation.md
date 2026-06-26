# ADR-0026 — vault-proxy replacement: evaluation & PoC-gated decision (WS-C)

**Status:** Accepted + BUILT + SWITCHED-IN (2026-06-25). The gating PoC passed; `goproxy` (elazarl) is
the chosen replacement. The proxy is built (`infra/proxy/goproxy`, merged #189) and the perimeter is
switched to it: `compose.yml` + `perimeter.yml` `vault-proxy` now build/run the goproxy image
(`source: built`), the CI image-build job builds + signs it, and a Go CI job gates its tests.
**Verified on the 7.2 GB box (proxy + config level):** boots + the request path run under the real
`vault-proxy-seccomp.json`; the tightened cap set (cap-drop ALL + SETUID/SETGID/CHOWN/DAC_OVERRIDE,
**no** NET_BIND_SERVICE) works (su-exec drops to the mitmproxy user); an off-allowlist host is blocked
**403 via a real MITM request** under seccomp; the CA is generated at the agent-trusted path; the
orchestration check + the #182 pins pass. **LIVE BOUNDARY GATE — GREEN (2026-06-26):** the full live
`boundary-selftest.sh` ran via the product daemon (`opentrapp-daemon vault up` → `vault verify`) against
the goproxy perimeter on the 7.2 GB box — **B1** isolation, **B2** allowlist (off-list `example.org`→403,
on-list `api.anthropic.com`→400 not-blocked), **B3** credential-separation (goproxy injects; no vendor key
in `vault-agent`), **B4** L3 egress, **B5** CA fingerprint **unchanged cold==resumed** (after `vault
pause`→`resume`), **B6** read-only delivery — `pass=7 fail=0` cold AND resumed. A **placeholder** key
suffices for the boundary (the checks test isolation/allowlist/credential-separation/CA, not key validity;
see task #96/P1-1), so no real credential was handled. **Still pending:** the production digest-pin
(BundleVerifier) path, which engages only with a signed release overlay — naturally exercised at the
post-release T0 once the cargo-dist release lane lands (ADR-0023). This ADR records the evaluation + PoC +
switch + the live-gate result so the team does not re-litigate it.

**Cross-references:** [ADR-0009](0009-five-container-perimeter.md) (the L7/L3 split this proxy lives in) ·
[ADR-0001](0001-proxy-side-api-key-injection.md) (proxy-side key injection — the chokepoint) ·
[CLAUDE.md §1 (USP-1 air-gap) / §11 (verify at the consumption end) / §12 (the bar)](../../CLAUDE.md) ·
`docs/footprint-and-device-usability.md` (the mitmproxy leak watch-item).

---

## Context

`vault-proxy` is the L7 egress chokepoint and the sharp end of **USP-1**: it is the *only* container
that holds the API keys, and it injects them into the contained agent's outbound HTTPS by **MITMing
TLS** (decrypt → inject `x-api-key` → re-encrypt) before chaining upstream to the L3 filter
(`vault-egress`). Today it is **mitmproxy** (Python). Two problems motivate replacing it (WS-C of the
lean-down campaign):

1. **Memory leak** — mitmproxy grows ~54 MB → ~550 MB over a heavy session (upstream issue #4456).
   *Already mitigated* by WS-A's `mem_limit: 1g` cap (bounds the blast radius), so this is a
   leanness/quality issue, **not urgent and not a correctness failure**.
2. **Weight + Python attack surface** — the image is ~250 MB; a lean static binary would be ~10-20 MB.

### The hard contract any replacement must satisfy (drop-in)

A full boundary inventory produced a 21-point checklist; the load-bearing, *disqualifying* ones are:

1. **TLS MITM with a persisted CA** — generate per-host leaf certs under a CA that is **written to the
   `proxy-ca` volume once and reused across restarts** (boundary self-test B5 pins the CA fingerprint
   as stable across restarts). The agent trusts `/opt/proxy-ca/mitmproxy-ca-cert.pem` via
   `CURL_CA_BUNDLE`/`REQUESTS_CA_BUNDLE`/`SSL_CERT_FILE`.
2. **Scriptable app-layer key injection** — decrypt and inject an env-var-sourced API key header on
   matching domains. (Rules out non-scriptable proxies — Envoy/HAProxy/nginx — and declarative-DSL
   ones.)
3. **Upstream-proxy chaining** — forward the re-encrypted traffic to `http://vault-egress:8888`, NOT
   direct to the internet (the proxy has no `external-net` route; ADR-0009). **This is the crux** —
   the requirement most candidates miss.
4. Allowlist (403), DNS-rebinding/private-IP filter (fail-closed), 1 MB request / 10 MB response caps
   (413), reflected-key redaction, and `requests.jsonl` JSON logging with `ts_ms` (read by
   idle-auto-pause). All of (4) is already pinned by the #182 proxy pins + `boundary-selftest.sh`.

### Benchmarks (priority order)

1. **Security + auditability** of the chokepoint (solo maintainer, public security tool) — a *vetted,
   maintained* codebase beats a hand-roll.
2. **Lean** — ~10-20 MB image, <50 MB RSS, leak-free.
3. **Solo-maintainability.**

## Evaluation (two research rounds, 2026-06-24)

### Rust ecosystem — does NOT yield a clean fit
- **`hudsucker`** (the one mature Rust MITM crate, actively maintained): **cannot chain upstream**
  (open issue #51, no maintainer commitment). Disqualified for our architecture unless we land a PR.
- **`http-mitm-proxy`**: upstream chaining undocumented; weaker maintenance signal.
- **`third-wheel`**: abandoned (2021). Reject.
- **Hand-rolled hyper + rustls + rcgen**: supports chaining natively, but a realistic TLS-MITM proxy
  is **1,500–2,500 LOC**, not the plan's "~500" — a large new audit surface on the credential
  chokepoint, solo-maintained. Cuts against benchmark #1.

### Vetted scriptable proxies — only mitmproxy is a confirmed full fit (and it's the one we're escaping)
| Proxy | CA persist | Key inject | **Upstream chain** | Image | Verdict |
|-------|:---------:|:----------:|:------------------:|:-----:|---------|
| **mitmproxy** (Py) | ✓ | ✓ | ✓ | ~250 MB | Meets all — **but leaks** |
| **goproxy** (Go, elazarl) | ✓ | ✓ | **? unconfirmed** | ~10 MB | **Best shot** — vetted (Stripe/Google/Grafana), leak-free; CONNECT-tunnel chaining not proven from docs |
| **gomitmproxy** (Go, AdGuard) | ✓ | ✓ | **? no chaining API** | ~30 MB | Red flag — would need custom transport wrapping |
| **Martian** (Go, Google) | ✓ | **✗ declarative DSL, no env-var substitution** | partial | ~15 MB | **Disqualified** on key injection |

### Key new fact
A **Go hand-roll is ~300–400 LOC** (Go's `net/http` + `crypto/tls` make MITM + CONNECT + injection
far terser than Rust's 2,000+). So the *fallback* is palatable on the auditability benchmark in a way
the Rust hand-roll never was. The tradeoff for any Go path: it introduces a **Go toolchain** to the
repo (currently Rust workloads + shell/nftables infra).

## Decision (gated)

**The crux uncertainty — does `goproxy` forward MITM'd HTTPS through an upstream proxy via CONNECT —
was resolved by a proof-of-concept, not a guess.**

**PoC RESULT (2026-06-24): PASS.** A hermetic 3-component loopback test
(`client → goproxy[MITM, injects header] → upstream-proxy[counts CONNECTs] → HTTPS origin`,
goproxy v1.8.4, Go 1.23) confirmed all three make-or-break requirements at once:
`upstream CONNECTs = 1` (chained the MITM'd HTTPS through the upstream proxy via CONNECT — the exact
thing that blocked `hudsucker`), the origin received the injected header (TLS MITM + scriptable
injection works), and the body round-tripped. Wiring: `proxy.OnRequest().HandleConnect(AlwaysMitm)` +
`proxy.Tr.Proxy = http.ProxyURL(upstream)`. **goproxy is confirmed viable.**

Decision tree (resolved):
1. ✅ **PoC passed → adopt `goproxy`** + ~300-500 LOC of policy glue (allowlist, injection, DNS-rebind,
   size caps, redaction, `requests.jsonl`). Vetted, ~10 MB, leak-free — the real WS-C win. The build
   must keep the #182 proxy pins + `boundary-selftest.sh` B2/B3/B5 green, verified at the consumption
   end (the real perimeter), per §11.
2. ~~PoC fails → Go hand-roll (~300-400 LOC) / hold on mitmproxy~~ — not needed; the PoC passed.
2. **WS-C is NOT urgent** (the leak is bounded). It must not be rushed on the chokepoint — doing it
   under pressure is exactly what the bar (§12) warns against. The higher-ROI, lower-risk lean-down
   moves (WS-B lean container bases; WS-E sharpen the USPs + the post-cutover footprint story) proceed
   in parallel and do not depend on WS-C.

## Consequences

- The whole-team record of *why* the obvious paths (a vetted Rust crate; a "~500 LOC" rewrite) do not
  work, so this is not re-researched.
- Whoever runs the PoC updates this ADR's Status to Accepted (with the chosen approach) or records the
  fallback taken. The replacement must keep the #182 proxy pins + `boundary-selftest.sh` B2/B3/B5
  green — verified at the consumption end (the real perimeter), per §11.
