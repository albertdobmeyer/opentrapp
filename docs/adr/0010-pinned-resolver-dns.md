# ADR-0010 — Pinned-resolver DNS as a perimeter primitive

**Status:** Accepted — implementation pending (configuration landed 2026-05-18 alongside ADR-0009; activation requires the dedicated session that builds and verifies the `vault-egress` container end-to-end)
**Decision date:** 2026-05-18
**Companion ADR:** [ADR-0009 — Five-container perimeter](0009-five-container-perimeter.md). ADR-0010 specifies the resolver layer that ADR-0009 calls "Tier 5".
**Implemented by:**
- [`components/opencli-container/egress/unbound.conf`](../../components/opencli-container/egress/unbound.conf) — the unbound configuration: DoT upstreams, `cache-min-ttl: 60`, `private-address` rejection
- [`components/opencli-container/egress/entrypoint.sh`](../../components/opencli-container/egress/entrypoint.sh) — starts unbound and points `/etc/resolv.conf` at 127.0.0.1:5353 (best-effort)
- [`compose.yml`](../../compose.yml) — `vault-egress` service definition; the only container with public-internet attachment, holding `NET_ADMIN` but no secrets
**Verified by:** `bash tests/orchestrator-check.sh` §10 (perimeter topology); manual `dig @127.0.0.1 -p 5353 …` smoke checks documented in [`docs/reproduce.md`](../reproduce.md)

---

## Context

The DNS-rebinding gap that motivated [ADR-0009](0009-five-container-perimeter.md) is closed at the **packet** level by the nftables RFC1918 drop on `vault-egress`'s external interface and at the **request** level by the post-resolve destination-IP check in `vault-proxy.py`. Both layers fire on the *outcome* of DNS resolution — they detect that a hostname has been pointed at a private address and refuse to deliver the packet.

The detection model breaks down structurally against three threats that operate at the resolver layer rather than the packet layer:

1. **TTL=0 rebinding loop.** An attacker who controls authoritative DNS for an allowlisted domain returns answer A on one query and answer B on the next, both with TTL=0. The kernel filter and the addon's `_resolves_to_private()` *do* drop any individual query that resolves to private space, but they can be retried indefinitely. If any single subsequent retry happens to fall in the gap (e.g. the connection is opened immediately after a successful resolve, before a re-check), the attack succeeds.
2. **Locally-poisoned resolver.** The host's `/etc/resolv.conf` typically points at a system resolver (router, ISP, DHCP-provided). On a hostile network — a compromised café Wi-Fi, a captive portal, a state-level adversary — the system resolver can return attacker-chosen answers for *any* domain. The L3/L7 filters defend against private destinations; they do not defend against a public-IP answer that nonetheless points at attacker infrastructure.
3. **Resolver-layer SSRF assistance.** A poisoned resolver can also return answers that pass the L3 filter (public IPs) but the IP belongs to attacker infrastructure that then redirects, mirrors, or proxies attacks against allowlisted upstreams. The L3 filter cannot see this; only a trusted resolver path can.

DNS poisoning and DNS rebinding are not adjacent threats — they are the same class of attack expressed at different layers. ADR-0009's L3 filter is a packet-level **detection**. ADR-0010 is a resolver-level **prevention**: control the resolver, you control what answers can ever reach the perimeter in the first place.

## Decision

`vault-egress` runs `unbound` as a forwarding stub resolver on `127.0.0.1:5353`. All DNS queries from any perimeter container resolve through this stub. Three properties make the stub a hardening primitive rather than a transparent forwarder:

1. **DNS-over-TLS to trusted upstreams.** Forward zone `.` is sent over TLS to Quad9 (primary: `9.9.9.9@853#dns.quad9.net`, `149.112.112.112@853#dns.quad9.net`) and Cloudflare (secondary: `1.1.1.1@853#cloudflare-dns.com`, `1.0.0.1@853#cloudflare-dns.com`). The TLS handshake validates the upstream's hostname against the system CA bundle. A local-network MITM or poisoned host resolver cannot intercept these queries.

   Quad9 is preferred over Cloudflare for the primary because Quad9 layers an industry malware-domain blocklist on top of standard resolution. A skill the agent might be tricked into fetching from a known-malicious domain fails at the resolver before reaching either L3 or L7 enforcement. Cloudflare is the failover for availability.

2. **Minimum TTL enforcement.** `cache-min-ttl: 60` overrides any upstream TTL below 60 seconds, pinning a resolved answer in cache for at least one minute. This is the structural defence against the TTL=0 rebinding loop: an attacker who returns a public IP on one query and `127.0.0.1` on the next cannot reach the second answer for 60 seconds — by which time the kernel filter or L7 check has fired on the first attempt and the connection is already closed.

   60 seconds is the standard "safe floor" for minimum TTL in security-focused resolvers. It balances rebinding defence against legitimate fast-failover use cases (which target ~30 s typical TTLs but tolerate up to 60 s for resolver behaviour). Operators who need different floors can edit `unbound.conf`; the cost surface is documented.

3. **Private-address rejection at the resolver.** `unbound`'s `private-address` directive instructs the resolver to discard upstream answers that point at RFC1918, loopback, link-local, multicast, or reserved space. This is defence-in-depth with the L3 nftables drop: if a malicious DoT upstream (or an unlikely TLS-cert-validated MITM) returns a private answer, it is filtered before any downstream container ever sees the address. The address ranges enumerated mirror the `_PRIVATE_DEST_NETWORKS` constant in `vault-proxy.py` and the nftables sets in `vault-egress/nftables.conf` — three independent enumerations of the same private-space concept, each in its own enforcement layer, by design.

The stub is published to the rest of the perimeter via `vault-egress`'s entrypoint, which rewrites `/etc/resolv.conf` to `nameserver 127.0.0.1` on best-effort (some compose runtimes mount the file read-only; the entrypoint logs and continues if so, and Tier 4's kernel filter remains active). The remaining four containers continue to inherit `/etc/resolv.conf` from their normal compose runtime — for them, the L7 + L3 layers are the resolver-independence guarantee.

A future iteration may require all perimeter containers to use `vault-egress`'s stub by setting their `dns:` blocks in `compose.yml`. The current design accepts the inconsistency because (a) only `vault-egress` itself resolves names on behalf of outbound traffic — the agent and forge resolve through `vault-egress` via the HTTP-CONNECT proxy path, which performs its own resolution after the L3 filter; (b) requiring a DNS dependency on `vault-egress` at compose startup increases the failure-coupling between containers in a way the four-container layout deliberately avoided.

## Consequences

### Positive

- **DNS rebinding is structurally defeated**, not just detected. The 60-second TTL floor closes the rebinding loop because no attacker DNS-rotation timing can outpace it.
- **The resolver path is trusted end-to-end.** Local-network MITM, captive portals, ISP-level interception, and DHCP-provided rogue resolvers can no longer alter DNS answers that reach the perimeter. The threat model's T3 (Network MITM) gains a structural mitigation it previously lacked.
- **Malware-domain blocking is free.** Quad9's threat-intel blocklist adds an industry-standard layer at no implementation cost to this project. Skills downloaded from known-malicious domains fail at the resolver before any of the perimeter's defences are exercised.
- **Three independent enumerations of "private space"** become deliberate defence-in-depth: the unbound `private-address` directive, the nftables ipv4/ipv6 sets, and the Python `_PRIVATE_DEST_NETWORKS` constant. A mistake in any one is caught by the others.
- **The perimeter's DNS posture is auditable in 30 seconds.** A reviewer reading `unbound.conf` sees the upstream pin, the minimum TTL, and the private-address rejection. The configuration is the audit.

### Negative

- **External dependency on Quad9 and Cloudflare.** The perimeter's DNS resolution is now structurally dependent on at least one of two well-known public resolvers. An outage at both simultaneously breaks resolution (the kernel filter is still active, but everything fails closed). The choice of two independent providers in different geographic regions reduces but does not eliminate this risk. Self-hosted recursive DNS (running `unbound` in recursive mode rather than forwarding) is documented as a future option for users with sovereignty requirements.
- **DoT handshake latency on first lookup.** TLS to the upstream resolver adds ~100–300 ms one-time setup per provider. After the connection is established, queries multiplex on it; subsequent lookups are normal-speed. The minimum-TTL cache amortizes this further.
- **`unbound` is an additional moving part.** The Containerfile adds the `unbound` package; the entrypoint adds a startup step that can fail. Failure modes are documented in `entrypoint.sh`: if `unbound` fails to start, the entrypoint logs a `WARN`, the system resolver is used as fallback, and Tier 5 silently degrades. Tier 4 (the kernel filter) remains the load-bearing defence. The perimeter does not fail-closed on `unbound` failure because the L3 layer is independently sufficient against the original threat (DNS rebinding); the L5 layer extends the defence to a wider class.
- **`cache-min-ttl: 60` can briefly mask legitimate DNS updates.** A legitimate operator who rotates an allowlisted domain to a new IP must wait up to 60 seconds for the perimeter to see the new answer. This is acceptable for the allowlisted upstream set (Anthropic, OpenAI, Telegram, GitHub raw) — none of which performs fast-failover at sub-60-second granularity.

### Risks accepted

- **Compromise of a DoT upstream propagates to the perimeter.** If an attacker compromises Quad9's resolver, signs a malicious answer with a valid TLS certificate, and the answer points at a public IP that itself attacks an allowlisted upstream, the perimeter has no further defence at the resolver layer. The L7 destination-IP check still rejects private answers; the L7 allowlist still rejects unknown hosts; the L3 filter still drops private packets. But a public-IP-attack against a legitimate upstream is in scope of T3 (Network MITM) generally and is documented there.
- **Resolver enumeration is observable.** Quad9 and Cloudflare can see the perimeter's DNS query stream. This reveals which allowlisted upstreams the agent is talking to and when, but no query content or response handling. This is the same disclosure the agent's existing TLS-to-upstream traffic already makes via SNI; the resolver does not add new disclosure.
- **A user behind a corporate firewall that blocks DoT (port 853 outbound)** cannot use Quad9 or Cloudflare's encrypted resolvers. The perimeter currently has no plain-DNS fallback by design — falling back to plain DNS would reopen the local-MITM vector this ADR exists to close. Such users must either configure their firewall to allow port 853 to the documented IPs, or accept that the perimeter cannot run on that network. This is consistent with the "operates only on internet-connected networks" requirement.

## Alternatives considered and rejected

- **Run `unbound` in recursive mode (no upstream forwarder).** Eliminates the dependency on Quad9/Cloudflare but introduces full recursive resolution from the perimeter, including UDP port 53 traffic to the public DNS infrastructure. Rejected for v1 because (a) the additional attack surface (port 53 UDP from the perimeter to arbitrary authoritative nameservers) is larger than the dependency on two well-known DoT resolvers; (b) recursive resolution is observable at the network layer in a way forwarding-over-TLS is not; (c) the implementation complexity of trust anchors, root hints, and DNSSEC validation is meaningful. Documented as a future option.
- **DoH instead of DoT.** DNS-over-HTTPS would multiplex over the same TCP/443 path the rest of the perimeter's traffic uses, reducing network-layer fingerprintability. Rejected because DoH adds a full HTTPS stack and library dependency (`unbound` does not speak DoH natively in the version packaged by Alpine); DoT achieves the same trust property with a smaller dependency footprint. Cloudflared (a Cloudflare-provided DoH proxy) was considered but adds another supply-chain element (a Go binary downloaded outside the alpine repository) that the current design avoids.
- **Trust the host's resolver.** Rejected on the grounds enumerated in the Context section. The host's resolver is not a trust boundary the perimeter can depend on.
- **No resolver layer at all (rely on ADR-0009 Tier 2 + Tier 4 only).** Documented as the "Tier 4 only" landing state — what ships if ADR-0010 is delayed. Tier 4 alone closes the original DNS-rebinding gap but does not close the wider DNS-poisoning attack surface. Tier 5 is the structural extension.

## Cross-references

- [ADR-0009 — Five-container perimeter](0009-five-container-perimeter.md) — establishes the `vault-egress` container that hosts the resolver. ADR-0010 specifies the resolver's behaviour; ADR-0009 specifies the container that hosts it.
- [`components/opencli-container/egress/unbound.conf`](../../components/opencli-container/egress/unbound.conf) — the live configuration; the source of truth for what the resolver actually does.
- [`docs/threat-model.md`](../threat-model.md) — T3 (Network MITM) residual-risk block forward-references this ADR; future-work table cites it.
- [`docs/whitepaper.md`](../whitepaper.md) §3 — public narrative for the resolver layer.
