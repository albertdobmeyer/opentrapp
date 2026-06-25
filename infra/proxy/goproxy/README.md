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
- `policy/` — the **pure, network-free** L7 policy (a faithful port of `infra/proxy/vault-proxy.py`):
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

## NOT done yet — the perimeter switch (follow-up)
This builds + verifies the proxy in isolation. Switching the perimeter to it is the consumption-end
follow-up (each gated on the real boundary self-test, per CLAUDE.md §11):
1. Add the GHCR build for this image; flip `perimeter.yml` + `compose.yml` `vault-proxy` from the
   external mitmproxy image (`source: external`) to this **built** image, and simplify the entrypoint
   override (the chown is now in-image).
2. Re-validate the **seccomp** profile (`vault-proxy-seccomp.json`) against the Go runtime's syscalls.
3. Run the perimeter boundary self-test (`tests/boundary-selftest.sh`): **B2** (allowlist 403),
   **B3** (key not in agent), **B5** (CA fingerprint stable) must stay green, plus the #182 proxy pins.
4. Add a Go CI job (`go test ./infra/proxy/goproxy/...`). Then remove the Python `vault-proxy.py`.
