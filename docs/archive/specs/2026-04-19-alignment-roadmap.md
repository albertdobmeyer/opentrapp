# Alignment Roadmap — From Vision to v0.1.0 Release

**Date:** 2026-04-19
**Northstar:** "Your own personal AI assistant, safe on any computer."
**Prerequisite:** `docs/specs/2026-04-19-product-identity-spec.md`

---

## The Two Audiences, Two Layers

| Layer | Repo | Audience | Voice |
|---|---|---|---|
| **Frontend** | lobster-trapp (parent) | Non-technical end users | "Your assistant is running safely" |
| **Backend** | openclaw-vault, clawhub-forge, moltbook-pioneer | Developers, contributors, security researchers | "24-point verification, seccomp, proxy-gated networking" |

The parent repo is ALL the non-technical user ever sees. The submodules are invisible to them — they're the plumbing. Documentation must respect this boundary.

---

## What Needs to Align (Ordered by Impact)

### 1. Frontend Presentation Reframe (the big one)

Implement the product identity spec in React. Same manifest engine, new presentation.

| Current | Becomes |
|---|---|
| Sidebar: "ClawHub Forge" | Sidebar: "Skills" |
| Sidebar: "Moltbook Pioneer" | Sidebar: "Network" |
| Sidebar: "OpenClaw Vault" | Sidebar: "My Assistant" |
| Dashboard: 3 component cards | Dashboard: assistant status + skill count + network status |
| Component Detail: developer commands | Assistant page: "Talk to your assistant" + quick actions |
| Wizard: "Set Up Components" | Wizard: "Setting up your assistant..." |
| Wizard: raw build output | Wizard: progress bar with friendly status messages |

**Files:** `app/src/components/Sidebar.tsx`, `app/src/pages/Dashboard.tsx`, `app/src/pages/ComponentDetail.tsx`, `app/src/pages/Setup.tsx`, wizard step components

**Effort:** ~4-6 hours (presentation only, no backend changes)

### 2. Landing Page Final Polish

The landing page is already simplified but needs to fully commit to the "assistant" framing.

| Current | Becomes |
|---|---|
| "Run AI agents with defense-in-depth isolation" (title, og:title) | "Your own AI assistant, safe on your computer" |
| Meta description: "defense-in-depth isolation" | "A personal AI assistant that runs on your computer..." |
| "Contain / Scan / Monitor" feature cards | "Your Assistant / Safe Skills / Network Protection" |

**Files:** `docs/index.html` (content + meta tags)

**Effort:** ~30 minutes

### 3. README.md — The GitHub Front Door

Current README is better than before but still leads with "4-container security perimeter." For a non-technical user landing on GitHub, it should lead with what the product DOES, not how it works.

**Structure:**
```
# Lobster-TrApp — Your Personal AI Assistant, Safe on Any Computer

[badges]

Get a personal AI assistant you control from your phone — without risking 
your files, accounts, or digital life. Lobster-TrApp installs OpenClaw 
safely on any computer with a simple GUI. No terminal needed.

## What You Get
- An AI assistant you talk to from Telegram
- That can search the web, manage files, schedule tasks
- Running locally on YOUR computer (not in the cloud)
- In an invisible security sandbox (it can't touch your personal stuff)

## Download
[platform links]

## How It Works (for the curious)
[brief technical overview — the current content, collapsed]

## For Developers
[contributing, building from source — collapsed]
```

**Files:** `README.md`

**Effort:** ~30 minutes

### 4. Parent Repo CLAUDE.md — AI Assistant Context

The CLAUDE.md is for AI coding assistants (Claude Code, Copilot). It should reflect the product identity so that AI assistants working on this codebase understand the vision.

**Key addition:** A "Product Identity" section at the top that states:
- This is a personal AI assistant app for non-technical users
- The submodules are invisible backend infrastructure
- The GUI must never expose developer concepts to end users
- The northstar: "Your own personal AI assistant, safe on any computer"

**Files:** `CLAUDE.md`

**Effort:** ~15 minutes

### 5. GLOSSARY.md — User Terms Mapping

Add a "User-Facing Terms" section that maps developer concepts to what appears in the UI:

| Developer Term | User Term |
|---|---|
| OpenClaw Vault | My Assistant |
| ClawHub Forge | Skills / Skill Store |
| Moltbook Pioneer | Agent Network |
| Hard Shell | Chat Only |
| Split Shell | Supervised |
| Soft Shell | (default, no name shown) |
| Component | (invisible) |
| Manifest | (invisible) |
| Container | Secure sandbox |
| Perimeter | (invisible) |
| Proxy | (invisible) |
| Workflow | Action |
| Command | (hidden or renamed) |

**Files:** `GLOSSARY.md`

**Effort:** ~15 minutes

### 6. Handoff.md — Point to the Northstar

Update the handoff to reference the product identity spec and mark the UX reframe as the current work.

**Files:** `docs/handoff.md`

**Effort:** ~10 minutes

---

## What Does NOT Need to Change

| Document | Why it stays |
|---|---|
| Submodule CLAUDE.md files | Developer-facing, correctly describe technical internals |
| Submodule README.md files | Developer-facing, correctly describe setup and usage |
| `docs/trifecta.md` | Architecture doc for developers/contributors |
| `docs/product-assessment.md` | Strategic analysis (already has the right vision at the end) |
| `docs/roadmap-v4-finalization.md` | Historical planning document |
| `schemas/component.schema.json` | Technical contract |
| `compose.yml` | Infrastructure definition |
| All Rust backend code | Architecture is solid |
| All test suites | Coverage is good |

---

## Release Sequence

### Phase A: Documentation Alignment (~1 hour)
1. Update README.md — assistant-first framing
2. Update CLAUDE.md — add product identity section
3. Update GLOSSARY.md — add user terms mapping
4. Update handoff.md — reference product identity spec
5. Commit + push

### Phase B: Frontend Reframe (~4-6 hours)
1. Sidebar: rename components to user terms
2. Dashboard: assistant-centric layout
3. Component detail pages: contextual guidance per component role
4. Wizard: friendly step labels + progress bar (replace raw build output)
5. Commit + push + verify in running app

### Phase C: Landing Page Go-Live (~30 min)
1. Final polish on index.html meta tags and content
2. Deploy index.html to Hetzner (replace coming-soon.html)
3. Verify lobster-trapp.com shows the real landing page

### Phase D: Release (~30 min)
1. Run full test suite (42 orchestration + 147 frontend + Rust tests)
2. Tag v0.1.0
3. Verify CI builds all platform binaries
4. Verify release artifacts appear on GitHub Releases
5. Verify updater manifest (latest.json) is generated
6. Announce

---

## After v0.1.0

- Claude Code CLI/MCP integration (Phase 7) — power users can manage via terminal
- Shell level switching UX (with hot-reload investigation)
- In-app help system
- Skill browsing from ClawHub registry
- Moltbook integration (when API returns)
- Usage analytics (opt-in, privacy-respecting)

---

## The Constraint That Makes This Work

**We don't rewrite anything.** The manifest-driven architecture stays. The 24-point security verification stays. The 4-container perimeter stays. The compose.yml stays. The Rust backend stays.

We add a **presentation layer** that translates developer concepts into user concepts. The engine is built. We're putting the body on the car.

The four-repo split works perfectly for this:
- **lobster-trapp** = the car body (what users see and touch)
- **openclaw-vault** = the engine (runtime containment)
- **clawhub-forge** = the fuel filter (supply chain defense)
- **moltbook-pioneer** = the radar (social threat detection)

The user drives the car. They never open the hood. But if they do — everything is there, well-documented, and proven.
