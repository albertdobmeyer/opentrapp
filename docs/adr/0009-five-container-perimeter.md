# ADR-0009 — From four-container perimeter to five: separating L7 and L3 egress policy

**Status:** Accepted — implementation pending (Tier 2 landed 2026-05-18; Tier 4 deferred to a dedicated session)
**Decision date:** 2026-05-18
**Supersedes (partially):** [ADR-0006 — Four-container compose topology](0006-four-container-topology.md) — the count changes; the rationale for *why each boundary exists* is preserved.
**Companion ADR:** [ADR-0010 — Pinned-resolver DNS](0010-pinned-resolver-dns.md) (in flight; specifies the Tier 5 layer)
**Implemented by:**
- Tier 2 (L7 destination-IP check inside `vault-proxy.py`) — [`components/opencli-container/proxy/vault-proxy.py`](../../components/opencli-container/proxy/vault-proxy.py); regression-pinned by [`components/opencli-container/proxy/test_vault_proxy.py`](../../components/opencli-container/proxy/test_vault_proxy.py)
- Tier 4 (L3 kernel-level RFC1918 egress filter in `vault-egress` sidecar) — pending; will land as a new compose service + `egress-net` network + `components/opencli-container/egress/` directory
**Verified by:** Tier 2 — `python3 -m unittest discover -s components/opencli-container/proxy -p 'test_*.py'`; Tier 4 — `tests/orchestrator-check.sh` extension that asserts the five-container topology and `vault-proxy`'s lack of `external-net` attachment

---

## Context

The perimeter as shipped through v0.4.1 is described in [ADR-0006](0006-four-container-topology.md) as "four containers connected by per-service internal compose networks, with `vault-proxy` as the only bridge." That description is accurate at the network-topology level. It is **incomplete at the policy level.**

`vault-proxy` carries two structurally independent responsibilities:

1. **L7 (application-layer) policy.** Domain allowlisting, API-key injection, request logging, payload-size limits, response redaction. This is the role mitmproxy is designed for. It requires access to TLS interception, the request headers, and the live `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` credentials in process memory.
2. **L3 (network-layer) policy.** Ensuring outbound traffic from the perimeter cannot reach private/loopback destinations (RFC1918, 127/8, 169.254/16, IPv6 ULA, link-local, multicast). This is the responsibility that should structurally prevent DNS-rebinding attacks and SSRF against host/cloud-metadata services.

A 2026-05-17 dogfood review surfaced that the L3 responsibility is not currently enforced anywhere. The hostname-based allowlist matcher in `vault-proxy.py` rejects raw IP-literal hosts (regression-pinned by `test_vault_proxy.py`) but does *not* verify the IP the hostname resolves to. mitmproxy's `block_private` and `block_global` flags do not close the gap: they are source-IP filters (which clients may use the proxy), not destination-IP filters. `block_private=false` is set in `compose.yml:81` because the agent container's own source IP is private; setting it to `true` would block the agent from reaching the proxy. This semantic was previously documented in `components/opencli-container/docs/openclaw-internals.md:172` as "killed connections FROM private IPs (the vault container)" — accurate, but misread in the 2026-05-17 triage as "Telegram WebSocket compat" rather than as the structural source-side filter it actually is.

The gap is therefore real and load-bearing: an allowlisted domain whose authoritative DNS server briefly returns `127.0.0.1`, `172.17.0.1` (the default docker/podman bridge gateway), `169.254.169.254` (cloud metadata), or any RFC1918 range would pass the allowlist and be proxied to that destination. The threat model documents this as a **partially residual** risk under T3 (Network MITM) and T1 row 3 (fetch from attacker-controlled URL).

The architectural question is not "where do we add the check" but "where does the L3 responsibility belong." Two configurations are possible:

- **Co-located.** Keep `vault-proxy` as the single egress container and add kernel-level nftables rules inside it. This requires granting `vault-proxy` the `NET_ADMIN` capability and loosening its seccomp profile to permit netlink syscalls. The container that holds the live API credentials and intercepts all TLS traffic now also has elevated network privileges. This is a step *backward* on the principle that the smallest-blast-radius container should hold the most-privileged capabilities.
- **Separated.** Add a fifth container (`vault-egress`) that sits between `vault-proxy` and the public internet. `vault-egress` holds `NET_ADMIN` but holds *no secrets* and runs *no application code* — its entire attack surface is the netfilter ruleset and (with [ADR-0010](0010-pinned-resolver-dns.md)) a minimal DNS forwarder. `vault-proxy` keeps its existing hardened posture (all caps dropped, seccomp untouched) and loses its direct attachment to `external-net`.

The separated configuration is structurally the same decision that ADR-0006 made when it split the agent, the scanner, the proxy, and the feed analyser into four containers: a boundary exists when the responsibilities on either side of it have structurally different roles. L7 policy (application-aware, credential-bearing, TLS-terminating) and L3 policy (kernel-level, secret-free, ruleset-bearing) are structurally different. The four-container topology was a snapshot that conflated them; the five-container topology names them correctly.

## Decision

The runtime perimeter becomes **five containers**, with the responsibility split as follows:

| Container | Responsibility | Network connectivity | Capabilities |
|-----------|----------------|----------------------|--------------|
| `vault-agent` | agent runtime, Telegram gateway, loaded skills | `agent-net` (internal); only `vault-proxy` reachable | all dropped; seccomp deny-default |
| `vault-forge` | supply-chain scanner + CDR pipeline | `forge-net` (internal); only `vault-proxy` reachable | all dropped |
| `vault-pioneer` | social-content analyser (parked) | `pioneer-net` (internal); only `vault-proxy` reachable | all dropped |
| `vault-proxy` | **L7 policy** — domain allowlist, API-key injection, request logging, payload limits, response redaction | `agent-net` + `forge-net` + `pioneer-net` + `egress-net`; **no `external-net` attachment** | `NET_BIND_SERVICE`, `SETUID`, `SETGID`, `CHOWN`, `DAC_OVERRIDE` (unchanged); holds live API keys |
| `vault-egress` (new) | **L3 policy** — kernel-level RFC1918/loopback/link-local destination drop; pinned-resolver DNS forward (per ADR-0010); IP forwarding + masquerade for permitted traffic | `egress-net` + `external-net`; the *only* container with internet | `NET_ADMIN` (only this container); holds **no secrets**, runs **no application code** |

The flow becomes:

```
agent-net ─┐
forge-net ─┼─→ vault-proxy ──→ egress-net ──→ vault-egress ──→ external-net
pioneer-net┘    (L7 policy)                    (L3 policy)
                Tier 1: domain allowlist
                Tier 2: post-resolve IP check  Tier 3: kernel RFC1918 drop
                                               Tier 5: pinned resolver
```

The three layers are independent and each catches a distinct failure mode:

- **Tier 1 — domain allowlist.** Rejects hostnames not on the allowlist. Catches the common case.
- **Tier 2 — post-resolve IP check (in `vault-proxy.py`).** After the hostname passes the allowlist, `socket.getaddrinfo()` is called and the result is rejected if any A/AAAA falls in a private/loopback range. Catches casual DNS-rebinding. **Landed 2026-05-18.**
- **Tier 3 — kernel RFC1918 egress drop (in `vault-egress`).** nftables rules on `vault-egress`'s external interface drop any packet destined for RFC1918, 127/8, 169.254/16, IPv6 ULA/link-local, multicast, or reserved space. Catches the TOCTOU residue between Tier 2's resolver and mitmproxy's, and catches any future code path that bypasses Tier 2 entirely. **Pending Tier 4 implementation.**
- **Tier 5 — pinned resolver (in `vault-egress`).** See [ADR-0010](0010-pinned-resolver-dns.md). DoH to a trusted resolver with minimum-TTL cache; the only DNS path the perimeter can use. Defeats DNS poisoning at the resolver layer.

### Implementation order

1. **Tier 2** — landed 2026-05-18 inside the existing four-container model. Shipped as `_resolves_to_private()` in `vault-proxy.py` plus 11 unit tests in `test_vault_proxy.py`. Closes the gap at the application layer with no infrastructure changes.
2. **Threat-model edit (A2)** — landed 2026-05-18. T3 residual-risk block now documents the full picture (block_private semantics, the resolver path, the planned ADR-0009 / ADR-0010 mitigations).
3. **Tier 4 — `vault-egress` container.** Deferred to a dedicated session. New `components/opencli-container/egress/` directory containing `Containerfile` (Alpine + nftables), entrypoint that installs the drop rules and configures IP forwarding + masquerade, seccomp profile, `component.yml` manifest. Compose surgery to remove `vault-proxy` from `external-net` and add the new `egress-net`. Migration logic in `app/src-tauri/src/bootstrap/migrate_from_lobster_trapp.rs` and the kill paths in `components/opencli-container/scripts/kill.sh` + `app/src-tauri/src/commands/lifecycle.rs`. Orchestrator-check extension that asserts the five-container topology.
4. **Doc rewrite.** "Four-container perimeter" → "five-container perimeter" across README.md, docs/trifecta.md, docs/whitepaper.md, docs/diagrams.md, docs/threat-model.md, docs/index.html (landing-page hero), ADR-0001 / ADR-0006 / ADR-0007, tests/dogfood/CHECKLIST.md, and the v0.4.1 release notes (errata). ADR-0006's "Decision" table grows a row; its "Context" prose acquires a forward-reference to this ADR.
5. **ADR-0010 + Tier 5.** Pinned-resolver DNS implementation in `vault-egress`. Deferred to a second dedicated session.

## Consequences

### Positive

- **No single container holds both API credentials and elevated network capabilities.** The container that intercepts all TLS and holds the live `ANTHROPIC_API_KEY` (`vault-proxy`) loses its `external-net` attachment and gains no new capabilities. The container that gets `NET_ADMIN` (`vault-egress`) holds no secrets and runs no application code. This is principle-of-least-privilege at the per-container granularity.
- **Three independent defenses against DNS rebinding.** L7 allowlist (Tier 1) → L7 post-resolve IP check (Tier 2) → L3 kernel drop (Tier 3). A bug in any single layer does not produce the vulnerability. The kernel drop in particular survives addon bugs, mitmproxy bugs, and any future code path that bypasses the addon entirely.
- **The threat model becomes architecturally honest.** T3's DNS-rebinding residual risk goes from "partially residual" to "addressed by three independent layers, with a documented TOCTOU between layers 1+2 and layer 3 that is itself the reason layer 3 exists." OpenSSF / SignPath reviewers reading the threat-model and ADR slate get a clean separation-of-concerns story.
- **Future hardenings have a clear home.** Certificate pinning, egress rate-limiting, and per-destination QoS all belong in `vault-egress` rather than in the credential-bearing `vault-proxy`. The shape is now extensible.
- **The change names what was already true.** L7 and L3 policy were always structurally different; we'd been letting one container hold both responsibilities. Splitting them is the textbook architectural move.

### Negative

- **Operational surface grows.** Five containers instead of four. One more compose service, one more network (`egress-net`), one more `component.yml` manifest, one more kill-path entry, one more migration step for upgraders. The orchestrator-check suite grows by ~5 assertions.
- **Editorial cost is meaningful.** "Four-container perimeter" is load-bearing in the project's marketing copy (README hero, landing page hero, whitepaper §1, multiple ADRs). The doc rewrite is its own dedicated session.
- **One more failure mode at startup.** `vault-egress` must be running before `vault-proxy` can reach the internet; the compose `depends_on` graph deepens by one node. If `vault-egress`'s entrypoint fails to install the nftables ruleset, the perimeter must fail-closed (no internet) rather than fail-open. This is the right policy but it adds a class of "perimeter started but agent has no internet" failures the user-facing copy must handle.
- **Slight per-request latency increase.** Outbound packets now traverse one additional container hop. In practice this is ~hundreds of microseconds on a local network; negligible for an autonomous agent's typical request pattern but measurable on micro-benchmarks.
- **`NET_ADMIN` is granted, even if scoped.** Granting any capability to any container in the perimeter is a tradeoff against the "all caps dropped" baseline that the other four containers maintain. The mitigation is that `vault-egress` has no application code, no secrets, no writable filesystem outside the netfilter ruleset, and a minimal seccomp profile that allows only the syscalls needed for nftables, DNS forwarding, and IP forwarding. Its attack surface is the ruleset itself.

### Risks accepted

- **Tier 4 is deferred to a dedicated session.** Between Tier 2 (landed) and Tier 4 (pending), the residual risk is the TOCTOU between `_resolves_to_private()`'s `getaddrinfo()` and mitmproxy's at TCP-connect time. An attacker would need to control the authoritative DNS for an allowlisted domain (Anthropic, OpenAI, Telegram, GitHub raw) — a high bar — and time their rebinding to land between the two lookups. This is documented in T3's residual-risk block and is the gating reason the SignPath / OpenSSF resubmissions wait for Tier 4 rather than going out on Tier 2 alone.
- **The five-container framing is a public commitment.** Reverting to four containers (e.g. if Tier 4 reveals operational complications) would require another ADR and a doc rewrite. The decision is mergeable but not free to undo.

## Alternatives considered and rejected

- **Tier 0 — Document only.** Write the residual risk into the threat model; ship no code or compose changes. Rejected because the project's stated value commitment is "safety-first, safety-always" and the marketing pitch is "the best open-source security wrapper for autonomous CLI agents." A documented-but-unmitigated gap is not consistent with the public posture the project takes.
- **Tier 1 — Pre-resolve DNS check in `request` hook.** Resolve the host ourselves and reject if private, but call from a hook that fires before mitmproxy's resolver. Rejected in favour of Tier 2 (same hook, same TOCTOU, same defence) plus the recognition that the layer was always meant to be belt-and-suspenders with a kernel filter. Tier 2 alone shipping is fine; Tier 1 alone is weak.
- **Tier 3 — Kernel filter co-located in `vault-proxy`.** Add `NET_ADMIN` + loosen seccomp on the container that holds live API credentials. Rejected because it inverts the blast-radius principle: the container with the most-sensitive secrets becomes the container with the most-elevated capabilities. The separated configuration (Tier 4) costs one extra container and gives a meaningfully better security posture.
- **VM-level egress isolation.** Run the egress gateway in a per-session VM. Rejected for the same reason ADR-0006 rejected per-component VMs: operational and resource cost is disproportionate for the threat reduction, and the user-onboarding friction is unacceptable for a desktop application.

## Cross-references

- [ADR-0001 — Proxy-side API-key injection](0001-proxy-side-api-key-injection.md) — establishes why the credential-bearing container is a structural concern. ADR-0009 extends that reasoning to network capabilities.
- [ADR-0006 — Four-container topology](0006-four-container-topology.md) — partially superseded. The decision table grows from four rows to five; the rationale for *why each boundary exists* (structurally different responsibilities) is the same principle.
- [ADR-0010 — Pinned-resolver DNS](0010-pinned-resolver-dns.md) — companion ADR for the Tier 5 layer. Together ADR-0009 and ADR-0010 define the full five-container egress policy.
- [`docs/threat-model.md`](../threat-model.md) — T3 residual-risk block forward-references this ADR.
- [`docs/trifecta.md`](../trifecta.md) — architecture document; awaits the five-container rewrite in the dedicated session.
