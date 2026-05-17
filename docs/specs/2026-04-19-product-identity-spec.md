# Product Identity Spec — What OpenTrApp Actually Is

**Date:** 2026-04-19
**Author:** Albert + Claude Opus
**Status:** DESIGN — the foundational product reframe

---

## The Problem We Solve

OpenClaw is a powerful autonomous AI agent system. It's an always-on personal assistant you control from your phone via Telegram. It can browse the web, manage files, schedule tasks, send messages, and automate workflows. It runs locally on your computer — no cloud subscription, your data stays yours.

**But a non-technical user cannot safely use it.**

Why not:
1. **It's technically complex** — requires terminal, Docker, git, config files, compose commands
2. **It's dangerous** — the agent has full access to your system by default (SSH keys, passwords, files, network)
3. **The ecosystem is hostile** — 11.9% of published skills are malware, the social network has injection attacks, 21,639 instances are exposed without authentication
4. **There's no GUI** — everything is CLI-only, aimed at developers

The non-technical user wants the assistant. They're scared of the ecosystem. They can't set it up. They can't protect themselves.

## What OpenTrApp Is

**OpenTrApp is the safe front door to the OpenClaw ecosystem for non-technical users.**

It does two things:
1. **Makes OpenClaw accessible** — GUI wizard installs everything with clicks, no terminal needed
2. **Makes OpenClaw safe** — the agent runs in an invisible security sandbox, all skills are scanned for malware, all network traffic is monitored and filtered

The security is the ENABLER, not the PRODUCT. The product is: **your own personal AI assistant, controlled from your phone, that you can trust.**

## What the User Sees vs. What We Do

| What the user sees | What we do in the background |
|---|---|
| "Install my assistant" | Build a 4-container security perimeter with rootless Podman |
| "My assistant is running" | 24-point security verification, read-only filesystem, dropped capabilities, seccomp, noexec |
| "Add a new skill" | Download into quarantine, scan with 87 malware patterns, rebuild from scratch via CDR, deliver to sandbox |
| "My assistant searched the web" | Proxy-gated request through domain allowlist, API key injection, structured logging |
| "Connect my Telegram" | DM pairing with per-channel-peer isolation |
| "Everything is safe" | Container isolation, network segmentation, tool policy enforcement, kill switches |

**The user never needs to know any of the right column exists.**

## The Three Screens (Not Components)

The app currently shows 3 "components" (Vault, Forge, Pioneer) — developer concepts. A non-technical user should see 3 screens that map to what THEY care about:

### Screen 1: My Assistant
**What it is:** Your personal AI assistant, running safely on your computer.
**What you do here:**
- See if your assistant is running (big green/red indicator)
- Start or stop your assistant
- Connect your Telegram to talk to it
- See what your assistant can do (plain language capability list)
- See that everything is safe (security status as a simple badge, not a 24-point checklist)

**Maps to:** OpenClaw Vault (the runtime)

### Screen 2: Skill Store
**What it is:** Browse and install abilities for your assistant.
**What you do here:**
- See what skills your assistant has
- Check a skill for malware before installing it
- Download and install new skills safely
- See health overview (all skills clean, tests passing)

**Maps to:** ClawHub Forge (the toolchain)

### Screen 3: Agent Network
**What it is:** Your assistant's social network — where AI agents interact.
**What you do here:**
- See if the network is safe (threat scanning)
- Check for injection attacks before your assistant sees them
- (Future: let your assistant participate in the network)

**Maps to:** Moltbook Pioneer (the network layer)
**Current status:** Moltbook API is down. Show: "Agent Network — Coming Soon"

## The Onboarding Flow (What the Wizard Should Feel Like)

Current wizard: "Let's check prerequisites → Clone submodules → Create config files → Build containers → Go to dashboard"

What it should feel like:

```
Step 1: "Welcome! Let's set up your personal AI assistant."
        [image of person messaging their phone, assistant responds]
        
Step 2: "First, we need a few things."
        ✓ Container runtime detected (Podman)
        ✓ All components ready
        "Great — everything's in place."

Step 3: "Connect your AI account."
        [Anthropic API key field] "This powers your assistant's brain."
        [Telegram bot token field] "This lets you talk to it from your phone."
        Links: "Get an API key" / "Create a Telegram bot"

Step 4: "Building your secure environment..."
        [progress bar — not raw build output]
        "Your assistant is being installed in a secure sandbox.
         It won't be able to access your personal files or passwords."

Step 5: "Your assistant is ready!"
        [big Telegram icon] "Message @YourBotName on Telegram to say hello."
        [Go to Dashboard]
```

## The Dashboard (What It Should Feel Like)

Current: Three component cards with developer labels (Vault, Forge, Pioneer).

What it should feel like:

```
┌─────────────────────────────────────────────────────┐
│                                                     │
│  Your AI Assistant                      ● Running   │
│                                                     │
│  Talk to your assistant:                            │
│  Open Telegram → message @NewLogoTrappBotBot                    │
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
│  │  Skills  │  │ Security │  │   Stop   │         │
│  │  (25)    │  │  ✓ Safe  │  │          │         │
│  └──────────┘  └──────────┘  └──────────┘         │
│                                                     │
│  What your assistant can do:                        │
│  • Search the web and summarize results             │
│  • Create and organize files                        │
│  • Schedule recurring tasks                         │
│  • Send you Telegram alerts                         │
│  • Manage 25 installed skills                       │
│                                                     │
│  ┌──────────────┐  ┌──────────────┐                │
│  │ Skill Store  │  │ Agent Network│                │
│  │ 25 skills    │  │ Coming Soon  │                │
│  │ All clean    │  │              │                │
│  └──────────────┘  └──────────────┘                │
│                                                     │
└─────────────────────────────────────────────────────┘
```

## Implementation Strategy

**The manifest-driven architecture stays.** We don't rewrite the backend. We add a presentation layer that translates developer concepts into user concepts:

| Developer concept | User concept |
|---|---|
| openclaw-vault | My Assistant |
| clawhub-forge | Skill Store |
| moltbook-pioneer | Agent Network |
| Hard Shell | Chat Only mode |
| Split Shell | Supervised mode |
| Soft Shell | (default — no name needed) |
| Workflow | Action button |
| Command (user tier) | Quick action |
| Command (advanced tier) | Hidden under "Developer Tools" |
| Health probe | Status badge |
| 24-point verification | "Safe" / "Warning" badge |
| Container logs | Hidden (developer tool) |
| Proxy logs | Hidden (developer tool) |
| component.yml | Invisible |
| compose.yml | Invisible |

**What changes:**
1. Dashboard layout — assistant-centric instead of component-centric
2. Component names in sidebar — user-friendly labels
3. Wizard step labels — narrative instead of technical
4. Security presentation — badge instead of checklist (checklist available under "Details")
5. Telegram guidance — prominent, not buried
6. Capability summary — what can my assistant DO, in plain language

**What doesn't change:**
- Manifest schema (component.yml)
- Rust backend (commands, workflows, health probes)
- Tauri IPC layer
- Container architecture (compose.yml)
- Security model (all 24 checks still run)
- Tier system (user/advanced commands)

## Success Criteria

A non-technical user who has never heard of OpenClaw, containers, or security hardening should be able to:

1. Visit opentrapp.com and understand: "This gives me a personal AI assistant"
2. Download and install in under 5 minutes
3. Complete the setup wizard without confusion
4. Send their first message to their assistant via Telegram within 10 minutes of downloading
5. Never see the words: container, seccomp, proxy, manifest, compose, vault, forge, pioneer, shell level, or component.yml
6. Feel safe — not because they checked 24 security points, but because the app told them "Your assistant is running safely" and they trust it

## What This Spec Does NOT Cover

- Redesigning the backend architecture (it's solid)
- Changing the security model (it works — 24/24 checks pass)
- Building new features (the capabilities exist, they just need better presentation)
- Mobile app (Telegram IS the mobile interface)

This is purely a **presentation and narrative** change. The engine is built. We're redesigning the dashboard of the car so the driver can actually drive it.

---

*"The security harness is not the product. It's the ENABLER. The product is: an always-on AI assistant accessible from any messaging app, that runs locally, and that you can trust because it physically cannot access what you haven't allowed."*
— from `docs/product-assessment.md` (2026-03-25)

We wrote that two months ago. It's time to build the product that matches this vision.
