# Archive

Historical design documents preserved for chronological reference. The decisions described here have shipped; the current state of the codebase reflects them. The terminology in these documents is older than the current vocabulary (early-2026 design notes use a "cage / arena / safari" allegory and the terms "exoskeleton" and "moat" that have since been replaced by plain technical labels). Read the current documentation in [`docs/`](..) first; consult these only when investigating a specific decision's history.

## Contents

| File | Subject | Status |
|---|---|---|
| `specs/2026-03-30-bot-token-decision.md` | Choice of Telegram bot token format and storage | Implemented |
| `specs/2026-03-30-feed-scanning-deferred.md` | Decision to defer Moltbook feed-scanning into a separate module (now `openagent-social`) | Implemented |
| `specs/2026-03-30-fix-test-scripts.md` | Audit and repair of the test-script suite (5/12 → 12/12 passing) | Implemented |
| `specs/2026-03-30-skill-installation-path.md` | Pipeline from `openskill-forge` certified skill to `vault-agent` workspace | Implemented |
| `specs/2026-03-30-tool-control-system-design.md` | Per-tool whitelisting/blacklisting via YAML manifest; replaced the gear-switching script | Implemented as `scripts/tool-control.sh` and `config/tool-manifest.yml` |
| `specs/2026-03-31-config-integrity-protection.md` | Config-file integrity hash and tamper-detection at startup | Implemented |
| `specs/2026-03-31-soft-shell-design.md` | Soft Shell capability boundary and approval model | Implemented |
| `specs/2026-03-31-trial-run-findings.md` | Findings from the first end-to-end trial run | Acted on |

For terminology that may appear in these documents and has since been replaced, see [`GLOSSARY.md`](../../../../GLOSSARY.md) Section 9 ("Historical term mapping") in the parent repository.
