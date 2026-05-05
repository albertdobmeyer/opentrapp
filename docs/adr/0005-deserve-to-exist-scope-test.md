# ADR-0005 — The "deserve-to-exist" scope test

**Status:** Accepted
**Decision date:** 2026-05-02 (mid-Pass-7 vision recheck)
**Implemented by:** [`docs/specs/2026-05-02-pass-8-preship-walk.md`](../specs/2026-05-02-pass-8-preship-walk.md) (the Pass-8 audit operationalises the test); reflected in `app/src/components/` removals during Pass 7 Day 1b
**Verified by:** Pass-8 audit verdict — every shipped surface walked, "would removing this make the product worse?" answered explicitly per surface

---

## Context

Halfway through the polish phase, the project had drifted toward the shape of an open-ended dashboard: a Home screen with three tiles (status, spending, activity), a Preferences page with six sections, plans for an extended notification surface, and an admin-key-based spending feature that was implemented and then almost-deployed.

Two observations crystallised at the 2026-05-02 vision recheck:

1. **Most of the dashboard surface was duplicating capabilities that better services already provided.** Anthropic Console covers spend, usage, and billing better than this application can. Telegram covers chat history, message threading, and search better than this application can. An "activity" tile in the desktop app was effectively a worse Telegram client.
2. **The application's defensible value proposition is the safe front door, not the dashboard.** What the application uniquely provides is the perimeter (containers, proxy, scanner, CDR pipeline) plus the onboarding scaffolding that makes the perimeter installable by a non-developer user. That value is concentrated in the wizard and the perimeter-lifecycle controls. Dashboard features beyond that are net-neutral at best.

A scope test was needed to make this judgement structurally rather than as a series of one-off subjective calls.

## Decision

Every user-facing surface in the application must answer **yes** to all three of the following questions, otherwise it is a candidate for removal:

1. **Is this unique to running OpenClaw safely on a personal computer?** If a third-party service (Anthropic Console, Telegram, the operator's own filesystem) covers the same need better, the surface is duplicative.
2. **Would the product be meaningfully worse without this?** A surface that can be removed without the user noticing is, by definition, not pulling its weight.
3. **Does this surface's mental model match the user's mental model?** If a feature requires the user to understand a developer concept (containers, manifests, shell levels, proxy rules) to use it, the surface is leaking the wrong mental model.

A "no" to question 1 is grounds for removal. A "no" to question 2 is grounds for either removal or merging into another surface. A "no" to question 3 is grounds for either rewording or re-architecting the surface to operate at the user's mental-model layer.

The test is operationalised by the Pass-8 pre-ship audit ([`docs/specs/2026-05-02-pass-8-preship-walk.md`](../specs/2026-05-02-pass-8-preship-walk.md)), which walks every shipped surface and records the test's verdict explicitly per surface.

## Consequences

### Positive

- **The Spending feature was unwound.** ~1,090 lines of frontend code, six Preferences sections, three Tauri commands, and a 256-bit-admin-key persistence layer were removed because the spending data is better consumed at [console.anthropic.com/cost](https://console.anthropic.com/cost). The replacement is a single deep-link tile.
- **The Activity tile was removed.** A four-tile Home grid became a two-tile Home grid (Status, Spending-deep-link). The result reads cleaner and matches what the user actually wants from a desktop status surface.
- **The "Anthropic API key" → "AI account key" rename was dropped.** The user does not care what the key is called; they care that pasting it works. The original developer-jargon term — already familiar to users from the Anthropic Console — was kept.
- **The `spendingLimit` settings field, the spending-limit alert, the spending-limit slider, and three associated test rows were all dropped.** The product no longer offers spending alerting; the operator's own spend cap on the API key is the security boundary (per [`docs/threat-model.md`](../threat-model.md) T1's residual risks).
- **Pass 7 shrank from ~4 days of work to ~2.5 days.** The surplus time went into Pass 8 polish.
- **The Pass-8 audit was honest.** Every surface had to defend itself. The audit's "deserve-to-exist" sweep produced a 100 % pass rate (no surface flagged for removal) — the test had already been applied iteratively during Pass 7.

### Negative

- **Some users will want what was removed.** A user who would value a built-in spending dashboard, a built-in activity timeline, or a richer notification subsystem is being deferred to upstream services. The trade-off is documented in the limitations sections of the README and the whitepaper; the application is honest about what it is not.
- **The test is judgement-heavy.** Question 2 ("would the product be meaningfully worse?") cannot be answered mechanically. Different reviewers may answer it differently; the audit's 100 % pass rate reflects one reviewer (the maintainer's) judgement at one moment.
- **A surface that fails the test cannot always be removed cleanly.** Some removals would have left other surfaces incoherent (e.g. removing the Status tile would leave the Home screen empty). A surface that fails the test should be re-examined against its dependencies, not just deleted.

### Neutral

- The test does not produce a single ranking; it produces a per-surface verdict that combines with other rubric scores (see [`docs/specs/2026-04-20-ux-principles-rubric.md`](../specs/2026-04-20-ux-principles-rubric.md)). A surface can pass deserve-to-exist while still scoring poorly on UX-rubric principles, and vice versa.

## Alternatives considered

**(A) Trust the rubric alone.** The 13-principle UX rubric already in place would catch most failure modes. Rejected because the rubric scores *quality* of execution; it does not ask the prior question of *whether the surface should exist at all*. A polished version of a redundant feature still scores well on the rubric and still fails the user.

**(B) Apply the test post-hoc only at Pass-8.** Wait until the pre-ship audit and remove anything that fails. Rejected because removing surfaces at Pass-8 produces churn (cascading edits to navigation, copy, tests) that is more expensive than catching the failure during the surface's own pass. The test is applied as surfaces are designed.

**(C) Defer entirely to a feedback channel ("ship it; remove what nobody uses").** Ship a maximalist surface and prune based on usage telemetry. Rejected because the application has no telemetry (intentionally — see the privacy stance in [`SECURITY.md`](../../SECURITY.md)), so the data needed for this approach does not exist.

**(D) Adopt a strict "less is more" prior.** Default to removing any surface that is not strictly required. Rejected as too aggressive; some surfaces the test approves (the Wizard, the Hero status card, the Pause/Resume controls) are not *strictly* required (a CLI-only product would be possible) but earn their existence by being how the user experiences the product.

## References

- The recheck moment: feedback-memory `feedback_lobster_trapp_scope.md` (the user's articulation of the test in conversation, 2026-05-02)
- The audit that operationalises it: [`docs/specs/2026-05-02-pass-8-preship-walk.md`](../specs/2026-05-02-pass-8-preship-walk.md)
- The unwound Spending work: commit `c052601` (the Pass-7 Day-1b removal pass, -1,090 / +42 lines)
- The companion rubric: [`docs/specs/2026-04-20-ux-principles-rubric.md`](../specs/2026-04-20-ux-principles-rubric.md)
- The product-identity document the test refines: [`docs/specs/2026-04-19-product-identity-spec.md`](../specs/2026-04-19-product-identity-spec.md)
