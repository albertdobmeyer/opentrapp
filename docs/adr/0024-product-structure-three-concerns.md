# ADR-0024 — Top-level product structure: one brand, three concern sub-apps, the perimeter is the Vault

**Status:** Accepted — *organizes and clarifies* the structure already established by
[ADR-0009](0009-five-container-perimeter.md) (USP-1), [ADR-0003](0003-content-disarm-reconstruction.md)
+ [ADR-0006](0006-four-container-topology.md) (USP-2), [ADR-0013](0013-monorepo-consolidation.md)
(monorepo) and [ADR-0020](0020-product-identity-and-distribution.md) (daemon-is-the-product).
**No container-topology change.** This ADR records the *mental model* and the *concern decomposition*
the project is narrated and built around, and settles a recurring "why are these separate containers?"
question on its own merits.

**Cross-references:** [ADR-0009](0009-five-container-perimeter.md) ·
[ADR-0003](0003-content-disarm-reconstruction.md) · [ADR-0006](0006-four-container-topology.md) ·
[ADR-0013](0013-monorepo-consolidation.md) · [ADR-0014](0014-monorepo-modular-distribution.md) ·
[ADR-0017](0017-unpark-social-live-adapter.md) · [ADR-0020](0020-product-identity-and-distribution.md) ·
the verify-first decision ([`docs/specs/2026-06-15-windows-session-portability-and-architecture-review.md`](../specs/2026-06-15-windows-session-portability-and-architecture-review.md) §5) ·
[CLAUDE.md §1](../../CLAUDE.md)

---

## Context

OpenTrApp began as **three concern-separated apps** — containerization, skill-scanning, and
agent-social analysis — bundled by a GUI parent. The evolution since (three repos → submodules →
monorepo, [ADR-0013](0013-monorepo-consolidation.md)) plus a habit of narrating the system as
**"five flat containers"** quietly *conflated* that clean separation.

The symptom is legibility, not correctness: the maintainer — and the docs — cannot recite in 60
seconds *what each container is and why the split exists*. [`docs/perimeter-explained.md`](../perimeter-explained.md)
even contradicts itself (`workloads/skills/` in one row, `workloads/forge/` two lines later), and the
status of `vault-social` is stated inconsistently across the docs (most still say "parked," while
[ADR-0017](0017-unpark-social-live-adapter.md) and the v0.6 release notes say it is un-parked/live).

Two prior, correct decisions are load-bearing but under-narrated:

- **USP-1 — privilege separation** ([ADR-0009](0009-five-container-perimeter.md)): no single
  container holds both the API credentials *and* internet access (`vault-proxy` holds keys with no
  internet; `vault-egress` holds `NET_ADMIN`+internet with no secrets).
- **USP-2 — anti-tamper supply-chain defense** ([ADR-0003](0003-content-disarm-reconstruction.md) /
  [ADR-0006](0006-four-container-topology.md)): skills are scanned and Content-Disarm-Reconstructed
  in isolation the agent **cannot tamper with**, then delivered read-only.

A recurring question — *"the skill workshop and the social lab were meant to live inside the vault;
why are they separate containers?"* — comes from a single conflation: treating **"the Vault" as the
agent's container** rather than as the **whole perimeter**. The verify-first decision (#111) rejected
collapsing the workload containers, but argued it via USP-1 + memory; the specific *"merge skills/social
into the agent"* option was never reasoned through on its own merits. This ADR does that.

## Decision

**1. OpenTrApp is ONE brand, decomposed into three concern sub-apps + an optional GUI** — each
independently runnable (composable; *run what you need*, not the whole bundle at once), CLI-first over
the headless daemon ([ADR-0020](0020-product-identity-and-distribution.md)):

| Sub-app | Concern | USP | Form |
|---|---|---|---|
| **Vault** | Containerization — the perimeter | **USP-1** (privilege separation) | The perimeter: a contained agent + the egress split. |
| **Skill** | Supply-chain firewall (scan + CDR) | **USP-2** (anti-tamper vetting) | An isolated container *within the Vault*; **also ships standalone**, no perimeter required (the adoption wedge). |
| **Social** | Agent-social shield | — (neither USP) | Optional; a *second instance of the content-vetting pattern*. The **deferred** third concern. |
| **GUI** | Human projection | — | Optional. The CLI controls every concern. |

**2. "The Vault" is the PERIMETER (the whole containment), not the agent's container.** Every concern
operates *within* the Vault. The mental model the architecture should teach is **one untrusted SUBJECT
(the agent) surrounded by GUARDS**: the **egress guard** (USP-1) and the **skill firewall** (USP-2),
with Social as a second instance of the vetting-guard pattern.

**3. We do NOT collapse Skill (or Social) into the agent's container.** The skill scanner runs in its
**own** container **within the Vault** — *inside the Vault ✓, inside the agent ✗* — because an
inspector the agent can tamper with is not an inspector. Concretely, co-locating the scanner with the
agent would let a compromised agent (the entire threat the product contains) rewrite the pattern
catalogue, **forge its own clearance reports**, and read the quarantined *raw, un-disarmed* bytes; it
also forces the deliberately minimal agent image (`curl`/`wget`/`rm`/`chown` physically deleted) to
carry the scanner's opposite toolchain. **The isolation is USP-2 itself; collapse destroys it.**

**4. The proxy/egress split (USP-1) likewise stays two containers** ([ADR-0009](0009-five-container-perimeter.md)).
Merging recombines credentials with internet/`NET_ADMIN`.

**5. Sequencing:** close **Vault → Skill → GUI** to *done/verified*, then build out **Social**. (The
roadmap detail lives in `MISSION.md` / [`road-to-recommendable.md`](../road-to-recommendable.md), not
in this ADR.)

## Consequences

**Positive**
- A 60-second explanation the structure *itself* teaches: one subject, two guards, each split for one
  sayable reason. The two USPs become the sole story instead of trivia in a five-row table.
- The **Skill firewall becomes independently adoptable** — the realistic opencode wedge (a single
  offline scanner is a far smaller ask than "adopt a five-container perimeter").
- The recurring "why separate?" question is settled with a written rationale; the parked/live `social`
  contradiction gets one canonical answer (opt-in/on-demand; live adapter shipped per
  [ADR-0017](0017-unpark-social-live-adapter.md); full build-out deferred).

**Negative / cost (stated honestly)**
- A naming/narration sweep: docs + diagrams, and the "Workshop/Monitoring Station" labels that teach a
  *tool* framing must flip to *guard* framing to match the egress side ("Gate/Fence/Cell Block").
- The standalone Skill firewall adds a second delivery surface with its own scope/claim boundary →
  ADR-0025 (forthcoming, roadmap Phase 2).
- None of this changes the running topology, so there is **no perimeter risk** — but the docs sweep and
  the CLI/standalone work are real effort.

## Alternatives considered

- **Collapse Skill (and/or Social) into the agent container** (the "one container for all the agent's
  needs" option). **Rejected** — destroys USP-2's anti-tamper property (Decision §3); also produces a
  `node`+`python` dual-runtime container with an entrypoint-dispatch problem and a wider agent attack
  surface. Memory is not a driver (the resting perimeter is ~400 MB, ~0 with idle auto-pause; #111).
- **Keep narrating "five flat containers."** Rejected — it *is* the source of the felt complexity and
  the 60-second failure. The topology is right; the narration is the problem.
- **Two separate brands/products** (a "sandbox" and a "skill-firewall"). Rejected — one brand keeps the
  user's memory focused. The Skill firewall is a *sub-app of OpenTrApp* that also runs standalone, not a
  separate brand.

## What this ADR does NOT decide (staged)

- The **standalone Skill-firewall scope** + its scan-on-host claim boundary → **ADR-0025**.
- The **de-Tauri / GUI-demotion** mechanics → [ADR-0022](0022-daemon-control-surface.md).
- The **agent-recipe abstraction** that makes the Vault wrap agents other than OpenClaw (e.g. opencode)
  → a forthcoming ADR (Phase 4 of the roadmap).
