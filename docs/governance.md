# Governance

This document describes how decisions are made in OpenTrApp, who makes them, and
how that is intended to evolve. It is deliberately honest about the project's
current size: OpenTrApp is, as of this writing, a **single-maintainer** project,
and the governance model says so plainly rather than implying a structure that does
not yet exist.

## Current model: single maintainer (BDFL)

| Role | Who | Responsibilities |
|------|-----|------------------|
| Maintainer / lead | [@albertdobmeyer](https://github.com/albertdobmeyer) | Final say on architecture, releases, and merges; security contact; release signing. |

All changes reach `main` through a pull request that passes the full CI gate
(`main` is PR-only and branch-protected). The maintainer self-merges after green
because there is currently no second person to approve — this is the explicit
limitation the [roadmap](roadmap.md) and the bus-factor section below acknowledge.

## How decisions are made

- **Day-to-day changes** (bug fixes, tests, docs, dependency bumps): a PR that
  passes the [test gates](../CONTRIBUTING.md#test-gates) and the
  [code-review standards](../CONTRIBUTING.md#maintainers--code-review).
- **Architecturally significant changes** (anything touching the perimeter
  topology, the manifest contract, the security boundary, or a project-wide
  convention): proposed and recorded as an **Architecture Decision Record** in
  [`docs/adr/`](adr/). ADRs are the durable record of *why* a decision was made;
  they are not overturned silently — a reversal is itself a new ADR that supersedes
  the old one (e.g. ADR-0009 superseded ADR-0006).
- **Security-relevant changes**: additionally flagged per
  [`CONTRIBUTING.md` §Security-sensitive contributions](../CONTRIBUTING.md) and
  measured against the [threat model](threat-model.md). A change that weakens a
  boundary must justify itself against the
  [assurance case](assurance-case.md).

## Contribution path

Anyone may contribute. The full process — cloning, building, the test gates, the
reserved-term rule, the DCO sign-off, and the PR workflow — is in
[`CONTRIBUTING.md`](../CONTRIBUTING.md). Contributions are accepted on technical
merit and fit with the project [values](../README.md#values); there is no
contributor-license-agreement beyond the Developer Certificate of Origin.

## Becoming a maintainer

The project actively wants to grow beyond one person — it is the single
highest-leverage improvement to its resilience and its review quality. The path:

1. A track record of merged, high-quality PRs and constructive review comments.
2. Demonstrated understanding of the security model (the perimeter, the threat
   model, the verification discipline in [`CLAUDE.md` §11](../CLAUDE.md)).
3. Invitation by the existing maintainer, with `write` (or higher) access and a
   listing in [`CONTRIBUTING.md` §Maintainers](../CONTRIBUTING.md).
4. Onboarding per [`CONTRIBUTING.md` §Onboarding a new maintainer](../CONTRIBUTING.md)
   — including enabling two-factor authentication, which becomes a requirement for
   anyone with merge rights.

When a second maintainer exists, branch protection will be raised to require at
least one approving review on every PR (it is held off today only because it would
block the sole maintainer's own merges).

## Bus factor — stated honestly

The current **bus factor is 1**. If the sole maintainer became unavailable, the
project would stall until another maintainer was established. Mitigations in place:

- Everything needed to build, test, release, and reason about the project is in the
  repository and public: architecture ([`trifecta.md`](trifecta.md)), the decision
  history ([ADRs](adr/)), the [threat model](threat-model.md), the
  [assurance case](assurance-case.md), the [release process](../CONTRIBUTING.md),
  and the [reproducibility recipe](reproduce.md). There is no private knowledge
  required to continue the work.
- The MIT license permits any party to fork and continue.

This limitation is not hidden; resolving it (a second maintainer) is tracked on the
[roadmap](roadmap.md) as the project's top open-source-health item.

## Code of conduct

Participation is governed by the [Code of Conduct](../CODE_OF_CONDUCT.md). Reports
go to the maintainer via the contact in that document.

## Changing this document

Governance changes are themselves proposed via PR and, when significant, recorded
as an ADR.
