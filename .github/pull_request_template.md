<!--
Thank you for the contribution!

Please fill in the sections below. The maintainer will ask the same questions during review, so filling them in up front speeds things up. If a section doesn't apply, write "n/a" with one short reason — that's faster than guessing whether it applies.

Code of Conduct: https://github.com/albertdobmeyer/lobster-trapp/blob/main/CODE_OF_CONDUCT.md
Contributing guide: https://github.com/albertdobmeyer/lobster-trapp/blob/main/CONTRIBUTING.md
-->

## Summary

<!-- One or two sentences. What does this change do, and why? Prefer the why; the diff already shows the what. -->

## Linked issue

<!-- e.g. "Closes #42" — for non-trivial changes, an issue should already exist. -->

## Type of change

- [ ] Bug fix (non-breaking change which fixes a defect)
- [ ] New feature (non-breaking change which adds capability)
- [ ] Breaking change (fix or feature that changes existing behavior in a non-backward-compatible way)
- [ ] Documentation update
- [ ] Refactor / cleanup (no functional change)
- [ ] Build / CI / tooling change
- [ ] Security-relevant (touches the perimeter, the proxy, the scanner, the runner, the validation logic, or the verification scripts)

## Test gates

Please confirm the five gates are green locally before marking the pull request ready for review.

- [ ] `cd app/src-tauri && cargo test --lib` passes
- [ ] `cd app && npm test -- --run` passes
- [ ] `cd app && npx tsc --noEmit` passes
- [ ] `cd app && npx playwright test` passes
- [ ] `bash tests/orchestrator-check.sh` passes (0 warnings)

## Manifest contract

If this change touches the manifest schema or any of its three implementations:

- [ ] `schemas/component.schema.json` updated
- [ ] `app/src-tauri/src/orchestrator/manifest.rs` updated
- [ ] `app/src/lib/types.ts` updated
- [ ] `tests/orchestrator-check.sh` still passes
- [ ] n/a — this change does not touch the schema

## User-facing surface

If this change adds, changes, or removes user-visible text:

- [ ] No reserved terms (the 28 in `app/e2e/user-facing.spec.ts`) appear in user-mode visible text
- [ ] New developer-jargon terms have either a [`GLOSSARY.md`](../GLOSSARY.md) mapping or are added to the reserved-term list with rationale
- [ ] n/a — this change does not change user-visible text

## Documentation

- [ ] Updated relevant `*.md` files for the change
- [ ] [`CLAUDE.md`](../CLAUDE.md) is consistent with the change (or n/a)
- [ ] [`README.md`](../README.md) is consistent with the change (or n/a)

## Screenshots / output

<!-- For UI changes, please attach before/after screenshots.
For CLI or backend changes, please paste relevant terminal output. -->

## Notes for the reviewer

<!-- Anything the reviewer should pay particular attention to. Edge cases you considered, alternatives you ruled out, follow-ups you intend to file separately. -->
