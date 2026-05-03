# UX Redesign Spec — Feature Inventory & User Story Mapping

## Context

The Lobster-TrApp GUI currently renders component.yml manifests generically — every command, probe, and config declared in a manifest becomes a button on the dashboard. This is architecturally elegant but creates a terrible UX: a non-technical user sees 14 commands, 4 workflows, 3 configs, and developer jargon like "Nuclear Kill" and "seccomp profiles."

This spec inventories everything we have, categorizes it by who actually needs it, writes user stories from the non-technical user's perspective, and maps capabilities to stories — revealing what to show, what to hide, and what's missing.

---

## Phase 1: Complete Feature Inventory

### OpenClaw Vault (14 commands, 4 workflows, 3 configs)

**What a non-technical user actually needs:**

| Capability | User Need | Current UI |
|---|---|---|
| Start the agent | "Turn on my assistant" | Start command (buried in lifecycle group) |
| Stop the agent | "Turn off my assistant" | Soft Stop, Hard Kill, Nuclear Kill (3 confusing options) |
| Check if it's safe | "Is everything OK?" | Verify command + Security Audit workflow |
| Switch security level | "Give it more/less freedom" | Hard Shell, Split Shell buttons + 2 workflows |
| Pair Telegram | "Connect my phone" | Approve Pairing command (recently added) |
| Enter API keys | "Connect my AI account" | .env config editor |
| See what it's doing | "What is my assistant up to?" | Container Logs, Proxy Logs (raw streams) |
| Install a skill | "Give it a new ability" | Install Skill command |

**What only developers need (should be hidden or collapsed):**

| Capability | Why it's developer-only |
|---|---|
| Setup | One-time, handled by wizard |
| Hard Kill | Soft Stop is enough for users |
| Nuclear Kill | Dangerous, requires understanding of containers |
| Tool Status | Technical tool-policy details |
| Proxy Logs | Raw mitmproxy output |
| openclaw-hardening.json5 config | Never edited manually — controlled by shell switching |

### ClawHub Forge (14 commands, 3 workflows, 4 health probes)

**What a non-technical user actually needs:**

| Capability | User Need | Current UI |
|---|---|---|
| Check a skill for malware | "Is this skill safe?" | Vet Skill workflow |
| Download a skill safely | "Get this skill for my assistant" | Safe Download workflow |
| See skill health overview | "Are my skills OK?" | Health badges |

**What only developers need:**

| Capability | Why |
|---|---|
| New Skill, Create Skill (AI) | Skill creation is for developers |
| Lint, Scan, Verify, Certify, Export | Individual pipeline steps — workflows cover this |
| Publish | Publishing to ClawHub registry |
| Stats, Explore, Report | Registry analytics |
| Clean, Setup | Maintenance |
| Lint All, Scan All, Verify All | Batch operations |

### Moltbook Pioneer (10 commands, 3 workflows, 1 health probe)

**What a non-technical user actually needs:**

| Capability | User Need | Current UI |
|---|---|---|
| Check if feeds are safe | "Are there threats on the network?" | Scan Recent Feed workflow |
| See current safety status | "Am I protected?" | Safety Check workflow |

**What only developers/researchers need:**

| Capability | Why |
|---|---|
| Scan Agent, Census, Census Trend | Research tools |
| Observer/Researcher/Participant modes | Engagement level complexity |
| Export Patterns | Integration plumbing |
| Identity Checklist | Pre-registration check |
| All config editing | API keys, patterns, allowlists |

**Current status:** Moltbook API is down. Most features are non-functional. This entire module is informational only.

### Orchestrator Workflows (4 cross-component)

| Workflow | User Need | Status |
|---|---|---|
| Install Skill | "Download + scan + install in one click" | Working |
| First Run Setup | "Set everything up for me" | Working |
| Full Audit | "Check everything is safe" | Working |
| Enable Feeds | "Turn on network monitoring" | Deferred (API down) |

---

## Phase 2: User Stories

From the perspective of a non-technical user who just installed the app:

### Setup & First Use
1. "I downloaded the app. **Walk me through getting started.**"
2. "I need to **enter my API key**. Where do I get one?"
3. "I need to **connect my Telegram** so I can talk to my assistant."
4. "The setup is done. **What do I do now?**"

### Daily Use
5. "**Is my assistant running?** How do I tell?"
6. "I want to **start my assistant** for the day."
7. "I want to **stop my assistant** for the night."
8. "I want to **talk to my assistant** — how?"
9. "My assistant needs a **new skill** — how do I add one?"

### Security & Trust
10. "**Is everything safe?** How do I check?"
11. "I want to **give my assistant more freedom** to do things automatically."
12. "I want to **lock my assistant down** so it can only chat."
13. "**What is my assistant allowed to do** right now?"
14. "I'm worried — **can my assistant access my personal files?**"

### Troubleshooting
15. "**Something went wrong.** What happened?"
16. "My assistant **isn't responding** on Telegram."
17. "I want to **start over** from scratch."

---

## Phase 3: Story → Capability Mapping

| Story | Capabilities that serve it | Currently in UI? | UX Quality |
|---|---|---|---|
| 1. Get started | First Run Setup workflow, wizard | Yes (wizard) | Good |
| 2. Enter API key | .env config editor, ConfigStep form | Yes (new form) | Good |
| 3. Connect Telegram | Approve Pairing command | Yes (new) | Poor — buried in commands |
| 4. What now? | Dashboard onboarding banner | Yes (new) | OK — dismissable |
| 5. Is it running? | Container health probe, status badge | Yes | Good |
| 6. Start assistant | Start command, Secure Start workflow | Yes | OK — two options confuse |
| 7. Stop assistant | Soft Stop | Yes | Poor — 3 stop options (soft/hard/nuclear) |
| 8. Talk to assistant | (Telegram — external) | No guidance in app | Missing |
| 9. Add a skill | Install Skill workflow | Yes | Poor — requires understanding forge |
| 10. Is it safe? | Security Audit workflow, Verify | Yes | Good — clear checklist |
| 11. More freedom | Split Shell → Soft Shell switch | Yes | Poor — jargon, confusing flow |
| 12. Lock down | Hard Shell switch | Yes | Poor — "Hard Shell" means nothing |
| 13. What can it do? | Tool Status command | Yes | Poor — raw technical output |
| 14. Personal files safe? | (Implicit in architecture) | No | Missing — should be explained |
| 15. Something wrong | Error messages, logs | Partial | Poor — raw exit codes |
| 16. Not responding | (No diagnostic flow) | No | Missing |
| 17. Start over | Nuclear Kill + re-setup | Yes | Dangerous — too easy to click |

### Gaps Identified

**Missing entirely (no capability exists):**
- Story 8: No in-app guidance on how to talk to the assistant via Telegram
- Story 14: No explanation that personal files are protected
- Story 16: No "my assistant isn't responding" diagnostic flow

**Capability exists but UX is terrible:**
- Story 3: Pairing is buried in commands list
- Story 7: Three stop options where one would do
- Story 11/12: Shell level switching uses jargon, confusing prerequisites
- Story 13: Tool status is raw developer output
- Story 17: Nuclear Kill is too accessible and too dangerous

---

## Phase 4: UX Redesign Principles

Based on the mapping, here's what the redesign should accomplish:

### Principle 1: Two-tier interface
- **User tier**: Show only capabilities that serve user stories (stories 1-17)
- **Advanced tier**: Collapse developer tools behind an "Advanced" toggle
- The manifest-driven generic renderer stays, but presentation adds a human layer

### Principle 2: The three questions
Every component dashboard should immediately answer:
1. **Is it running?** (big clear status indicator)
2. **What can it do?** (current shell level in plain language)
3. **What should I do next?** (contextual action — start, pair, audit)

### Principle 3: Defer shell switching for v0.1.0
- Ship with **Soft Shell as default** — gives users the most useful experience
- Don't expose Hard/Split/Soft switcher to non-technical users (it's confusing and requires 30-70s container restart)
- Shell level switching remains available in the Advanced section for power users
- Post-v0.1.0: investigate hot-reload and build a proper segmented control

### Principle 4: One stop button
Replace Soft Stop / Hard Kill / Nuclear Kill with:
- **Stop** (default — graceful stop)
- "Factory Reset" (hidden in Settings, requires confirmation)

### Principle 5: Contextual actions
Instead of showing all 14 commands, show the RIGHT action based on state:
- Agent stopped → show "Start" prominently
- Agent running, not paired → show "Connect Telegram" prominently
- Agent running, paired → show "talk to your assistant on Telegram"
- Security check failed → show "Run Security Audit" prominently

---

## Phase 5: Proposed Component Dashboard Layouts

### Vault Dashboard (user tier)

```
┌─────────────────────────────────────────────────┐
│ Your AI Assistant                     [Running] │
│ Running safely inside a secure sandbox          │
│                                                 │
│ ┌─────────────────────────────────────────────┐ │
│ │ Your assistant can search the web, manage   │ │
│ │ files, and schedule tasks — all within safe │ │
│ │ boundaries. It can't access your personal   │ │
│ │ files or passwords.                         │ │
│ └─────────────────────────────────────────────┘ │
│                                                 │
│ Quick Actions                                   │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│ │ Security │ │ Connect  │ │  Stop    │        │
│ │  Audit   │ │ Telegram │ │ Assistant│        │
│ └──────────┘ └──────────┘ └──────────┘        │
│                                                 │
│ 💬 Talk to your assistant                       │
│ Open Telegram and message @YourBotName          │
│                                                 │
│ ▸ Advanced (14 commands, 3 configs)             │
└─────────────────────────────────────────────────┘
```

### Forge Dashboard (user tier)

```
┌─────────────────────────────────────────────────┐
│ Skill Scanner                          [Ready]  │
│ Checks skills for malware before installation   │
│                                                 │
│ Health: 25 skills │ All clean │ 168 tests pass  │
│                                                 │
│ ┌──────────────┐ ┌──────────────┐              │
│ │ Check Skill  │ │ Install Skill│              │
│ │ for Malware  │ │  Safely      │              │
│ └──────────────┘ └──────────────┘              │
│                                                 │
│ ▸ Advanced (14 commands, pipeline tools)         │
└─────────────────────────────────────────────────┘
```

### Pioneer Dashboard (user tier)

```
┌─────────────────────────────────────────────────┐
│ Network Monitor                        [Ready]  │
│ Scans the agent social network for threats      │
│                                                 │
│ ⚠ Moltbook API is currently unavailable.        │
│   Network monitoring will activate when the     │
│   service comes back online.                    │
│                                                 │
│ ┌──────────────┐                               │
│ │ Safety Check │                               │
│ └──────────────┘                               │
│                                                 │
│ ▸ Advanced (10 commands, engagement levels)      │
└─────────────────────────────────────────────────┘
```

---

## Implementation Approach

The key insight: **we don't need to change the manifest-driven architecture.** The generic renderer stays. We add a presentation layer ON TOP that:

1. Groups commands into "user" and "advanced" tiers using a new `tier` field in component.yml (or hardcoded in the frontend for now)
2. Renders the shell level switcher as a segmented control instead of separate buttons
3. Hides destructive commands behind Advanced toggle
4. Shows contextual guidance based on component state
5. Renames shell levels in the UI (Hard→Chat Only, Split→Supervised, Soft→Autonomous)

### Step 0: Save this spec
- Write the full inventory + UX mapping to `docs/specs/2026-04-18-ux-redesign.md`

### Files to modify

**Frontend (presentation layer):**
- `app/src/pages/ComponentDetail.tsx` — add user/advanced tier layout, contextual guidance
- `app/src/components/WorkflowPanel.tsx` — no change (already good)
- `app/src/components/CommandPanel.tsx` — add Advanced collapse toggle
- New: `app/src/components/QuickActions.tsx` — contextual action buttons based on state

**Manifests (command tier tagging):**
- `components/openclaw-vault/component.yml` — add `tier: user|advanced` to commands
- `components/clawhub-forge/component.yml` — add `tier: user|advanced` to commands
- `components/moltbook-pioneer/component.yml` — add `tier: user|advanced` to commands
- `schemas/component.schema.json` — add optional `tier` field to command schema
- `app/src-tauri/src/orchestrator/manifest.rs` — add `tier` field to Command struct
- `app/src/lib/types.ts` — add `tier` field to Command type

**Deferred (post-v0.1.0):**
- `app/src/components/ShellLevelSwitcher.tsx` — segmented control with hot-reload
- Shell level name mapping in frontend (Hard→Chat Only, Split→Supervised, Soft→Autonomous)

---

## Verification

After implementation:
1. Launch app with `npm run tauri dev`
2. Navigate to each component dashboard
3. Verify user-tier shows only relevant actions
4. Verify Advanced toggle reveals all commands
5. Verify shell level switcher works (Chat Only / Supervised / Autonomous)
6. Verify contextual guidance appears based on state
7. Run `npm test` — all 147 tests pass
8. Run `bash tests/orchestrator-check.sh` — all 42 checks pass
