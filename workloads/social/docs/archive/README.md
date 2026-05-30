# Archive

Historical implementation artifacts for the openagent-social module. The decisions described here have shipped; the current state of the codebase reflects them. Read the current documentation in [`docs/`](..) first; consult these only when investigating a specific decision's history.

The module itself has been **parked since 2026-05-03** following Meta's acquisition of Moltbook on 2026-03-10 and the resulting API instability. See the banner in the repository root [`README.md`](../../README.md) for the full context.

## Specs (`specs/`)

| File | Subject | Implementation |
|---|---|---|
| `specs/2026-04-04-vault-integration-design.md` | Pioneer ↔ vault-agent integration: how feed-scan results reach the agent through the perimeter | Implemented (defined in `component.yml` and `opentrapp/compose.yml`) |
| `specs/2026-04-05-regex-security-hardening.md` | Hardening the injection-pattern regex engine against PCRE / ERE compatibility issues | Implemented |
| `specs/2026-04-07-engagement-presets.md` | Three engagement-level presets (Observer, Researcher, Participant) with GUI commands | Implemented |

## Superpowers (`superpowers/`)

Earlier-form planning documents that were later refined into the specs above.

| File | Subject |
|---|---|
| `superpowers/2026-04-05-regex-security-hardening.md` | Initial regex-hardening plan |

## Handoff documents (`handoffs/`)

Phase-completion handoff notes from the original development sessions.

| File | Subject |
|---|---|
| `handoffs/2026-04-04-pioneer-completion.md` | Pioneer phases 1-2 complete |
| `handoffs/2026-04-05-pioneer-implementation.md` | Pioneer phases 3-5 implementation handoff |
| `handoffs/2026-04-05-regex-security.md` | Regex-security-hardening implementation handoff |
| `handoffs/2026-04-05-regex-verification.md` | Verification handoff for the regex-hardening work |
| `handoffs/2026-04-06-report-regex-verification.md` | Independent verification report by a fresh implementation session |

## Loose decision records

| File | Subject |
|---|---|
| `2026-04-05-pattern-harmonization.md` | Decision to keep Pioneer and Forge pattern catalogues separate (different threat surfaces, different consumers, low overlap) |

For terminology that may appear in these documents and has since been replaced, see [`GLOSSARY.md`](../../../../GLOSSARY.md) Section 9 ("Historical term mapping") in the parent repository.
