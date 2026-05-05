# ADR-0006 — Four-container compose topology

**Status:** Accepted
**Decision date:** 2026-04-15 (architecture v2 redesign)
**Implemented by:** [`compose.yml`](../../compose.yml) (the four-service definition with per-service internal networks and the proxy bridge)
**Verified by:** [`tests/orchestrator-check.sh`](../../tests/orchestrator-check.sh) §1 and §6 (compose-file structure and service set); the network-isolation matrix in [`docs/trifecta.md`](../trifecta.md) §3 verified by inspection of `compose.yml`'s `networks:` block

---

## Context

The perimeter could in principle have been built at three different topology levels:

1. **Single container** — the OpenClaw runtime, the proxy, the scanner, and the social-content analyser all in one container, separated by user accounts and Linux namespaces.
2. **Four containers** — each major responsibility in its own container, connected by an internal compose network with a single bridge.
3. **VM-level isolation** — each major responsibility in its own virtual machine.

Each level has structural trade-offs. A single container is operationally simple and resource-light but couples failure domains and provides no cross-component network isolation. Four containers give per-component network isolation, capability scoping, and seccomp profiles but add operational surface (compose file, network names, volume mounts). A VM topology gives the strongest isolation but the largest operational and resource cost — and the most user-onboarding friction (a non-developer user cannot reasonably provision a per-session VM).

The *content* of the boundaries matters as much as their existence. The four-container layout exists because four boundaries had structurally different roles: the runtime (where the agent lives), the supply-chain pipeline (where skills are scanned and rebuilt), the network egress gateway (where credentials are held and outbound traffic is filtered), and the social-content analyser (where feed content was scanned before reaching the agent).

A configuration that lumps any two of these into one container loses one of these distinctions. Lumping the runtime with the proxy puts credentials in the same process the agent runs in. Lumping the runtime with the scanner gives the agent influence over its own skill scans. Lumping the proxy with the scanner removes the network-layer separation that makes the perimeter audit-clean.

## Decision

The runtime perimeter is composed of **four containers** connected by per-service internal compose networks, with `vault-proxy` as the only bridge:

| Container | Responsibility | Network connectivity |
|-----------|----------------|----------------------|
| `vault-agent` | OpenClaw runtime, Telegram gateway, loaded skills | `agent-net` (internal); only `vault-proxy` is reachable |
| `vault-forge` | Skill scanner + line classifier + CDR pipeline | `forge-net` (internal); only `vault-proxy` is reachable |
| `vault-pioneer` | Social-content scanner (parked — see [ADR-0004](0004-parking-moltbook-pioneer.md)) | `pioneer-net` (internal); only `vault-proxy` is reachable |
| `vault-proxy` | Egress gateway, credential holder, allowlist enforcer | All three internal networks plus the host's network — the only container with external connectivity |

Three structural properties this layout produces:

**(a) Per-component blast radius.** A compromise of `vault-agent` cannot reach `vault-forge` or `vault-pioneer` through any routed path; the only delivery channel from forge to agent is the write-only `forge-deliveries` shared volume.

**(b) Per-component capability profile.** Each container has its own seccomp profile, capability drops, and read-only-root configuration. The proxy needs slightly broader syscalls (TLS interception primitives) than the agent does; this widening is contained to one container rather than applying perimeter-wide.

**(c) Per-component lifecycle.** Containers can be started, stopped, restarted, or rebuilt independently. The lifecycle controls in [`app/src-tauri/src/lifecycle.rs`](../../app/src-tauri/src/lifecycle.rs) operate at the compose-service level; an operator can restart `vault-proxy` (e.g. after rotating the API credential in `.env`) without disturbing the agent's session state.

The layout is verified at every commit by [`tests/orchestrator-check.sh`](../../tests/orchestrator-check.sh) (which validates the compose-service set against the manifest contract) and at every container start by the 24-point hardening verification ([`components/openclaw-vault/scripts/verify.sh`](../../components/openclaw-vault/scripts/verify.sh)).

## Consequences

### Positive

- **The threat model in [`docs/threat-model.md`](../threat-model.md) maps cleanly onto the topology.** T1 (compromised agent) is contained to `vault-agent`; T2 (malicious skill) is contained to `vault-forge`'s pipeline; T3 (network MITM) is addressed at `vault-proxy`; T6 (side-channel) is addressed by the per-container log mounts. A single-container topology would not allow this clean mapping.
- **Each container can be hardened to its specific needs.** The agent has the narrowest seccomp profile because it runs untrusted-by-design code; the proxy has a wider seccomp profile because it needs TLS interception; the forge has a different profile again because it runs a parser. A single seccomp profile that covered all three responsibilities would necessarily widen to the union of needs.
- **The architecture is auditable from one file.** A reader inspecting `compose.yml` sees the topology, the networks, the volumes, the environment, the resource limits, and the dependency relationships. There is no second configuration source that could disagree with the compose file.
- **Cross-component invariants are checkable mechanically.** The orchestrator-check suite validates that each container's manifest declares only commands that exist in that container's image; that orchestrator workflows reference only declared component commands; that no container has a network entry it should not have. These checks are tractable because the topology is bounded.
- **Re-activation of pioneer is structurally cheap.** The `vault-pioneer` service is parked but defined; un-parking is a single profile change rather than an architecture change.

### Negative

- **Operational surface is non-trivial.** A user installing the application must have Podman or Docker; a user debugging the perimeter sees four containers with four sets of logs. The wizard hides this behind progress indicators, but the underlying complexity is real.
- **Resource footprint is higher than a single container.** Four containers carry four image layers, four PID namespaces, and four sets of compose metadata. On the maintainer's dev laptop (7.2 GB RAM), running the full perimeter consumes ~600 MB; a single-container alternative would be lighter. The trade-off is documented in the README's *Requirements* section.
- **Inter-container debugging is awkward.** A reader who wants to follow a request from the agent through the proxy to the upstream needs to correlate logs across three containers. The proxy's structured request log (host-readable) helps; correlation IDs across containers do not yet exist (queued as future work).
- **The shared-volume delivery channel between forge and agent is a non-trivial moving part.** The write-only `forge-deliveries` volume is verified by the agent's hash check on every load; a misconfiguration of the volume permissions would silently break the integrity check. The 24-point startup verification covers the main misconfiguration cases.

### Neutral

- The four-container choice is not a fundamental property of the architecture; it is a balance struck against single-container simplicity and VM-level isolation strength. A reader who needs stronger isolation can run this perimeter inside a disposable VM (per [`docs/why-not-x.md`](../why-not-x.md) §5); a reader who needs lower operational overhead can use OpenClaw's native `sandbox.mode` standalone (per [`docs/why-not-x.md`](../why-not-x.md) §1) and accept the residual risk.

## Alternatives considered

**(A) Single container.** All four responsibilities in one container, separated by user accounts and Linux namespaces. Rejected because it puts the credential in the same process the agent runs in (the issue ADR-0001 specifically addresses), and because a network-layer separation between the agent and the scanner is not realisable inside a single network namespace.

**(B) Two containers (agent + everything-else).** Combine forge, pioneer, and proxy into one shared "infrastructure" container behind the agent. Rejected because it removes the per-component capability scoping; the infrastructure container would need the union of forge's parser-needs and proxy's TLS-needs.

**(C) Three containers (agent + forge + proxy, no pioneer slot).** Drop pioneer entirely and ship a three-container topology. Rejected because it loses the architectural slot for hostile-network/social-feed content (T3 in the threat model); see [ADR-0004](0004-parking-moltbook-pioneer.md) for the parking-not-removing decision rationale.

**(D) Five or more containers.** Split the proxy further into "credential holder" + "allowlist enforcer" + "request logger". Rejected because the three responsibilities compose naturally inside mitmproxy's addon architecture; splitting them adds operational surface without adding clarity.

**(E) Per-skill containers.** Run each loaded skill in its own ephemeral container. Rejected because the skill catalogue is large (10+ skills installed in a typical Karen-like profile), the per-container startup cost is non-trivial, and the skill isolation already provided by the agent's container hardening + the CDR pipeline is sufficient for the threat model.

**(F) VM-level isolation.** Run each component in its own VM (or a single VM for the whole perimeter). Rejected for an end-user-installable product per the friction argument in [`docs/why-not-x.md`](../why-not-x.md) §5; the user who wants this is directed to run the four-container perimeter inside a disposable VM externally.

## References

- The compose definition itself: [`compose.yml`](../../compose.yml)
- Architecture document: [`docs/trifecta.md`](../trifecta.md) §3 (container topology, network-isolation matrix, ASCII tree, Mermaid drawing)
- Whitepaper: [`docs/whitepaper.md`](../whitepaper.md) §3.2 (system design)
- Threat model mapping: [`docs/threat-model.md`](../threat-model.md) — every attacker category maps to one or more containers
- Prior-art comparison: [`docs/why-not-x.md`](../why-not-x.md) (the differential against single-container, VM, and other alternatives)
- Companion ADRs: [ADR-0001](0001-proxy-side-api-key-injection.md) (proxy-side credentials are why proxy is its own container); [ADR-0003](0003-content-disarm-reconstruction.md) (forge's pipeline is why forge is its own container); [ADR-0004](0004-parking-moltbook-pioneer.md) (pioneer's parked status)
- Origin design spec: [`docs/archive/superpowers/2026-04-15-architecture-v2-perimeter-redesign.md`](../archive/superpowers/2026-04-15-architecture-v2-perimeter-redesign.md)
