# Lobster-TrApp Master Roadmap v4 — Finalization

**Date:** 2026-04-07
**Previous:** `docs/superpowers/plans/2026-04-04-master-roadmap-v3.md` (superseded)
**Current state:** Ecosystem 85-100% complete. All three components operational with passing test suites. GUI functional with manifest-driven discovery, command execution, streaming, and config editing.
**Cross-reference:** `docs/trifecta.md` (module relationships), per-repo `docs/roadmap.md` (module-level detail)

> **Note (2026-04-16):** The Architecture v2 perimeter redesign (2026-04-15, commit c89c7ca) introduced a parallel phase track (Phases 1-7) covering containerization of forge/pioneer, compose topology, schema evolution, workflow executor, and workflow UI. Phases 1-5 of that track are complete. This roadmap's Phases F-J remain valid for release infrastructure and polish. See `docs/superpowers/specs/2026-04-15-architecture-v2-perimeter-redesign.md` (v2 spec) and `docs/handoff.md` (current status).

---

## Product

A desktop app that lets anyone safely run OpenClaw on their personal computer, controlled from their phone, without risking their digital life.

**Repos:** openclaw-vault (containment) + clawhub-forge (skill security) + moltbook-pioneer (ecosystem tools) + lobster-trapp (GUI + landing page)

---

## What's Done

### v3 Phases (A-E) Status

| v3 Phase | Status | Notes |
|----------|--------|-------|
| **Phase A:** Pioneer completion | DONE (2026-04-07) | 6 phases, 48 tests, 3 engagement presets, pattern export with ReDoS hardening |
| **Phase B:** Forge → Vault skill installation | DONE (2026-03-30) | `install-skill.sh` validates clearance reports, `verify-skills.sh` checks trust files |
| **Phase C:** Pioneer → Vault feed scanning | DEFERRED | Spec complete. Pioneer export ready. Blocked on Moltbook domains entering allowlist. |
| **Phase D:** Setup wizard E2E | NOT STARTED | Wizard exists but not wired to real container commands. → **Phase I below** |
| **Phase E:** Landing page + release | NOT STARTED | → **Phase J below** |

### Per-Module Completion

| Module | Phases | Status | Key Metric |
|--------|--------|--------|------------|
| **openclaw-vault** | 8/8 complete | Certified | 24-point verify, 13 test scripts, 3 shell levels |
| **clawhub-forge** | 4/5 complete | Phase 5 deferred (ClawHub API) | 87-pattern scanner, CDR pipeline, 25 skills certified |
| **moltbook-pioneer** | 6/6 complete | Done | 48 tests, 25 injection patterns, 3 engagement presets |
| **lobster-trapp GUI** | Functional | ~85% complete | 10 Tauri commands, 6 renderers, 52 frontend tests, 41 orchestrator checks |

### Cross-Repo Harmonization — Complete (2026-04-06)

- All roadmaps reflect actual state
- GLOSSARY.md, trifecta.md, product-assessment.md current
- Submodule refs synchronized
- 28/28 cross-module integration checks passing

---

## What's Left (Phases F-J)

### Priority Summary

| Tier | Phases | Rationale |
|------|--------|-----------|
| **Tier 1: Ship with confidence** | F (tests/CI), H (bundles/updater) | Can't release without CI catching regressions and installable binaries |
| **Tier 2: Complete the contract** | G (card-grid renderer) | Only display enum without its own implementation |
| **Tier 3: End-to-end polish** | I (wizard E2E, forge polish) | Non-technical users need GUI-only setup |
| **Tier 4: Go public** | J (landing page, release) | People need to find and download the product |

### Dependency Graph

```
Phase F (tests/CI)  ──→  Phase G (card-grid)  ──→  Phase H (bundles/updater)
                                                          │
Phase I (wizard + forge)  ────────────────────────────────┘
                                                          │
                                                     Phase J (release)
```

F and I can run in parallel. G depends on nothing. H blocks J. I blocks J.

---

## Phase F: Test Infrastructure & CI Hardening

**Why:** Confidence to ship. The integration test script (467 lines, 28 checks) exists but runs manually. Frontend test coverage is ~6%. Catching regressions before everything else prevents silent breakage.

**Spec:** `docs/specs/2026-04-07-ci-integration-tests.md`

| Task | Repo | Details |
|------|------|---------|
| F1. Wire integration tests into CI | lobster-trapp | Add `tests/integration-test.sh --ci` as a job in `.github/workflows/ci.yml` after `check-orchestration`. Skip container-dependent checks in CI. |
| F2. Frontend unit tests: renderers | lobster-trapp | Tests for all 7 renderers (`LogRenderer`, `TerminalRenderer`, `ChecklistRenderer`, `TableRenderer`, `BadgeRenderer`, `ReportRenderer`, new `CardGridRenderer`) + `ansi.ts` parseAnsi function. |
| F3. Frontend unit tests: StreamOutput + CommandPanel | lobster-trapp | Two most complex untested interactive components. Mock Tauri IPC for command execution and event streaming. |
| F4. E2E test expansion | lobster-trapp | Playwright tests for: component detail view, command execution (mocked), settings persistence, setup wizard flow, error states (404, failed command). |

**Exit criteria:** CI runs orchestrator + integration + frontend + Rust + Playwright on every PR. Frontend test coverage above 35%.

---

## Phase G: Card-Grid Renderer

**Why:** The only `OutputDisplay` enum value without a dedicated renderer. Currently aliased to `ReportRenderer` at `app/src/components/OutputRenderer.tsx:27-29`. Completing it means the full schema contract — every enum value has a real implementation.

**Spec:** `docs/specs/2026-04-07-card-grid-renderer.md`

| Task | Repo | Details |
|------|------|---------|
| G1. Design card-grid renderer | lobster-trapp | Define input format detection (JSON vs. section-header heuristics), card anatomy, responsive grid layout. |
| G2. Implement `CardGridRenderer.tsx` | lobster-trapp | New file: `app/src/components/renderers/CardGridRenderer.tsx`. Parse structured output, render as responsive Tailwind CSS Grid. |
| G3. Wire into OutputRenderer.tsx | lobster-trapp | Replace `ReportRenderer` alias at line 27-29 with `CardGridRenderer` import. |
| G4. Tests + adopt in component.yml | lobster-trapp | Unit tests for parsing and rendering. Switch at least one component command to use `display: card-grid` (e.g., forge `stats` or pioneer `agent-census`). |

**Exit criteria:** A command with `display: card-grid` renders structured data as a responsive grid of cards, visually distinct from `ReportRenderer`. At least one component.yml command uses it.

---

## Phase H: Cross-Platform Bundle & Updater

**Why:** Release infrastructure. CI already builds for Linux, macOS (ARM + Intel), and Windows — but `tauri.conf.json` only has a `bundle.windows` section. The updater plugin is installed but `active: false` with an empty pubkey. These are release blockers.

**Spec:** `docs/specs/2026-04-07-bundle-and-updater.md`

| Task | Repo | Details |
|------|------|---------|
| H1. macOS bundle config | lobster-trapp | Add `bundle.macOS` to `tauri.conf.json`: dmg settings, `minimumSystemVersion`, signing identity placeholder. |
| H2. Linux bundle config | lobster-trapp | Add `bundle.linux` to `tauri.conf.json`: targets (deb, AppImage), desktop file metadata, categories. |
| H3. Configure updater | lobster-trapp | Generate signing keypair (`npx tauri signer generate`), set `active: true`, store private key in GitHub Secrets, pubkey in `tauri.conf.json`. |
| H4. Test release workflow | lobster-trapp | Push a `v0.1.0-rc.1` tag. Verify CI produces a draft release with installable binaries + `latest.json` for all 4 platform targets. |

**Exit criteria:** `tauri.conf.json` has platform configs for macOS, Linux, and Windows. Updater enabled with valid pubkey. A git tag produces a draft release with installable binaries for all platforms.

---

## Phase I: Forge Finalization & Setup Wizard E2E

**Why:** Two remaining threads. The setup wizard (v3 Phase D) exists as a 5-step guided flow but isn't connected to real Podman/Docker commands — non-technical users still need a terminal. The forge has three polish items that improve dashboard value.

**Spec:** `docs/specs/2026-04-07-setup-wizard-e2e.md` (wizard portion)

| Task | Repo | Details |
|------|------|---------|
| I1. Wire wizard to real commands | lobster-trapp | The wizard's "Setup" step should trigger each component's `setup_command` from its manifest via the existing `run_command` Tauri handler. Backend already supports this — work is primarily frontend wiring in `app/src/pages/Setup.tsx`. |
| I2. CDR Ollama fallback | clawhub-forge | `tools/lib/cdr-intent.sh:30-35` hard-fails if Ollama is unreachable. Add: (a) cached intent fallback (check for existing `intent.json`), (b) configurable remote endpoint in `config/cdr.conf`, (c) graceful error with actionable message. |
| I3. Health metrics expansion | clawhub-forge | Add to `component.yml` health section: `lint-health` (count of lint-passing skills), `scan-health` (count of clean scans), `test-health` (count of passing tests). Commands already exist as `make lint-all`, `make scan-all`, `make test` with parseable output. |
| I4. skill-create non-interactive fix | clawhub-forge | `tools/skill-create.sh` lines 109/123 check `INTERACTIVE` flag before processing `--commands`/`--tips` flags. Non-interactive mode should accept these args or proceed to AI generation when empty. |

**Exit criteria:** Non-technical user can set up the full stack through the GUI without opening a terminal. CDR degrades gracefully when Ollama is unavailable. Forge dashboard shows 3+ health badges.

---

## Phase J: Landing Page & Release Prep

**Why:** People need to find and understand the product before downloading. This is the final step from project to product.

| Task | Repo | Details |
|------|------|---------|
| J1. Static landing page | lobster-trapp | GitHub Pages at lobster-trapp.com. Hero section, three-module explainer (Vault/Forge/Pioneer), download buttons per platform, screenshot or demo. Uses `docs/index.html` as base. |
| J2. README polish | all repos | Final pass on all 4 READMEs for a public audience. Remove internal references, add badges, standardize structure. |
| J3. Security audit for public repos | all repos | Grep for secrets, internal URLs, personal paths. Verify `.gitignore` covers `.env`, credentials. Verify no API keys in git history. |
| J4. Make repos public | all repos | Flip visibility on GitHub. Verify submodule URLs still resolve. |
| J5. First tagged release | lobster-trapp | `v0.1.0` tag triggers CI → release with binaries + update manifest. Announce. |

**Exit criteria:** A stranger landing on lobster-trapp.com understands the product and can download it within 30 seconds. All repos public. `v0.1.0` release published with binaries for Linux, macOS, and Windows.

---

## Explicitly Deferred

| Item | Repo | Reason | Trigger to Revisit |
|------|------|--------|---------------------|
| Feed scanning integration (v3 Phase C) | vault + pioneer | Moltbook domains not in allowlist. Design spec complete (`pioneer/docs/specs/2026-04-04-vault-integration-design.md`). Pioneer's pattern export ready. | Moltbook API comes online AND domains enter allowlist |
| VM isolation (Vault Phase 9+) | vault | Placeholder scripts at `phase2-vm-isolation/`. Container isolation proven and sufficient. | User demand for hardware-level isolation beyond containers |
| CI auto-publish | forge | ClawHub API liveness unverified. Publish step commented in `.github/workflows/skill-ci.yml:82-92`. | ClawHub API confirmed live and stable |
| GPG signing for certificates | forge | Integrity verified via SHA-256 checksums. GPG adds trust chain but requires key ceremony. | Public registry integration or enterprise deployment |

---

## Spec Files

| Spec | Phase | Location |
|------|-------|----------|
| Card-Grid Renderer | G | `docs/specs/2026-04-07-card-grid-renderer.md` |
| CI Integration Tests | F | `docs/specs/2026-04-07-ci-integration-tests.md` |
| Bundle & Updater | H | `docs/specs/2026-04-07-bundle-and-updater.md` |
| Setup Wizard E2E | I | `docs/specs/2026-04-07-setup-wizard-e2e.md` |

---

## Per-Repo Roadmap Cross-References

- **openclaw-vault:** No changes needed. Phases 1-8 complete. Phase 9+ (VM isolation) already documented as aspirational.
- **clawhub-forge:** Phase 5a added to `docs/roadmap.md` referencing tasks I2-I4 from this document.
- **moltbook-pioneer:** No changes needed. All 6 phases complete.

---

*This roadmap supersedes `docs/superpowers/plans/2026-04-04-master-roadmap-v3.md`. Per-module details live in each module's `docs/roadmap.md`. Cross-module strategy lives in `docs/trifecta.md`.*
