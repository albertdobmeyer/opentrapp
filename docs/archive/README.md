# Archive

Documents preserved for historical reference but no longer current. Filenames inside this directory are date-prefixed for chronological scanning. The current documentation in [`docs/`](..) and the per-component documentation in [`components/<component>/`](../../components/) supersedes everything here. Read the current documentation first; consult the archive only when investigating a specific commit or milestone.

## Top-level archived documents

| File | Period of currency | Content |
|---|---|---|
| `2026-03-03-todo.md` | Pre-v0.1.0 | Internal TODO list from the early-2026 audit |
| `2026-03-27-vision-and-status.md` | Pre-v0.1.0 | Architecture vision and progress snapshot |
| `2026-04-09-landing-page-handoff.md` | Landing-page setup | Hetzner deployment and domain-publishing handoff |
| `2026-04-16-roadmap-v4-finalization.md` | v0.1.0 / v0.2.0 transition | Roadmap planning before the v0.2.0 architecture |
| `2026-04-24-handoff-pioneer-gaps.md` | Pre-Pioneer-parking | Internal handoff describing remaining `openagent-social` integration work; predates the parking decision |
| `2026-04-24-product-assessment.md` | Pre-v0.2.0 | "Honest pros and cons" internal assessment |
| `2026-04-25-v0.2.0-ship-plan.md` | v0.2.0 ship | Ship plan for the v0.2.0 release |

## Archived specs (`specs/`)

Implementation specifications for features that have shipped or roadmap documents that have been executed. Their plans are reflected in the live source; the documents themselves are no longer load-bearing.

| File | Subject |
|---|---|
| `specs/2026-04-07-bundle-and-updater.md` | Tauri bundling and auto-updater integration (shipped) |
| `specs/2026-04-07-card-grid-renderer.md` | Card-grid renderer (component subsequently removed in the v0.3.0 cleanup) |
| `specs/2026-04-07-ci-integration-tests.md` | CI integration-test wiring (shipped) |
| `specs/2026-04-07-setup-wizard-e2e.md` | Setup-wizard end-to-end harness (shipped) |
| `specs/2026-04-18-ux-redesign.md` | Pre-Pass-6 UX redesign brief |
| `specs/2026-04-19-alignment-roadmap.md` | Pre-Pass-6 frontend / brand alignment roadmap |
| `specs/2026-04-19-frontend-reframe-spec.md` | Pre-Pass-6 frontend reframe spec |
| `specs/2026-04-25-voice-and-calendar-perimeter-extension.md` | Out-of-scope perimeter extension proposal (vault-voice, vault-calendar) |
| `specs/2026-04-30-pass-6-roadmap.md` | Pass-6 roadmap (executed) |

## Archived superpowers (`superpowers/`)

The `superpowers/` subtree was the project's design-and-planning workspace before the Tauri integration was complete. The v2 architecture spec at [`docs/superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md`](../superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md) is still current and remains in the live tree. The pre-v2 specs and all pre-v0.2.0 implementation plans are archived here.

| File | Subject |
|---|---|
| `superpowers/2026-03-23-opencli-container-security-harness-design.md` | Original vault security-harness design (pre-v2) |
| `superpowers/2026-03-25-split-shell-capability1-persistent-memory.md` | Persistent-memory capability spec for Split Shell |
| `superpowers/2026-04-06-cross-module-integration-tests-design.md` | Cross-module integration-test design (shipped) |
| `superpowers/plans/2026-03-23-opencli-container-master-roadmap.md` | Original vault roadmap |
| `superpowers/plans/2026-03-23-phase0-bug-fixes.md` | Phase-0 bug-fix list (executed) |
| `superpowers/plans/2026-03-23-phase1-verify-openclaw-compatibility.md` | Phase-1 OpenClaw-compatibility plan (executed) |
| `superpowers/plans/2026-03-24-phase2-formalize-gear1.md` | Phase-2 plan to formalise the original "Gear 1" (now Hard Shell) |
| `superpowers/plans/2026-03-25-master-roadmap-v2.md` | v2 master roadmap |
| `superpowers/plans/2026-04-04-master-roadmap-v3.md` | v3 master roadmap |
| `superpowers/plans/2026-04-06-cross-module-integration-tests.md` | Plan for cross-module integration tests |

These plans use older project terminology (Gear 1 / 2 / 3 instead of Hard / Split / Soft Shell; Warden instead of CLI coordinator; etc.). The mapping between old and current terms is documented in [`GLOSSARY.md`](../../GLOSSARY.md) Section 9.

## Pointers to current documentation

| For | See |
|---|---|
| Current handoff | [`../handoff.md`](../handoff.md) |
| Current architecture | [`../trifecta.md`](../trifecta.md) |
| Project README | [`../../README.md`](../../README.md) |
| Contributor guide | [`../../CLAUDE.md`](../../CLAUDE.md) |
| Glossary | [`../../GLOSSARY.md`](../../GLOSSARY.md) |
| UX rubric | [`../specs/2026-04-20-ux-principles-rubric.md`](../specs/2026-04-20-ux-principles-rubric.md) |
| Pre-ship audit (Pass 8) | [`../specs/2026-05-02-pass-8-preship-walk.md`](../specs/2026-05-02-pass-8-preship-walk.md) |
| Architecture v2 design spec | [`../superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md`](../superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md) |
