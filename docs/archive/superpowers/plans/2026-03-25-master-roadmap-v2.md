# Lobster-TrApp Master Roadmap v2 — DEPRECATED

> **This document is superseded by [`2026-04-04-master-roadmap-v3.md`](2026-04-04-master-roadmap-v3.md).** Phases 3-6 listed below as future work were completed between 2026-03-27 and 2026-04-03. The v3 roadmap reflects the actual state.

**Updated:** 2026-03-25
**Previous:** `2026-03-23-openclaw-vault-master-roadmap.md` (vault-only, superseded)

**Product:** A desktop app that lets anyone safely run OpenClaw on their personal computer, controlled from their phone, without risking their digital life.

**Repos:** openclaw-vault (containment) + clawhub-forge (skill security) + moltbook-pioneer (ecosystem tools) + lobster-trapp (GUI + landing page)

**Domain:** lobster-trapp.com

---

## What's Done

| Phase | Status | What We Proved |
|-------|--------|---------------|
| Phase 0: Bug fixes | **DONE** | Clean foundation |
| Phase 1: OpenClaw compatibility | **DONE** | Container works, 23/23 security checks (expanded from original 15), source code analyzed |
| Phase 2: Gear 1 (Manual) | **DONE** | Agent responds via Telegram, tool policy verified, hallucination documented |

---

## What's Next (Priority Order)

### Phase 3: Make NewLobsterTrappBot Useful — Gear 2 (Semi-Auto)

**Why:** Gear 1 is a chatbot. Gear 2 is an assistant. Without Gear 2, there's no product worth downloading.

**What Gear 2 enables (from the user's phone via Telegram):**
- "Remind me to call the dentist tomorrow at 9am" → cron
- "Summarize this document" → file read in workspace
- "What's the latest news about X?" → sandboxed web browsing
- "Save these notes for me" → file write in workspace
- "Draft a reply to this email" → text generation + messaging

**Engineering work:**
- Credential persistence (Telegram pairing survives restart)
- Selective tool enabling (per-capability allow lists)
- File workspace (tmpfs with user-initiated file transfer)
- Web browsing (sandboxed Chromium through proxy)
- Cron/scheduling (persistent container mode)
- Per-gear compose templates and config profiles
- Gear-switching mechanism (`switch-gear.sh semi`)
- Gear 2 verification tests (prove restricted access holds)
- Solve LLM hallucination (better system prompt, stronger model option)

**Exit criteria:** User can schedule a reminder, read a file, and browse a website through Telegram — all logged by the proxy, all within allowed boundaries, all verifiable.

---

### Phase 4: Monitoring Dashboard

**Why:** Before Gear 3 (broad access), users must be able to see what NewLobsterTrappBot is doing. The GUI must render activity in plain language, not raw JSON logs.

**Engineering work:**
- Implement `monitoring/network-log-parser.py` (parse proxy JSONL → human-readable)
- Implement `monitoring/session-report.sh` (what did NewLobsterTrappBot do this session?)
- Implement `monitoring/activity-feed.sh` (live feed for the GUI)
- Update `component.yml` with monitoring commands
- Verify rendering in Lobster-TrApp GUI

**Exit criteria:** A non-technical user can open the GUI and see, in plain English, every action NewLobsterTrappBot took, every website it contacted, and every request that was blocked.

---

### Phase 5: Skill Security — clawhub-forge Integration

**Why:** Gear 2/3 may allow skill installation. Before that happens, downloaded skills must pass through the clawhub-forge scanner. 11.9% of ClawHub skills were malware.

**Engineering work:**
- Verify clawhub-forge's 87-pattern scanner works against real skills
- Integrate scanner into vault's skill-loading pathway
- Test with known-malicious skill samples (ClawHavoc dataset)
- Add skill scanning commands to vault's `component.yml`
- Document: how skills are scanned, what patterns are detected, how to review results

**Exit criteria:** A skill downloaded from ClawHub is automatically scanned before it can execute. Malicious patterns are flagged with MITRE ATT&CK references. User sees a clear "safe" or "blocked" verdict in the GUI.

---

### Phase 6: Gear 3 (Full-Auto) — Broad Autonomy

**Why:** The power mode. NewLobsterTrappBot can operate broadly — shell commands, file system, web, messaging — while the driver seat (root, SSH keys, passwords) stays permanently locked.

**Engineering work:**
- Broad host mounts (user home minus protected resources)
- Exec allowlist with curated safeBins
- Protected resources test suite (attempt every driver-seat resource)
- Exfiltration threshold per gear
- Agent self-modification prevention
- Full end-to-end testing

**Exit criteria:** NewLobsterTrappBot operates autonomously. Protected resources verified inaccessible at maximum autonomy. User can downshift to Gear 1 at any time.

---

### Phase 7: Lobster-TrApp GUI — Setup Wizard + Gear Controls

**Why:** The current setup requires 8 terminal steps. A non-technical user cannot do this. The GUI must handle everything: Podman detection, container building, API key entry, Telegram bot setup, gear switching, and monitoring.

**Engineering work:**
- Setup wizard: detect Podman, guide API key entry, guide BotFather bot creation
- Gear selector in GUI (Manual / Semi-Auto / Full-Auto toggle)
- Per-capability checkboxes for Gear 2 (messaging, files, web, scheduling)
- Live monitoring dashboard (activity feed, proxy logs, security status)
- Kill switch button (always visible, works in any gear)
- Connect gear-switching to vault's `component.yml` commands

**Exit criteria:** A non-technical user can download Lobster-TrApp, click through a setup wizard, and have a working NewLobsterTrappBot on Telegram within 10 minutes — no terminal commands needed.

---

### Phase 8: Landing Page — lobster-trapp.com

**Why:** People need to find the product and understand it before downloading.

**Content:**
- Hero: "Your own AI assistant, safe by design"
- What it does (3 bullet points, non-technical language)
- How it works (security diagram, simplified)
- Download button (links to GitHub releases)
- Comparison table (vs raw OpenClaw, vs Claude.ai, vs ChatGPT)
- Trust section: open source, all traffic logged, you're always in control

**Tech:** Static HTML/CSS/JS, hosted on GitHub Pages or Vercel, connected to lobster-trapp.com domain.

**Exit criteria:** A stranger landing on lobster-trapp.com understands what the product does and can download it within 30 seconds.

---

### Phase 9: Moltbook Exploration — moltbook-pioneer

**Why:** The agent social network. Lowest priority because it's not essential for the core product, but it completes the trifecta.

**Engineering work:**
- Verify moltbook-pioneer tools work (feed scanner, agent census, identity checklist)
- Fix known bugs (safe_patterns, chmod, eval in curl)
- Write automated tests
- Integrate with vault (Moltbook API domains in Gear 2/3 allowlist)

**Exit criteria:** User can safely explore Moltbook, scan agent feeds for injection attacks, and register an agent identity — all through the GUI.

---

### Phase 10: Release

**Checklist:**
- [ ] All four repos have clean README.md matching actual functionality
- [ ] All security claims verified with test evidence
- [ ] Setup wizard works end-to-end for non-technical users
- [ ] Landing page live at lobster-trapp.com
- [ ] GitHub releases with pre-built binaries (Linux, macOS, Windows)
- [ ] All repos made public
- [ ] Post to: Hacker News, Reddit r/selfhosted, OpenClaw Discord

---

## Dependency Graph

```
DONE ──→ Phase 3 (Gear 2 — make NewLobsterTrappBot useful)
              |
         Phase 4 (Monitoring dashboard)
              |
         Phase 5 (Skill scanning — clawhub-forge)
              |
         Phase 6 (Gear 3 — full autonomy)
              |
         ┌────┴────┐
    Phase 7      Phase 8
  (GUI wizard)  (Landing page)    ← can be done in parallel
         └────┬────┘
              |
         Phase 9 (Moltbook)
              |
         Phase 10 (Release)
```

---

## The One-Sentence Pitch Per Audience

**Non-technical user:** "Message your own AI assistant from Telegram — it helps with your tasks and can't touch your private stuff."

**Developer:** "Container-isolated OpenClaw with proxy-gated networking, six-layer defense-in-depth, and a GUI for non-technical users."

**Security researcher:** "Defense-in-depth sandbox for OpenClaw: custom seccomp, proxy key injection, tool policy verified via source code analysis, 23-point live verification."

**GitHub star-hunter:** "The only security harness for the most dangerous open-source AI agent. We proved the containment works. You can too."
