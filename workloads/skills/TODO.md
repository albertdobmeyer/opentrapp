# OpenSkill-Forge — TODO

Current actionable items from Phase 1 (Housekeeping). See `docs/roadmap.md` for the full 5-phase plan and `docs/forge-identity-and-design.md` for the authoritative design.

---

## Phase 1: Housekeeping

- [x] Remove duplicate `docs/security-report.md` — keep `docs/research/security-report.md`
- [x] Create `.devcontainer/setup.sh` — referenced in `devcontainer.json` but missing (already existed)
- [x] Fix `coding-agent` skill — included in pipeline (passes scan + verify + test)
- [x] Generate `.trust` files for all 25 skills — 25/25 verified, SHA-256 hashes with line counts
- [x] Add `make trust-all` Makefile target — runs `skill-verify.sh --trust` on all skills

## Upcoming (Phase 2+)

- [x] Build security certificate system (`skill-certify.sh`, `skill-export.sh`)
- [ ] (Deferred) GPG signature support for certificates
- [x] Build Content Disarm & Reconstruction pipeline (CDR — the core innovation)
- [ ] Build AI-assisted skill creation wizard
- [ ] Verify ClawHub API liveness, configure CI auto-publish

---

*Last updated: 2026-04-03*
