# Vision & Personas

**Prerequisite reading:** `00-HANDOFF.md`
**Purpose:** Establish the two personas and the product identity before touching code. Every design decision in every subsequent spec is answered by "which persona, and what does the product identity say?"

---

## Product Identity — The Invisible Security Wrapper

OpenTrApp is **not** an AI assistant. OpenTrApp is **not** a chat client. OpenTrApp is **not** an agent framework.

**OpenTrApp is the invisible security wrapper that makes it safe for a non-technical person to run OpenClaw on their personal computer.**

Compare:

| Product | What it does | User awareness |
|---------|--------------|----------------|
| Windows Defender | Invisible anti-malware | Low — checks in occasionally |
| 1Password (desktop) | Invisible credential vault | Low — unlocks automatically |
| Little Snitch | Network firewall | Low — alerts when needed |
| Tailscale | Secure private networking | Low — runs in tray |
| **OpenTrApp** | **Invisible security wrapper for OpenClaw** | **Low — user talks to OpenClaw via Telegram, not via us** |

The app's job is to:
1. **Get OpenClaw installed safely** (setup wizard, container perimeter, manifest-driven orchestration)
2. **Keep it running safely** (security monitoring, allowlist enforcement, skill scanning)
3. **Stay out of the way** (system tray, minimal check-ins, automation-first)

The user's relationship with OpenClaw happens on Telegram. The user's relationship with OpenTrApp is: "something that keeps me safe while I use my AI assistant."

---

## Karen — The Non-Technical End User

### Who she is

- 54 years old, retired teacher
- Uses Facebook, WhatsApp, email, and recently started using ChatGPT
- Bought a mid-range laptop from Best Buy last year; uses it for email, video calls with grandkids, and browsing
- Has a smartphone she uses heavily (Telegram, WhatsApp, Instagram)
- Believes in "computers should just work"
- Does NOT know what a container, API, terminal, or repository is
- Has heard "AI can be dangerous" and is worried about her files/passwords
- Found us via a YouTube video titled "Run ChatGPT-like AI safely on your computer"

### What she wants

1. **Install easily** — download the installer, double-click, done
2. **Get her AI assistant talking to her on Telegram quickly** — under 10 minutes total
3. **Be confident her stuff is safe** — her photos, passwords, work files
4. **Forget the app exists** once it's set up — it should just run

### What she doesn't want

- To read documentation
- To open a terminal
- To understand what a "container" is
- To make technical decisions
- To see error codes, file paths, or stack traces
- To feel stupid because the app uses words she doesn't know

### Her pain points today

Watching a first-time walkthrough of the current app, Karen would:

1. See "Set Up Components" and think "what's a component?"
2. See `components/opencli-container/.env` and think "I don't know what any of this means"
3. See security verification output with 24 checks and think "something has gone wrong, 24 is a big number"
4. See a stack trace and think "I broke my computer"
5. Give up.

### The fix

Every screen Karen sees must:
- Use words she already knows (Facebook-literate vocabulary)
- Show pictures and icons more than text
- Automate decisions she can't make
- Celebrate success
- Reassure on failure

---

## The Developer — The Tech-Savvy Power User

### Who they are

- 20s–40s, software engineer / security researcher / open-source contributor
- Uses Linux or a Mac, comfortable in a terminal
- Familiar with containers, API keys, security tools
- Found us on GitHub because a colleague shared the repo, or because they searched for "AI agent sandbox" or "openclaw security"
- Might fork one of the submodules for their own project
- Evaluates the security model critically

### What they want

1. **Deep visibility** — see every component's state, every command's output, every config's contents
2. **Customization** — edit the allowlist, swap shell levels, tune security profiles
3. **Trustworthiness** — verify that the security claims are real by inspecting the 24-point audit themselves
4. **Extensibility** — understand the manifest contract well enough to contribute a new component
5. **A terminal fallback** — if the GUI annoys them, they can use the submodules directly

### What they don't want

- Hand-holding
- Childish illustrations
- Walls of gentle reassurance
- Hidden information
- To be locked out of powerful features

### Their pain points with the current app

- **Developer tools are collapsed by default** — they have to click "Developer Tools" on every page
- **No logs view** — stream output is ephemeral
- **No manifest inspector** — they have to open `component.yml` in an editor
- **No allowlist editor** — they have to edit files manually
- **Security audit runs are buried** — they have to click through workflows to see them
- **Role-based labels strip information** — "My Assistant" hides whether it's actually opencli-container

### The fix

When they toggle to Advanced Mode:
- Information density increases (smaller type, tighter spacing)
- Full component/manifest/log/audit visibility
- All configs editable in-app (with schema validation)
- Technical labels return (opencli-container, openskill-forge, openagent-social)
- Keyboard-first navigation

---

## The North Star

> **Karen should forget OpenTrApp exists after week 1. A developer should prefer it over raw podman.**

If the app delights Karen, she will recommend it to her sister. If the app satisfies developers, they will contribute to it. Both audiences need to feel the app was built for them, specifically.

---

## Product Tensions and How to Resolve Them

Throughout implementation, you'll hit decisions where Karen's needs conflict with a developer's. **Resolve every such conflict in favor of the active mode.**

| Tension | Karen's need | Developer's need | Resolution |
|---------|--------------|------------------|------------|
| Component naming | Role-based ("My Assistant") | Canonical ("opencli-container") | Active mode decides: user mode → role-based, dev mode → canonical |
| Error details | Hidden by default | Visible by default | Active mode decides |
| Status text | Sentences ("Running safely") | State tokens ("running") | Active mode decides |
| Available commands | Only "user" tier | All tiers | Already in schema via `tier: user \| advanced` |
| Security audit output | "All checks passed" summary | 24-line detailed output | Active mode decides |
| Settings granularity | 5 toggles | 50 settings | Active mode decides |

**No compromise views.** Don't invent a "medium mode" or a "slightly technical label". Pick one audience per screen and serve it fully.

---

## Feature Inclusion Test

When considering whether to add a new feature, ask:

- **Does it serve Karen?** → Build it in user mode
- **Does it serve developers?** → Build it in dev mode
- **Does it serve neither?** → Don't build it
- **Does it serve both?** → Build it in both modes, with different presentations

Example: "Activity feed"
- Karen: wants to know her assistant is busy doing useful things → friendly timeline ("2 PM — Helped you plan your Tuesday")
- Developer: wants to audit what the agent actually did → raw event stream with timestamps, command IDs, exit codes
- Build: Yes, in both modes, with different views.

Example: "24-point security audit"
- Karen: doesn't care about the 24 points → show "Safe" badge
- Developer: cares deeply → show full audit report
- Build: Yes, in both modes, with different views.

Example: "Manifest inspector"
- Karen: doesn't know what a manifest is → don't show
- Developer: wants to see the component.yml structure → show tree view
- Build: Yes, only in dev mode.

---

## Vocabulary Rules

The design system (spec `02`) will codify this, but the principle:

**User mode vocabulary** is Facebook-user level. Hard words banned outright. Easy synonyms always.

**Developer mode vocabulary** is engineer-level. Precision wins. Canonical names used.

A word that belongs in one mode must not leak into the other. Example: "manifest" never appears in user mode; "My Assistant" never appears in dev mode.

---

## Measuring Success

For each persona, define observable outcomes:

### Karen success metrics
- Install → first Telegram message: **< 10 minutes** (currently ~15)
- Clicks in setup flow: **< 15** (currently ~25)
- User-visible jargon words: **0** (currently 3–5 per screen, see rubric scores)
- "Tip of the day" card engagement: **> 20%** (TBD, new feature)
- Support contact rate after 1 week: **< 5%** (TBD)

### Developer success metrics
- Time to find logs for a specific component: **< 10 seconds** (currently — not possible in-app)
- Number of files to edit to change allowlist: **0** via UI (currently 1, manually)
- Full security audit visibility: **in one screen** (currently buried in workflows)
- Discoverable keyboard shortcuts: **≥ 5** (currently 0)

---

## Next

Read `02-design-system.md` to see how the visual language encodes these personas.
