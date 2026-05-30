# Archive

Historical design documents preserved for chronological reference. The decisions described here have shipped; the current state of the codebase reflects them. Read the current documentation in [`docs/`](..) first; consult these only when investigating a specific decision's history.

## Specs (`specs/`)

| File | Subject | Implementation |
|---|---|---|
| `specs/2026-04-02-content-disarm-reconstruction.md` | Content Disarm & Reconstruction pipeline design | Implemented as `tools/skill-cdr.sh` and `tools/lib/cdr-*` |
| `specs/2026-04-02-security-certificate-system.md` | Skill clearance-report (SHA-256 + JSON certificate) format | Implemented as `tools/skill-certify.sh` |
| `specs/2026-04-03-ai-assisted-skill-creation.md` | AI-assisted skill scaffolding with pipeline gating | Implemented as `tools/skill-create.sh` and `tools/lib/create-*` |

## Superpowers (`superpowers/`)

Earlier-form planning documents that were later refined into the specs above. Kept for traceability of how the designs evolved.

| File | Subject |
|---|---|
| `superpowers/2026-04-02-content-disarm-reconstruction.md` | Initial CDR plan |
| `superpowers/2026-04-02-security-certificate-system.md` | Initial certificate-system plan |

## Internal handoffs (`internal/`)

| File | Phase |
|---|---|
| `internal/2026-04-03-handoff-phase4.md` | Phase 4 (AI-assisted skill creation) implementation handoff |
| `internal/2026-04-03-handoff-phase4-cleanup.md` | Phase 4 cleanup (CRLF normalisation, .trust generation, test fixes) |

For terminology that may appear in these documents and has since been replaced, see [`GLOSSARY.md`](../../../../GLOSSARY.md) Section 9 ("Historical term mapping") in the parent repository.
