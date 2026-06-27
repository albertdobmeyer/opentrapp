# OpenTrApp Roadmap

> **⚠️ DEPRECATED — superseded by [`/ROADMAP.md`](../ROADMAP.md), the single canonical roadmap.**
> This was a second roadmap (a "Tier 1–3" framing of the same journey the canonical
> file tracks as the "Rung 1–4" benchmark ladder); two roadmaps drift (this one fell
> stale at v0.7.2 while v0.8.0 shipped and `main` went post-de-Tauri). Per the
> documentation-governance rule (CLAUDE.md §13), roadmap status lives in `ROADMAP.md`
> ONLY. **Do not update this file — update `ROADMAP.md`.** The containment-confidence-
> first ordering it championed is carried in `ROADMAP.md`'s benchmark ladder + CLAUDE.md §11.

The honest framing it preserved: do not call OpenTrApp "recommendable to non-expert
users" until the security boundary is verified through the product's own entrypoints
on capable hardware. **Current version: see [`/ROADMAP.md`](../ROADMAP.md)** (last tagged
release v0.8.0 — pre-de-Tauri; `main` is the post-de-Tauri lean build).

---

## Guiding principle

The roadmap is ordered by *containment confidence*, not by feature count. A feature
does not ship until the security boundary it depends on has been verified at the end
that consumes it; see the verification discipline in
[`CLAUDE.md` §11](../CLAUDE.md). "Builds" and "runs" are necessary, never sufficient.

## Status legend

✅ done · 🔶 in progress · ⬜ planned · 🖥️ needs capable hardware · 👤 needs a person/external party

---

## Now: the recommendability gate (v0.7.x → first stable)

These are the load-bearing items. The project does not describe itself as
"recommendable to non-expert users" until **Tier 1** is green on capable hardware.

| Item | Status | Notes |
|------|--------|-------|
| Boundary self-test on a **cold-started** perimeter | ✅ | `tests/boundary-selftest.sh` (6 checks: network isolation, L7 allowlist, credential injection, L3 egress drop, proxy-CA pinning, read-only skill delivery). **Verified on real hardware 2026-06-16 (PR #112): exit 0, all six pass.** Wired into the supervisor; runs on every resume path by default, incl. wake-on-message (verified 2026-06-26). |
| The **same** self-test on a **resumed** perimeter | 🔶 🖥️ | **Restart-resume path verified (PR #112): exit 0, CA fingerprint unchanged, ×3.** The *production* idle-auto-pause → wake path still needs a live run before this is fully met. Fail-closed on any failure. |
| Idle auto-pause + wake verified **in production** | ⬜ 🖥️ | ADR-0018. Proven on the dev box; needs a production-representative run. |
| Code signing (Windows + macOS) | 🔶 👤 | SignPath Foundation application submitted; Apple Developer enrollment pending. CI is a ready-to-activate template. |

## Next: hardening (makes the claim robust)

| Item | Status | Notes |
|------|--------|-------|
| Proxy memory bounded over (load × time) | 🔶 🖥️ | Attribute and cap `vault-proxy` RSS growth under sustained load. |
| Adversarial / red-team pass | 🔶 🖥️👤 | `tests/red-team-breakout.sh` playbook authored; needs an execution pass on capable hardware. |
| Third-party security review | ⬜ 👤 | An independent reviewer of the perimeter design and implementation. |

## Later: trust polish

| Item | Status | Notes |
|------|--------|-------|
| Cut a **stable** release (not an RC) | ⬜ | Gated on Tier 1. |
| Byte-for-byte reproducible build | ⬜ | SLSA L2 + per-platform SBOM exist; end-to-end reproducibility not yet verified. |
| Residual-risk transparency, front-and-center | ✅ | [`what-this-protects.md`](what-this-protects.md), linked from the README and the threat model. |

## Project-health track (parallel, mostly landed)

Supply-chain and open-source-hygiene work that runs alongside the security gate:

- ✅ Signed releases (cosign keyless + SLSA provenance), per-platform SBOM
- ✅ CodeQL on every commit; `cargo deny` / `cargo audit` / `npm audit` CI gates
- ✅ Fuzzing (`cargo fuzz` against the manifest parser + argument interpolator)
- ✅ DCO sign-off enforced; branch protection (`main` is PR-only, strict checks)
- ✅ Every *fixable* dependency advisory eliminated; upstream Tauri/GTK3 advisories documented in [`known-advisories.md`](known-advisories.md)
- ✅ OpenSSF Best Practices **Passing** badge; **Silver** in progress (see [`openssf-best-practices-application.md`](openssf-best-practices-application.md))

## Explicitly out of scope / not promised

- **Absolute safety.** The application raises the cost of compromise via
  defense-in-depth; it cannot make running an autonomous agent absolutely safe, and
  it says so. See the [threat model](threat-model.md) for every named gap.
- **A remote-management surface.** No network services are exposed; the app is not
  remotely controllable by design.
- **Processing untrusted content on the host.** All skill downloads, scanning, and
  feed processing happen inside containers, never on the host filesystem.
- **The agent-social workload** (`vault-social`) is **opt-in / on-demand**: a live AT Protocol
  adapter shipped ([ADR-0017](adr/0017-unpark-social-live-adapter.md)), though it is not yet
  invoked by a default product command (canned data remains the default). Its full build-out as a
  generalized agent-social shield is the **deferred** third concern (MISSION Thread C /
  [ADR-0024](adr/0024-product-structure-three-concerns.md)); it is not on the critical path.

## How to influence the roadmap

Open a GitHub issue describing the use case or gap. Architectural changes are
proposed as ADRs (see [`docs/adr/`](adr/)). Contributions that broaden agent
compatibility or strengthen the perimeter are explicitly welcomed; see
[`CONTRIBUTING.md`](../CONTRIBUTING.md).
