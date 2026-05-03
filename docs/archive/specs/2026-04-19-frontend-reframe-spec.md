# Frontend Reframe Spec — Detailed Implementation

**Date:** 2026-04-19
**Prerequisite:** `docs/specs/2026-04-19-product-identity-spec.md`
**Scope:** 8 frontend files, presentation changes only, no backend modifications

---

## Guiding Principle

Every user-facing string must answer the question a non-technical user would ask. Replace developer explanations ("security perimeter," "ecosystem components") with user benefits ("your assistant is ready," "safe on your computer").

---

## File 1: Sidebar.tsx (82 lines)

### Current
```
Lobster-TrApp
OpenClaw Orchestrator

[Dashboard icon] Dashboard

COMPONENTS
[icon] ClawHub Forge
[icon] Moltbook Pioneer
[icon] OpenClaw Vault

[Settings icon] Settings
```

### Target
```
Lobster-TrApp

[Dashboard icon] Dashboard

[icon] My Assistant
[icon] Skills
[icon] Network

[Settings icon] Settings
```

### Changes

1. **Remove subtitle** "OpenClaw Orchestrator" — user doesn't know what this means
2. **Remove "COMPONENTS" section header** — the items speak for themselves
3. **Map component names by role:**
   - `identity.role === "runtime"` → "My Assistant"
   - `identity.role === "toolchain"` → "Skills"
   - `identity.role === "network"` → "Network"
   - fallback → `identity.name` (preserves generic renderer for unknown components)

### Exact string changes
| Line | Current | New |
|------|---------|-----|
| Header subtitle | `"OpenClaw Orchestrator"` | Remove entirely |
| Section header | `"Components"` | Remove entirely |
| Nav item text | `{identity.name}` | Role-based mapping function |

---

## File 2: Dashboard.tsx (96 lines)

### Current
```
Dashboard
OpenClaw ecosystem components
[Refresh]

[onboarding banner: "Setup complete — your security perimeter is ready.
 Click a component... Start with OpenClaw Vault...
 Open Vault Dashboard"]

[3 ComponentCards in grid]
```

### Target
```
Dashboard
[Refresh]

[assistant status card — prominent, role=runtime component]
  Your AI Assistant          ● Running
  Talk to your assistant on Telegram → @NewLobsterTrappBotBot

  [Security Check]  [Stop Assistant]  [Skills (25)]

[onboarding: "Your assistant is ready! Message @NewLobsterTrappBotBot on Telegram to get started."]

[2 secondary cards: Skills + Network]
```

### Changes

1. **Remove subtitle** "OpenClaw ecosystem components"
2. **Rewrite onboarding banner:**
   - Title: "Your assistant is ready!"
   - Body: "Message your bot on Telegram to start a conversation. You can also check your skills or run a security audit from this dashboard."
   - Link: "Learn what your assistant can do" → `/component/openclaw-vault`
3. **Split dashboard into primary + secondary:**
   - Find the `runtime` component → render as a prominent status card (not a grid card)
   - Remaining components → render as smaller secondary cards below
4. **Primary card (runtime component):**
   - Show name as "Your AI Assistant" (not component name)
   - Show status badge prominently
   - Show Telegram guidance: "Message @YourBotName on Telegram"
   - Show 2-3 quick action buttons: Security Check, Stop/Start, Skills count
5. **Secondary cards (non-runtime):**
   - Show role-based labels ("Skills", "Network") not component names
   - Show status + brief description
   - Smaller layout (no full ComponentCard)
6. **Empty state:** "No assistant detected. Run the setup wizard to get started." (not "No components detected yet")

---

## File 3: WelcomeStep.tsx (28 lines)

### Current
```
[logo]
Welcome to Lobster-TrApp
Your security-first desktop GUI for the OpenClaw ecosystem.
Let's check that everything is set up correctly.
[Let's get started]
```

### Target
```
[logo]
Welcome to Lobster-TrApp
Let's set up your personal AI assistant.
It'll run safely on your computer — you control it from Telegram.
[Get Started]
```

### Exact string changes
| Current | New |
|---------|-----|
| "Welcome to Lobster-TrApp" | "Welcome to Lobster-TrApp" (keep) |
| "Your security-first desktop GUI for the OpenClaw ecosystem. Let's check that everything is set up correctly." | "Let's set up your personal AI assistant. It'll run safely on your computer — you control it from Telegram." |
| "Let's get started" | "Get Started" |

---

## File 4: SetupComponentsStep.tsx (326 lines)

### Current
```
Set Up Components
Run the initial setup for each component. Container builds may take a few minutes.

[component name]
Ready to set up / Setting up... / Setup complete / Failed (exit N)
[required] badge
[raw streaming build output in monospace]
```

### Target
```
Setting Up Your Assistant
This may take a few minutes. We're building a secure environment for your assistant.

[user-friendly label based on role]
Ready / Setting up... / Ready to go / Something went wrong
[raw output hidden by default, "Show details" toggle]
```

### Changes

1. **Title:** "Set Up Components" → "Setting Up Your Assistant"
2. **Subtitle:** "Run the initial setup for each component. Container builds may take a few minutes." → "This may take a few minutes. We're building a secure environment for your assistant."
3. **Component labels:** Show role-based name, not `identity.name`
   - runtime → "Your AI Assistant"
   - toolchain → "Skill Scanner"
   - network → "Network Monitor"
4. **Status text:**
   - "Ready to set up" → "Ready"
   - "Setting up..." → "Setting up..."  (keep)
   - "Setup complete" → "Ready to go"
   - "Failed (exit N)" → "Something went wrong — click Retry" (hide exit code)
5. **"required" badge** → remove (confusing — why is one required and others aren't?)
6. **Build output:** Hidden by default behind "Show details" toggle. Still available for debugging but not shown to users who'd be confused by Podman build logs.

---

## File 5: CompleteStep.tsx (96 lines)

### Current
```
[green checkmark]
All Set!
Your environment is ready. You can start components now or head to the dashboard.

[component name] [Start button]
[component name] [Start button]

[Go to Dashboard]
```

### Target
```
[green checkmark]
Your Assistant is Ready!
Message @YourBotName on Telegram to start chatting.

[Telegram icon] Open Telegram

[Go to Dashboard]
```

### Changes

1. **Title:** "All Set!" → "Your Assistant is Ready!"
2. **Subtitle:** "Your environment is ready. You can start components now or head to the dashboard." → "Message your bot on Telegram to start chatting. You can manage your assistant from the dashboard."
3. **Remove component start list** — the vault was already started during setup. Don't confuse users with per-component start buttons.
4. **Add Telegram link** — prominent button to open Telegram
5. **"Go to Dashboard" stays** — but becomes secondary

---

## File 6: ComponentDetail.tsx (273 lines)

### Current
```
← Dashboard
[icon] OpenClaw Vault
v0.1.0 · runtime                [Running]
Hardened container for the OpenClaw agent runtime

HEALTH: [Container: running] [Security: 24 checks]

[contextual guidance card]

WORKFLOWS: [Start Securely] [Maximum Restriction] ...
COMMANDS: [Start] [Stop] ... Advanced (9)
CONFIGURATION: [Environment] [Allowlist] [Security Config]
```

### Target (for runtime role)
```
← Dashboard
[icon] Your AI Assistant
Running safely                   [● Running]

Talk to your assistant by messaging @YourBotName on Telegram.
It can search the web, manage files, and schedule tasks — all within
safe boundaries. Your personal files and passwords are protected.

[Security: ✓ Safe]

ACTIONS: [Security Audit] [Connect Telegram] [Stop Assistant]

SKILLS: 25 installed, all clean

▸ Developer Tools (workflows, commands, configs)
```

### Changes

1. **Header: role-based name mapping**
   - runtime → "Your AI Assistant"
   - toolchain → "Skills"
   - network → "Agent Network"
   - fallback → `identity.name`
2. **Remove version + role badge** from user view (move to developer tools)
3. **Remove manifest description** ("Hardened container for the OpenClaw agent runtime" means nothing to users)
4. **Contextual guidance card:** Keep and improve
   - Running: Add Telegram bot name guidance, capability summary
   - Stopped: "Your assistant is stopped. Click Start to bring it back online."
   - Not setup: "Your assistant needs to be set up first. Go back to the setup wizard."
5. **Health → simplified**
   - Show security as single badge: "✓ Safe" (green) or "⚠ Check needed" (amber)
   - Hide "Container: running" (redundant with status badge)
6. **Workflows + Commands + Configs** → collapse into "Developer Tools" toggle (same pattern as current Advanced toggle but for the entire bottom section)
   - Exception: user-tier commands (Start, Stop, Security Check, Connect Telegram) stay visible as "Quick Actions"
7. **For toolchain role:** Show skill health summary prominently ("25 skills, all clean, 168 tests passing")
8. **For network role:** Show API status notice ("Agent Network — coming soon. The Moltbook API is currently unavailable.")

---

## File 7: ComponentCard.tsx (91 lines)

### Current
```
[icon] ClawHub Forge
v0.1.0                          [Ready]
Skill development workbench and security scanner
                                 Toolchain
```

### Target
```
[icon] Skills
25 skills installed, all clean   [Ready]
```

### Changes

1. **Card title:** Role-based name mapping (same function as Sidebar)
2. **Remove version** from cards (noise for users)
3. **Remove manifest description** (developer jargon)
4. **Remove role badge** at bottom
5. **Add contextual subtitle based on role:**
   - runtime → status-based: "Running" / "Stopped" / "Not set up"
   - toolchain → health-based: "X skills installed, all clean" (from health probes)
   - network → API status: "Coming soon" or "Active"
6. **Placeholder cards:** Keep "Coming Soon" badge

---

## Shared: Role Label Mapping Function

Create a utility used by Sidebar, Dashboard, ComponentDetail, ComponentCard, and SetupComponentsStep:

```typescript
// app/src/lib/labels.ts
export function getUserLabel(role: string): string {
  switch (role) {
    case "runtime": return "My Assistant";
    case "toolchain": return "Skills";
    case "network": return "Network";
    default: return role;
  }
}

export function getSetupLabel(role: string): string {
  switch (role) {
    case "runtime": return "Your AI Assistant";
    case "toolchain": return "Skill Scanner";
    case "network": return "Network Monitor";
    default: return role;
  }
}
```

---

## What Does NOT Change

- `app/src/App.tsx` — routing stays the same
- `app/src/components/WorkflowPanel.tsx` — already has good user_description text
- `app/src/components/CommandPanel.tsx` — already has user/advanced tier split
- `app/src/hooks/*` — no hook changes
- `app/src/lib/tauri.ts` — no IPC changes
- `app/src/lib/types.ts` — no type changes (tier field already added)
- All Rust backend code
- All component.yml manifests (tier tags already added)
- All test files (may need mock updates for new labels)

---

## Verification

1. `cd app && npx tsc --noEmit` — zero type errors
2. `cd app && npm test` — 147 tests pass (update mocks if needed)
3. `cd app && npm run tauri dev` — visual check:
   - Sidebar shows: My Assistant, Skills, Network
   - Dashboard shows assistant status card prominently
   - Wizard welcome says "personal AI assistant"
   - Setup step says "Setting Up Your Assistant"
   - Complete step says "Your Assistant is Ready!"
   - Vault detail shows "Your AI Assistant" with Telegram guidance
4. `bash tests/orchestrator-check.sh` — 42 checks pass
