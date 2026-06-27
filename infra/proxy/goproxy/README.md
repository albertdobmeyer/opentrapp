# vault-proxy (goproxy) — the L7 egress chokepoint, in Go

The replacement for the leaky Python mitmproxy `vault-proxy`, decided + de-risked in
[ADR-0026](../../../docs/adr/0026-vault-proxy-replacement-evaluation.md). It does TLS
MITM to inject API keys into the contained agent's HTTPS (the credential never enters
the agent container — USP-1), enforces the allowlist + DNS-rebinding filter + size
caps + reflected-key redaction, logs every request as JSON, and chains **upstream** to
`vault-egress:8888` (no direct internet). Built on [`elazarl/goproxy`](https://github.com/elazarl/goproxy)
(vetted, ~10 MB, leak-free) after a PoC confirmed it supports MITM + injection + CONNECT
upstream-chaining — the requirement that ruled out the Rust path.

## Layout
- `policy/` — the **pure, network-free** L7 policy (originally ported from the now-removed `vault-proxy.py`; the goproxy is the sole L7 impl, ADR-0026):
  allowlist, DNS-rebinding ranges, injection rules, size thresholds, redaction, and
  `DecideRequest` (which pins the **size-block-before-injection** ordering). Unit-pinned in
  `policy/policy_test.go` against the same contract as the #182 Python pins.
- `main.go` — wires the policy into goproxy (handlers, upstream chaining, SIGHUP allowlist reload, JSON logging).
- `ca.go` — generate-or-load the CA, persisted in the `proxy-ca` volume as `mitmproxy-ca.pem`
  (cert+key) + `mitmproxy-ca-cert.pem` (the cert the agent trusts) so the CA fingerprint is
  **stable across restarts** (boundary self-test B5).
- `main_test.go` — handler + CA tests (off-allowlist 403, key injection, oversized 413 before
  injection, response-key redaction, oversized-response 413, CA fingerprint stability).
- `Containerfile` / `entrypoint.sh` — multi-stage static build on pinned alpine (**15.6 MB image**);
  the entrypoint chowns the mounted log + CA volumes then drops to the `mitmproxy` user (the ZONE-3 fix).

## Verified (this build, on the 7.2 GB box)
`go test ./...` green (13 tests, the ordering pin mutation-proven) · image builds (15.6 MB) · the
binary boots, listens on `:8080`, chains to `vault-egress:8888`, generates the CA at the
agent-trusted path, and fail-closes on a missing allowlist.

## Perimeter switch — status
1. ✅ **Switched in.** `compose.yml` (build:) + `perimeter.yml` (`source: built`) `vault-proxy` now
   build/run this image; the CI image-build job builds + signs it; the chown moved in-image.
2. ✅ **Seccomp re-validated.** The Go runtime boots *and* the request path run under the unchanged
   `vault-proxy-seccomp.json` (verified on the 7.2 GB box) — no profile change needed.
3. ✅ **Go CI job added** (`check-goproxy`: `go test` + `go vet`).
4. ✅ **Live boundary self-test GREEN.** `opentrapp-daemon vault up` → `vault verify` returns
   `pass=7 fail=0` cold **and** after pause→resume (B5 CA unchanged), via the product daemon on the
   7.2 GB box (2026-06-26; re-confirmed on the 1.25.11 rebuild 2026-06-27). Off-allowlist → 403,
   on-allowlist forwarded, credentials proxy-injected (none in `vault-agent`).
5. ✅ **Python `vault-proxy.py` removed** (source + test + the embedded copy; the goproxy compiles
   the matcher in, so only the allowlist data is provisioned). The live gate is green.
