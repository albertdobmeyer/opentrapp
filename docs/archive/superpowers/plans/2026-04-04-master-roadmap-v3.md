# Lobster-TrApp Master Roadmap v3

**Updated:** 2026-04-04
**Previous:** `2026-03-25-master-roadmap-v2.md` (superseded — listed Phases 3-6 as future but they're done)

**Product:** A desktop app that lets anyone safely run OpenClaw on their personal computer, controlled from their phone, without risking their digital life.

**Repos:** openclaw-vault (containment) + clawhub-forge (skill security) + moltbook-pioneer (ecosystem tools) + lobster-trapp (GUI + landing page)

**Domain:** lobster-trapp.com

---

## What's Done

### Vault (openclaw-vault) — 8/8 Phases Complete

| Phase | What It Proved |
|-------|---------------|
| Phase 1: Doc cleanup | Clean foundation, terminology migrated from Gear to Shell |
| Phase 2: Monitoring | Network log parser, session reports, log rotation — all operational |
| Phase 3: Split Shell | Shell-aware verification, per-shell allowlists, 12 test scripts, read-chat.sh |
| Phase 4: Tool control | Per-tool whitelisting/blacklisting, tool-manifest.yml, tool-control.sh |
| Phase 5: Cross-module | Skill installation path defined, bot token decision made, feed scanning planned |
| Phase 6: Hardening | Config read-only mount, /bin/rm stripped, attack surface tests formalized |
| Phase 7: Soft Shell | 17 tools, on-miss approval, 28 safeBins, web search + cron + process enabled |
| Phase 8: Certification | All 3 shells verified, 24/24 security checks, round-trip Hard↔Split↔Soft passes |

**Current state:** All three shell levels operational. 24-point verification. Per-tool whitelisting with risk scoring. Monitoring tools (network, session, audit, log rotation) all implemented.

### Forge (clawhub-forge) — 4/5 Phases Complete

| Phase | What It Proved |
|-------|---------------|
| Phase 1: Housekeeping | 25 trust files, devcontainer, coding-agent fixed, 12/12 workbench green |
| Phase 2: Certificates | skill-certify.sh (4-gate), skill-export.sh, vault-compatible clearance reports |
| Phase 3: CDR | 8-stage pipeline: quarantine → parse → prefilter → Ollama intent → validate → reconstruct → post-verify → deliver |
| Phase 4: AI creation | Interactive + non-interactive wizard, Ollama-powered drafting + test generation |
| Phase 5: CI/CD | **DEFERRED** — blocked on ClawHub API availability |

**Current state:** 87-pattern scanner, zero-trust verifier, CDR pipeline, security certificates, AI skill creation wizard, 25 skills with trust files, 168 behavioral test assertions + 37 tool tests.

### GUI (lobster-trapp app) — Functional

| Area | Status |
|------|--------|
| Manifest discovery | Complete — glob-based component.yml detection |
| Command execution | Complete — injection-safe arg interpolation, streaming output |
| Status probes | Complete — exit_code, stdout_contains, stdout_regex |
| Health probes | Complete — regex, json_path, line_count, exit_code |
| Config editing | Complete — YAML validation, path traversal prevention |
| Setup wizard | Complete — 5-step guided configuration |
| Output rendering | 6 renderers (log, terminal, checklist, table, badge, report) |
| Tests | 52 frontend + 17 Rust + 40 orchestrator checks |
| CI/CD | Cross-platform builds (Linux, macOS ARM/Intel, Windows) |

### Cross-Repo Harmonization — Complete (2026-04-04)

- All roadmaps reflect actual state (no stale "future work" for done phases)
- Internal handoff docs archived to `docs/internal/` in vault and forge
- Parent CLAUDE.md and README cleaned up
- Submodule refs synchronized
- GLOSSARY.md, trifecta.md, product-assessment.md all current

---

## What's Next (Priority Order)

### Phase A: Complete Moltbook-Pioneer — Bugs + Tests DONE (2026-04-04)

**Status:** Phase 1 (bug fixes) and Phase 2 (test framework) completed in a single 11-commit session. All 7 known bugs fixed, 16 behavioral tests passing, Makefile with standard targets, safe_patterns wired and tested. Two latent bugs discovered and fixed during testing: `(?i)` PCRE flag broke grep ERE matching, and `|` delimiter collided with regex alternation.

**Remaining pioneer work (Phases 3-5):**
- Phase 3: Offline mode — add `--file` to agent-census, census fixture, API liveness check
- Phase 4: Vault integration — pattern export for proxy-level feed scanning (spec: `pioneer/docs/specs/2026-04-04-vault-integration-design.md`)
- Phase 5: Pattern harmonization — compare pioneer's 25 vs forge's 87 patterns, document overlap

**Exit criteria for Phase A completion:** `make test` passes (DONE), offline mode works, pattern export format defined.

---

### Phase B: Forge → Vault Skill Installation

**Why:** The most important integration gap. Forge produces certified skill bundles. Vault needs to accept them. Currently manual (copy files).

**Engineering work:**
- Build `install-skill.sh` in vault (validates clearance report, copies to workspace)
- Define host-side transfer directory (`~/.lobster-trapp/skill-transfers/`)
- Wire into GUI as "Install to Vault" button on forge dashboard
- Shell level policy: Hard = blocked, Split = user confirms, Soft = allowed with notification

**Exit criteria:** A skill exported from forge can be installed into vault via one GUI click, with certificate validation.

---

### Phase C: Pioneer → Vault Feed Scanning

**Why:** When Moltbook domains enter the Soft Shell allowlist, social content must be scanned for injection attacks before the agent sees it.

**Engineering work:**
- Import pioneer's 25 injection patterns into `vault-proxy.py`
- Pattern matching on Moltbook API response bodies at the proxy level
- Log and optionally block responses containing injection signatures

**Not blocking anything now** — Moltbook domains are not in the allowlist.

---

### Phase D: Setup Wizard End-to-End

**Why:** The GUI's setup wizard exists but isn't connected to vault's compose commands. Non-technical users still need a terminal.

**Engineering work:**
- Wire setup wizard steps to real Podman/Docker commands
- Guide API key entry, bot creation, pairing
- Gear selector triggers shell switching via component.yml commands

**Exit criteria:** Non-technical user can set up the full stack through the GUI — no terminal.

---

### Phase E: Landing Page + Release Prep

**Why:** People need to find and understand the product before downloading.

**Engineering work:**
- Static site at lobster-trapp.com (GitHub Pages)
- Pre-built binaries via GitHub releases (CI already configured, just needs git tag)
- Final README polish across all 4 repos
- All repos made public

**Exit criteria:** A stranger landing on lobster-trapp.com understands the product and can download it within 30 seconds.

---

## Dependency Graph

```
DONE ──→ Phase A (Pioneer completion)
              |
         Phase B (Skill installation — forge→vault)
              |
         Phase C (Feed scanning — pioneer→vault)
              |
         Phase D (GUI setup wizard end-to-end)
              |
         Phase E (Landing page + release)
```

Phases B and C can run in parallel once Pioneer is complete.

---

## The One-Sentence Pitch Per Audience

**Non-technical user:** "Message your own AI assistant from Telegram — it helps with your tasks and can't touch your private stuff."

**Developer:** "Container-isolated OpenClaw with proxy-gated networking, six-layer defense-in-depth, and a GUI for non-technical users."

**Security researcher:** "Defense-in-depth sandbox for OpenClaw: custom seccomp, proxy key injection, tool policy verified via source code analysis, 24-point live verification."

**GitHub star-hunter:** "The only security harness for the most dangerous open-source AI agent. We proved the containment works. You can too."
