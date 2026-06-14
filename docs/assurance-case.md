# Security Assurance Case

An assurance case is a structured, auditable argument that the software is
*adequately secure for its stated purpose*, linking each security **claim** to the
**argument** for it and the **evidence** that supports it. This document is that
argument for OpenTrApp. It is deliberately scoped by the project's honest promise:
OpenTrApp **raises the cost of compromise** when running an autonomous CLI agent; it
does **not** claim absolute safety.

Related: the [threat model](threat-model.md) enumerates the attackers and gaps; the
[architecture](trifecta.md) describes the mechanisms; this document ties claims to
evidence. Where a claim depends on a check that can only run on capable hardware,
that is stated — an unverifiable claim is recorded as *unverified*, not *met*.

---

## Top-level claim

> **C0 — Running an autonomous CLI agent inside OpenTrApp is meaningfully safer than
> running it directly on the host, and the residual risks are documented.**

This decomposes into five sub-claims (C1–C5). Each lists its argument and evidence,
and — per the [verification discipline](../CLAUDE.md) — the *consumption-end* check
that confirms it, plus its current verification status.

---

### C1 — The agent cannot reach the network except through the policy proxy

**Argument.** The agent container (`vault-agent`) sits on `internal: true` compose
networks with no default gateway; the only egress path is `vault-proxy` (L7
allowlist) over `vault-egress` (L3 filter). See [ADR-0009](adr/0009-five-container-perimeter.md).

**Evidence.**
- `compose.yml` networks marked `internal: true` (no host/public route).
- Boundary self-test **B1** (network isolation) and **B4** (`vault_egress_drop_private`
  nftables rule) in [`tests/boundary-selftest.sh`](../tests/boundary-selftest.sh).
- L7 allowlist denial returns 403 — self-test **B2**.

**Consumption-end check.** `make boundary-selftest` on a running perimeter:
B1/B2/B4 pass. **Status:** 🔶 verified on the dev box for individual checks; full
cold+resume run gated on capable hardware (roadmap Tier 1).

### C2 — The vendor API credential is never exposed to the agent

**Argument.** The API key is injected by `vault-proxy` at the egress boundary, not
held in the agent's environment ([ADR-0001](adr/0001-proxy-side-api-key-injection.md)).
The agent sends unauthenticated requests; the proxy adds the credential.

**Evidence.**
- Proxy-side injection in `infra/proxy/`.
- Boundary self-test **B3** (vendor-credential injection: the key is present at the
  proxy, absent in the agent environment).
- The host's `.env` and stored credentials are never mounted into `vault-agent`.

**Consumption-end check.** Self-test B3 asserts the key reaches the *upstream* call
but is not readable inside `vault-agent`. **Status:** 🔶 (same hardware gate as C1).

### C3 — Untrusted content is never processed on the host

**Argument.** All skill downloads, scanning, and reconstruction happen inside
`vault-skills` via Content Disarm & Reconstruction
([ADR-0003](adr/0003-content-disarm-reconstruction.md)): the original downloaded
file is never executed or delivered — it is rebuilt from extracted intent or
discarded. Certified skills reach the agent through a volume that is **read-only**
on the agent side.

**Evidence.**
- CDR pipeline + 87-pattern scanner in `workloads/skills/`.
- Boundary self-test **B6** (read-only skill delivery).
- The host filesystem is not exposed to any workload container (the
  [generic-backend constraint](../CLAUDE.md) keeps untrusted-content handling out of
  the host-side Rust/React code).

**Consumption-end check.** Self-test B6; the skills test suite (scanner self-test,
CDR/disarm). **Status:** ✅ for the scanner/CDR unit suites; 🔶 for the in-perimeter
read-only delivery assertion (hardware gate).

### C4 — A rebuilt or resumed perimeter is held to the same boundary as a cold start

**Argument.** A boundary that is "alive but subtly wrong" is worse than a visible
failure. After any (re)start or idle-resume, the perimeter must pass the **same**
self-tests as a fresh cold start before being reported healthy; any failure holds
**fail-closed** ([ADR-0018](adr/0018-idle-auto-pause-host-waker.md), [CLAUDE.md §11](../CLAUDE.md)).

**Evidence.**
- `selftest.rs` + `supervisor.rs` `verify_boundary_fail_closed` (Fail → stop +
  `BOUNDARY_FAILED` marker; CannotAssess → alert; Pass → clear).
- Fail-closed exit-code contract (0 pass / 1 boundary failed / 2 cannot-assess).

**Consumption-end check.** Resume the perimeter, confirm the self-test runs and
fail-closes on an injected boundary fault. **Status:** ⬜ **unverified on capable
hardware** — implemented and opt-in-gated; not yet exercised end-to-end in
production. Recorded as unverified, not met.

### C5 — The residual risks are named, not hidden

**Argument.** The project's value of honesty-about-residual-risk requires that every
gap the perimeter does *not* close is documented in user- and expert-facing terms.

**Evidence.**
- [`what-this-protects.md`](what-this-protects.md) — plain-language residual risk,
  linked from the README.
- [`threat-model.md`](threat-model.md) — attacker categories T1–T6 with the gaps
  named (e.g. host-mediated allowlist loosening under T1; approval fatigue under T5).
- [`known-advisories.md`](known-advisories.md) — accepted upstream advisories and
  the honest Scorecard interpretation.

**Consumption-end check.** A reader can enumerate the residual risks from public
docs without reading source. **Status:** ✅.

---

## What this assurance case does **not** claim

- It does not claim the agent's *reasoning* is safe — reasoning is delegated to the
  vendor API and is out of the perimeter's control.
- It does not claim immunity to a host-level compromise that precedes the perimeter
  (the host is the trust root).
- It does not claim byte-for-byte reproducibility yet (roadmap "Later").
- It does not claim the C4 resume contract is verified in production (see C4).

## Maintenance

This case is revisited when a boundary mechanism changes, when an ADR alters the
topology, or when a self-test is added or removed. Each sub-claim's status is kept
honest: a claim whose consumption-end check cannot be run here is marked
*unverified*, and the [roadmap](roadmap.md) tracks moving it to *met* on capable
hardware.
