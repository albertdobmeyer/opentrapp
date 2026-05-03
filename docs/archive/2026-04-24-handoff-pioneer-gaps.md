# Handoff: Moltbook-Pioneer Gap Analysis — UX and Integration

**Date:** 2026-04-06
**From:** Phase 8 certification session
**To:** Next instance picking up gap closure work
**Context:** All three modules (vault, forge, pioneer) are certified complete at the tooling level. This document identifies the gaps between "tooling exists" and "non-technical users can safely use it."

---

## Current State

All test suites pass:
- Orchestrator validation: 39/39
- Cross-module integration: 28/28
- Pioneer tests: 30/30
- Vault security tests: 13/13 (last verified Phase 7)
- Vault 24-point verify: 24/24 (last verified Phase 7)

Pioneer has three working tools (feed-scanner, agent-census, identity-checklist), 25 validated injection patterns, and excellent documentation. But everything is CLI-only.

**Moltbook API status:** DOWN as of 2026-04-06. All tools have offline modes.

---

## The Architecture (How Things Connect)

```
human <-> Telegram <-> OpenClaw agent <-> Moltbook
  you      phone        inside vault     agent social network
```

- **Telegram** is how humans talk TO their bot (messaging interface)
- **Moltbook** is where agents talk TO EACH OTHER (social network)
- **Vault** isolates the agent runtime (container + proxy)
- **Pioneer** provides defensive tools for Moltbook interaction
- **Forge** gates skills before they enter the vault
- **Lobster-TrApp GUI** is supposed to make all of this accessible to non-technical users

There is NO X/Twitter integration. Moltbook is a separate agent-only social network.

---

## Identified Gaps (Priority Order)

### Gap 1: Pioneer is CLI-Only

**Problem:** All three pioneer tools are bash scripts requiring terminal comfort. Non-technical users can't use them.

**What exists:**
- `component.yml` with 6 GUI commands (feed-scan, feed-scan-agent, agent-census, census-trend, identity-check, setup)
- Lobster-TrApp GUI can discover and render these commands as buttons
- Tools produce ANSI output with report/table/checklist display formats

**What's missing:**
- No guided workflow in the GUI for "I want to safely observe Moltbook"
- No setup wizard specific to pioneer (just a `cp .env.example .env` command)
- No visual presentation of scan results (just raw terminal output)
- Users must manually edit `.env` and `feed-allowlist.yml` via text editor

**Suggested approach:** Build a pioneer-specific onboarding flow in the GUI that walks through the three engagement levels (Observer → Researcher → Participant), configuring the right settings at each level.

### Gap 2: Vault <-> Pioneer Not Wired at Runtime

**Problem:** Pioneer's feed scanner runs on the host. When the agent reads Moltbook from inside the vault, pioneer's scanner is NOT consulted. The agent processes raw feed content without injection filtering.

**What exists:**
- Pioneer's `make export-patterns` produces `data/patterns-export.yml` (25 patterns, SHA-256 integrity hash)
- Vault's `proxy/vault-proxy.py` handles all agent traffic
- Design spec: `components/moltbook-pioneer/docs/specs/2026-04-04-vault-integration-design.md`
- Vault-side spec: `components/openclaw-vault/docs/specs/2026-03-30-feed-scanning-deferred.md`

**What's missing:**
- Vault-proxy loading pioneer patterns at startup (from `MOLTBOOK_PATTERNS` env var)
- Response inspection for Moltbook domains in vault-proxy.py
- Blocking logic: CRITICAL → block response, HIGH/MEDIUM → log, pass through
- Moltbook domains not in vault's proxy allowlist (intentionally — adding them is a policy decision)
- Check #25 in verify.sh for feed scan status

**Blocking dependency:** Moltbook domains must be added to `proxy/allowlist.txt` before any of this is testable. The API is currently down, which makes this untestable regardless.

**Design is complete.** Implementation is straightforward once the API is accessible and the allowlist policy decision is made.

### Gap 3: No Push-Button Safe Moltbook Entry

**Problem:** A non-technical user cannot go from "I want to observe Moltbook" to actually doing it without terminal skills.

**What's needed:**
- GUI setup wizard for pioneer (configure .env, choose engagement level)
- Visual scan results (instead of raw ANSI terminal output)
- Integrated allowlist editor (add/remove trusted agents via GUI)
- One-click "Scan feed now" with visual threat indicators
- Dashboard showing platform stats (from census data)

**Where this lives:** Lobster-TrApp GUI (`app/src/` React frontend). The manifest contract already declares the commands — the gap is in how the GUI presents them.

### Gap 4: Hard Shell Is Just a Chatbot

**Problem:** The default security level (Hard Shell) only allows Telegram conversation. Users need Split or Soft Shell to see value, but switching requires `make split-shell` terminal commands.

**What's needed:**
- GUI control for shell switching (buttons in vault dashboard)
- Clear explanation of what each shell level enables
- component.yml already has `hard-shell`, `split-shell`, `soft-shell` commands

**This is a GUI integration task**, not a vault task. The shell switching works perfectly via CLI.

### Gap 5: Three Engagement Levels Not Enforced

**Problem:** Pioneer documents three engagement levels (Observer, Researcher, Participant) with different rate limits, allowlist policies, and automation rules. But nothing enforces the level — it's just documentation.

**What exists:**
- `docs/safe-participation-guide.md` — 224 lines, excellent
- `docs/threat-landscape.md` — 205 lines, real incident analysis
- `config/feed-allowlist.yml` — structure for trusted agents
- Identity checklist validates config before registration

**What's missing:**
- No config preset per engagement level (like vault's shell presets)
- `safe_patterns` key in `feed-allowlist.yml` is declared but silently ignored by the scanner
- No enforcement of rate limits (documented but not in code)
- No "Level 2 mode" vs "Level 3 mode" switch

**Suggested approach:** Create engagement level presets (like vault's hard/split/soft shell system) that configure rate limits, allowlist strictness, and automation rules per level.

---

## Key Files for Implementation

| Purpose | Path |
|---------|------|
| Pioneer tools | `components/moltbook-pioneer/tools/{feed-scanner,agent-census,identity-checklist}.sh` |
| Injection patterns | `components/moltbook-pioneer/config/injection-patterns.yml` |
| Feed allowlist | `components/moltbook-pioneer/config/feed-allowlist.yml` |
| Pattern export | `components/moltbook-pioneer/scripts/export-patterns.py` |
| Pioneer manifest | `components/moltbook-pioneer/component.yml` |
| Vault proxy | `components/openclaw-vault/proxy/vault-proxy.py` |
| Vault allowlist | `components/openclaw-vault/proxy/allowlist.txt` |
| Feed scan design | `components/moltbook-pioneer/docs/specs/2026-04-04-vault-integration-design.md` |
| Feed scan vault spec | `components/openclaw-vault/docs/specs/2026-03-30-feed-scanning-deferred.md` |
| Safe participation | `components/moltbook-pioneer/docs/safe-participation-guide.md` |
| Threat landscape | `components/moltbook-pioneer/docs/threat-landscape.md` |
| Platform anatomy | `components/moltbook-pioneer/docs/platform-anatomy.md` |
| Trifecta (cross-module) | `docs/trifecta.md` |
| GUI frontend | `app/src/` (React 18) |
| GUI backend | `app/src-tauri/` (Rust) |
| Manifest schema | `schemas/component.schema.json` |

## Documentation to Read First

1. **This document** — you're here
2. **`components/moltbook-pioneer/CLAUDE.md`** — pioneer operating instructions
3. **`components/moltbook-pioneer/docs/safe-participation-guide.md`** — the three engagement levels
4. **`components/moltbook-pioneer/docs/threat-landscape.md`** — what the threats actually are
5. **`docs/trifecta.md`** — how vault + forge + pioneer work together
6. **`CLAUDE.md`** (lobster-trapp root) — the hard constraint: GUI must be generic, no component-specific logic

## Test Suites

```bash
cd ~/Repositories/lobster-trapp
bash tests/orchestrator-check.sh    # 39 manifest/structure checks
bash tests/integration-test.sh      # 28 cross-module data contract checks

cd components/moltbook-pioneer
make test                           # 30 tool tests
make verify                         # workbench health check
make check-api                      # Moltbook API liveness
```

---

*This handoff focuses on UX and integration gaps. All security tooling is complete and tested. The foundation is solid — the work ahead is making it accessible.*
