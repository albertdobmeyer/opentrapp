# OpenTrApp v0.2.0 — Release Notes

**Status:** Draft, pending tag
**Target audience:** prosumer (tech-savvy users who can install Tauri + Podman locally)
**Container baseline:** 4-container perimeter; Split Shell hardcoded

## TL;DR

v0.2.0 is the **first defensible release** of the safe-front-door for the OpenClaw ecosystem. It ships:

- A 4-container security perimeter (vault-agent, vault-proxy, vault-forge, vault-pioneer) that air-gaps the AI agent from your host
- A Telegram bridge: enter an API key, pair Telegram, talk to your assistant from your phone
- A 19-entry curated use-case gallery — the assistant's "what can I ask?" picture book
- A 4-day stress-test campaign with 116 tests and **0 confirmed exploits**
- Three operational fixes (F11, F13, F14) that quietly land the path to v0.3.0's Soft Shell

## What you can do (Split Shell, default)

The bot can:

- Answer questions from training data (cooking, advice, drafting, translating)
- Read and summarise files you place in its workspace
- Process images you send via Telegram
- Draft emails, poems, brainstorm lists
- Route sensitive questions (medical, financial, legal) to the right professional with concrete next steps
- Recognise injection attempts and tell you about them

The bot will *not*:

- Reach the internet (no fetch / search at Split Shell — that's Soft Shell, opt-in via CLI for v0.2.0; surfaced in GUI in v0.3.0)
- Execute destructive commands (rm, chmod, network tools all stripped from the image)
- Break out of its workspace (workspaceOnly invariant; symlink resolver enforced at tool layer)
- Send messages to anyone you didn't pair (Telegram dmPolicy: pairing)

## The use-case gallery

This release populates `app/src/content/use-cases.ts` with 19 curated prompts, each tagged by capability:

- **12 ✅ "ready" entries** — work at the default Split Shell, end-to-end
- **6 🌐 "needs_fetch" entries** — bot redirects to a URL at Split Shell; fully functional at Soft Shell (opt-in via CLI for v0.2.0)
- **1 📅 "needs_calendar" entry** — bot routes to your phone's reminder app; full integration arrives with `vault-calendar` in v0.3.0

The Discover page that consumes this data is scaffolded as a placeholder; full wiring is Phase E.2.6 work.

## Security posture

| Layer | Test count | Result |
|---|---|---|
| Container (`tests/e2e-telegram/direct_probing/probe.sh`) | 22 host-reachability checks | 22/22 PASS |
| Tool (workspaceOnly, symlink resolver, safeBins constructive-only) | 14 enforcement assertions | All hold |
| LLM (prompt injection, social engineering, encoded payloads, multilingual) | 34 attacks (15 yesterday + 19 today) | **0 confirmed exploits** |
| Operational (F11/F13/F14 mediation path) | 73 static + live tests | All green |

Verdicts:
- `tests/e2e-telegram/VERDICT-2026-04-23.md` — direct probing
- `tests/e2e-telegram/VERDICT-2026-04-24.md` — 15-attack LLM-layer campaign
- `tests/e2e-telegram/VERDICT-2026-04-25.md` — use cases + Soft Shell experiment + findings
- `tests/e2e-telegram/F12-VERDICT-2026-04-25.md` — F12 falsified
- `tests/e2e-telegram/SOFT-SHELL-STRESS-VERDICT-2026-04-25.md` — Tier 4 stress-test replay

## Findings status

The four-day campaign catalogued 14 findings. State at v0.2.0 ship:

| # | Severity (orig) | Description | Status |
|---|---|---|---|
| F1 | HIGH | Bot tokens leaking into proxy logs | **Fixed** in commit `0ac3e9e` (parent) + `4f5b560` (submodule) |
| F2 | LOW | `/proc/mounts` discloses host device | Accepted-LOW |
| F3 | (positive) | Direct probing baseline | Documented |
| F4 | POSITIVE | Indirect injection caught at LLM layer with user notification | Documented |
| F5 | POSITIVE | Symlink escape blocked at TOOL layer | Documented |
| F6 | LOW | File-existence confabulation | Monitor — did not recur in this release |
| F7 | LOW | Tool inventory disclosed to friendly questioning | Optional — recon, not breach |
| F8 | POSITIVE | Session-context meta-cognition | Documented; reproduced today |
| F9 | (process) | Curated gallery vs full gallery | Resolved by capability tags in use-cases.ts |
| F10 | LOW | chat.py shell parsing edge | Fixed — see commit `9964185` |
| F11 | HIGH (product) | Soft Shell config didn't deliver tools | **Fixed** today — see "Operational fixes" below |
| F12 | MEDIUM | Soft Shell defensive verbosity degraded | **Falsified** — prompt-phrasing-dependent, not shell-dependent. Downgraded to LOW |
| F13 | LOW | `tool-control.sh` hardcoded container name | **Fixed** today |
| F14 | MEDIUM | `tool-control.sh --apply` spawned rogue parallel container | **Fixed** today |

## Operational fixes (this release)

These don't change anything a Karen-tier user does, but they unblock v0.3.0:

- **F11 (HIGH)** — `tool-manifest.yml` gained `also_allow:` for the soft preset; `tool-control-core.py` now emits `tools.alsoAllow` for tools whose OpenClaw `profiles` array is empty (web_fetch, web_search, cron, canvas, message). Confirmed live: at Soft Shell the bot now reports access to web_search, web_fetch, canvas, message, process — none of which were previously visible.
- **F13 (LOW)** — `tool-control.sh` honours `OPENCLAW_CONTAINER` env var. Embedders (opentrapp uses `vault-agent`) can target the right container without forking.
- **F14 (MEDIUM)** — `tool-control.sh --apply --no-restart` writes config + allowlist and returns; the parent orchestrator owns lifecycle. No more rogue parallel container.

## What's deferred to v0.3.0

- **GUI shell switcher** — Soft Shell is verified-safe (Tier 4 stress test: 19/19 attacks blocked) but not yet exposed as a user-flippable mode in the desktop GUI. Available via CLI for advanced users:
  ```
  yes y | OPENCLAW_CONTAINER=vault-agent \
    bash components/openclaw-vault/scripts/tool-control.sh \
      --preset soft --apply --no-restart && podman restart vault-agent
  ```
- **`vault-calendar` sidecar** — first sidecar following the tool-mediation pattern (see `docs/specs/2026-04-25-tool-mediation-pattern.md`). 3–5 days of work; v0.3.0 candidate.
- **Discover page wiring** — the data is in place; the React page is still a placeholder. Phase E.2.6.
- **Tip-of-the-day on Home dashboard** — same data source; same phase.

## Known limitations

- The bot retains in-session context across restarts of `chat.py` but not across container restarts. Restart → fresh session.
- Web fetch is allowlisted to `raw.githubusercontent.com` + the LLM-API + Telegram base. Other domains require user-side allowlist edit (this is intentional — the proxy is the trust boundary).
- The submodule remote on a few dev machines points at the old `gitgoodordietrying/openclaw-vault` URL. GitHub redirects pushes; a one-time `git remote set-url origin git@github.com:albertdobmeyer/openclaw-vault.git` silences the nag.

## Upgrade path

There is none. v0.2.0 is a clean install: download, enter key, pair Telegram, you're running. No state from v0.1.0 is consumed.

## Acknowledgements

This release is the result of a 4-day stress-test campaign that produced 116 tests, 5 verdict documents, 2 architectural specs (tool-mediation pattern + voice-and-calendar perimeter extension), and confidence that the security claim ("safe AI agent on any computer") holds under adversarial conditions.
