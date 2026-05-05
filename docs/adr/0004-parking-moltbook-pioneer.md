# ADR-0004 — Parking moltbook-pioneer

**Status:** Accepted
**Decision date:** 2026-05-03
**Implemented by:** [`compose.yml`](../../compose.yml) (vault-pioneer service retained but with `profiles: [parked]`); [`components/moltbook-pioneer/README.md`](../../components/moltbook-pioneer/README.md) (parked notice); [`docs/trifecta.md`](../trifecta.md) §4.3
**Verified by:** Surface-level smoke check — pioneer no longer reachable from the GUI's add-tool flow; `tests/orchestrator-check.sh` continues to validate the pioneer manifest against the schema even though the service is parked

---

## Context

`moltbook-pioneer` was designed to scan posts on the Moltbook agent social network for prompt-injection patterns before that content reached `vault-agent`. The container is still defined in `compose.yml`. The 25-pattern catalogue is still in the submodule. The integration with `vault-proxy` for filtered fetches works.

Two external events changed the calculus:

- **2026-03-10** — Meta acquired Moltbook for an undisclosed sum. Moltbook's public roadmap was withdrawn; the founding team was reassigned.
- **2026-04-05 onwards** — the Moltbook public API has been intermittent. Several documented endpoints either return 404, return inconsistent results, or have been silently rate-limited. The official advisory channel has not communicated whether this is transitional, deliberate, or a precursor to API closure.

By the time the polish phase concluded (2026-05-02), there was no integrator-facing signal that the API would stabilise. Continuing to test, document, and surface a feature that depends on the unreliable API was producing both maintenance load (broken integration tests, noisy health probes) and user-facing confusion (a "Pioneer" tile that sometimes worked).

A pure-removal decision would have lost the architectural slot — pioneer's role in the perimeter (third-tier defense for hostile network/social content; T3 in [`docs/threat-model.md`](../threat-model.md)) is real and would still be wanted if the agent-social-network category re-emerges under Meta's continued Moltbook operation, under a successor platform, or under an entirely new ecosystem.

## Decision

`moltbook-pioneer` is **parked**. The component continues to exist as:

1. A submodule at `components/moltbook-pioneer/` with its code, the 25-pattern catalogue, the platform-anatomy notes, and a README that begins with the parked notice and the rationale.
2. A `vault-pioneer` service in `compose.yml`, retained for completeness but excluded from the default startup profile.
3. An entry in [`docs/trifecta.md`](../trifecta.md) §4.3 and [`docs/threat-model.md`](../threat-model.md) T3 marking the architectural slot and noting that layers 1 and 2 (feed-scanner, network isolation) are dormant pending a stable target API.

The component is **not** removed from the repository, the schema, the orchestrator-check suite, or the manifest contract.

## Consequences

### Positive

- **The architectural slot is preserved.** Re-activation requires only the API integration; the perimeter layer (network isolation, scanner pipeline, manifest entry, GUI surface) is in place. The work to stand pioneer up was not wasted.
- **Maintenance load drops to near zero.** The component does not need to keep up with API changes that may not happen, does not need to ship working examples, and does not need to be exercised by integration tests against an unreliable upstream.
- **User-facing confusion is removed.** The parked notice is the first thing a user sees if they navigate into pioneer territory. The decision is documented; the absence is honest rather than mysterious.
- **The threat-model coverage is honest.** [`docs/threat-model.md`](../threat-model.md) T3 explicitly marks layers 1 and 2 as dormant; layers 3–5 (DM pairing policy, tool policy, coordinator approval) continue to apply.

### Negative

- **The four-container architecture story now contains a footnote.** Marketing copy and architecture diagrams need to qualify the "four-container perimeter" claim with "(one parked)" wherever the claim is precise. The README, the whitepaper, and the diagrams already do this consistently.
- **A future re-activation is not free.** Even with the architectural slot preserved, re-activation requires re-validating the API integration against whatever the upstream looks like at that point. The 25-pattern catalogue is calibrated for the 2026-Q1 Moltbook ecosystem and may not transfer cleanly to a successor platform.
- **The submodule continues to exist.** A reader cloning the repository with `--recurse-submodules` still pulls the full pioneer code. This is the right trade-off — losing the architectural slot would be worse — but the disk-space and clone-time cost is non-zero.

### Neutral

- The decision is reversible. If the Moltbook API stabilises or a successor platform appears, the component can be un-parked by removing the `parked` profile, re-enabling the GUI tile, and updating the parked notice. The version log is preserved.

## Alternatives considered

**(A) Remove the component entirely.** Deletes the submodule, the compose service, the manifest entry, the schema row, and all references. Rejected because it loses the architectural slot — re-introducing pioneer (or a successor) later becomes a from-scratch design exercise rather than an integration update. The cost of keeping the parked component is small.

**(B) Continue active development against the unreliable API.** Maintain the integration, accept broken tests as the API drifts, ship updates that may break again at any point. Rejected because the maintenance load was already producing user-facing noise (intermittent health probe failures) and contributor confusion (PRs that fixed the integration were undone by the next API change).

**(C) Re-target pioneer to a different platform.** Pivot the component to scan a different agent-social-network for prompt injection — Bluesky's AT-Protocol-based agent communities, Discord agent bots, or Mastodon's emerging agent corner. Rejected as scope creep at this point in the roadmap; the polish phase was the wrong moment to add a new integration. Re-targeting remains available as a future option.

**(D) Park behind a feature flag rather than at the compose layer.** Keep the pioneer service in the default startup profile and gate it on a runtime flag. Rejected because the compose-profile mechanism is the more honest signal: the component is not "off behind a flag", it is genuinely not running in the default configuration. A reader inspecting `compose.yml` sees the component is parked rather than discovering it via a configuration value.

## References

- Architecture: [`docs/trifecta.md`](../trifecta.md) §4.3 (the parked notice in the architecture document)
- Threat model: [`docs/threat-model.md`](../threat-model.md) T3 (residual coverage given the parked status)
- Whitepaper: [`docs/whitepaper.md`](../whitepaper.md) §3.2 and §4.3 (parked-status framing)
- Component README: [`components/moltbook-pioneer/README.md`](../../components/moltbook-pioneer/README.md)
- Compose configuration: [`compose.yml`](../../compose.yml) — `vault-pioneer` service entry
- External: Meta's acquisition of Moltbook announced 2026-03-10 (industry press, multiple sources); Moltbook public-API behaviour from 2026-04-05 onwards documented in maintainer integration logs
