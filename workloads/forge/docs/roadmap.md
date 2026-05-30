# OpenSkill-Forge Roadmap

**Updated:** 2026-04-07
**Current state:** 25 published skills, 87-pattern scanner, zero-trust verifier, gated publishing pipeline, 168 behavioral test assertions, AI-assisted skill creation wizard. Identity and feature set defined in `docs/forge-identity-and-design.md`.
**Cross-reference:** See `docs/trifecta.md` in the opentrapp root for how this module fits with opencli-container and openagent-social.

---

## Phase 1: Housekeeping — COMPLETED (2026-04-02)

**Why:** Small issues that erode trust in an otherwise solid codebase. Also generates trust files which exercises the entire pipeline end-to-end (serves as a code audit).

| Task | Details | Status |
|---|---|---|
| Remove duplicate `docs/security-report.md` | Keep `docs/research/security-report.md`. Delete `docs/security-report.md`. | Done |
| Create `.devcontainer/setup.sh` | Referenced in `devcontainer.json` but doesn't exist. | Done |
| Fix `coding-agent` skill exclusion | Either add tests and include in pipeline, or explicitly mark as draft. | Deferred — still no tests |
| Generate .trust files for all 25 skills | Run verify pipeline, generate SHA-256 trust manifests. | Done — 25/25 |
| Add `make trust-all` target | Regenerate trust files in one command. | Done |

**Exit criteria:** Clean repo, devcontainer works, all skills have trust files. **MET** (coding-agent deferred to Phase 5).

---

## Phase 2: Security Certificate System — COMPLETED (2026-04-02)

**Why:** The vault needs a machine-readable clearance report to accept forge-vetted skills. This is the bridge between forge and vault.

| Task | Details | Status |
|---|---|---|
| Define clearance report JSON schema | Formalize the certificate format. | Done |
| Build `skill-certify.sh` | Generate certificate from scan+verify+test results. | Done — 4-gate pipeline |
| Build `skill-export.sh` | Package skill directory + certificate for vault transfer. | Done |
| Add `make certify` and `make export` targets | Wire into Makefile. | Done |
| Update `skill-publish.sh` | Attach certificate to published skills. | Done |
| Update component.yml | Add certify and export commands for GUI. | Done |

**Exit criteria:** `make export SKILL=name` produces a skill bundle with security certificate. Vault's `install-skill.sh` can validate it. **MET.**

---

## Phase 3: Content Disarm & Reconstruction (CDR) — COMPLETED (2026-04-03)

**Why:** This is the novel feature that defines the forge's USP. ClawHub has an 11.9% malware rate — traditional scanning catches known patterns but misses novel attacks. CDR rebuilds downloaded skills from semantic intent, destroying any embedded attacks.

| Task | Details | Status |
|---|---|---|
| Design CDR spec | Full spec with architecture, data flow, security boundaries. | Done — `docs/specs/` |
| Build quarantine zone | Directory management, download-to-quarantine, immediate cleanup. | Done |
| Build CDR sanitizer | Extract safe lines from untrusted content using line-classifier.sh. | Done — `cdr-prefilter.sh` |
| Build CDR intent extractor | Send safe lines to isolated LLM (Ollama default), get structured intent. | Done — `cdr-intent.sh` |
| Build CDR generator | Reconstruct clean SKILL.md from intent + template. | Done — stage 6 in orchestrator |
| Build CDR orchestrator | End-to-end: quarantine -> sanitize -> extract -> generate -> verify. | Done — `skill-cdr.sh` (8 stages) |
| Build CDR config | LLM backend selection (Ollama/API), model, prompts. | Done — `config/cdr.conf` |
| Build skill download | Download from ClawHub to quarantine (never to workspace). | Done — `skill-download.sh` |
| Add Makefile targets | `make download`, `make cdr`, `make cdr-download`. | Done |
| Write CDR tests | Test with known-bad fixtures, verify injection destruction. | Done — 9 CDR tests |

**Key rule:** The original downloaded file is NEVER accessible. Binary: clean rebuild or discard entirely.

**Exit criteria:** `make download SKILL=name` downloads, CDRs, verifies, and delivers a clean skill. Tests prove injection attacks are destroyed in reconstruction. **MET.**

---

## Phase 4: AI-Assisted Skill Creation — COMPLETED (2026-04-03)

**Why:** Non-technical users need help writing properly formatted SKILL.md files. Reuses the CDR's LLM infrastructure.

| Task | Details | Status |
|---|---|---|
| Design creation wizard spec | What questions to ask, how to map answers to template. | Done — `docs/specs/2026-04-03-ai-assisted-skill-creation.md` |
| Build guided creation flow | Natural language -> template selection -> AI draft -> pipeline verification. | Done — `tools/skill-create.sh` (interactive + non-interactive) |
| Integration with Ollama/API | Use same LLM backend as CDR for skill drafting. | Done — `tools/lib/create-draft.sh`, `tools/lib/create-tests.sh` |

**Exit criteria:** A non-technical user can describe a skill in plain language and get a verified SKILL.md. **MET** — two end-to-end runs (cli-tool + workflow types) passed on first attempt.

---

## Phase 5a: Finalization Polish

**Why:** Three items that improve dashboard value and developer experience, identified during the ecosystem-wide progress assessment.

**Cross-reference:** See `docs/roadmap-v4-finalization.md` in the opentrapp root (Phase I, tasks I2-I4).

| Task | Details |
|---|---|
| CDR Ollama fallback | `tools/lib/cdr-intent.sh:30-35` hard-fails if Ollama is unreachable. Add cached intent fallback, configurable remote endpoint, graceful error message. |
| Health metrics expansion | Add `lint-health`, `scan-health`, `test-health` to `component.yml` health section. Commands already exist (`make lint-all`, `make scan-all`, `make test`). |
| skill-create non-interactive fix | `tools/skill-create.sh` lines 109/123 skip `--commands`/`--tips` in non-interactive mode. Fix to accept these args or proceed to AI generation when empty. |

**Exit criteria:** CDR degrades gracefully without Ollama. Forge dashboard shows 3+ health badges. Non-interactive skill creation works end-to-end.

---

## Phase 5: CI/CD and Registry Integration — DEFERRED

**Why:** The auto-publish CI job is commented out. For a production release, the pipeline should be automated with certificates.

**Status:** Deferred pending ClawHub API availability. The `clawdhub.com/api/v1` endpoints used by `skill-stats.sh`, `registry-explore.sh`, and `skill-publish.sh` have not been verified as live. When the API becomes accessible, this phase can proceed.

| Task | Details |
|---|---|
| Verify ClawHub API liveness | Confirm `clawdhub.com/api/v1` is accessible, add mock mode if not. |
| Uncomment auto-publish CI | Configure version detection, enable gated publish in CI. |
| Certificate-aware publishing | Published skills include security certificate. |

**Exit criteria:** PRs auto-tested, publishing includes certificates.

---

## Dependency Graph

```
Phase 1 (Housekeeping)
    |
    v
Phase 2 (Certificates) <-- vault needs this format for install-skill.sh
    |
    v
Phase 3 (CDR) <-- the core innovation, depends on certificates for output
    |
    v
Phase 4 (AI Creation) <-- reuses CDR's LLM infrastructure
    |
    v
Phase 5a (Finalization Polish) <-- CDR fallback, health metrics, skill-create fix
    |
    v
Phase 5 (CI/CD) <-- deferred (ClawHub API)
```

---

*This roadmap covers the openskill-forge module only. See `opencli-container/docs/roadmap.md` and `openagent-social/docs/roadmap.md` for the other modules. See `docs/forge-identity-and-design.md` for the full identity, architecture, and design rationale.*

*Last updated: 2026-04-07*
