# User Mode: Onboarding Flow

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`, `03-information-architecture.md`, `04-visual-assets-plan.md`, `05-automation-strategy.md`, `06-failure-ux-strategy.md`
**Screen:** Setup wizard (`/setup`)
**Rubric target:** 9+/10 across all principles

---

## Purpose

Take a user from "just double-clicked the installer" to "assistant is running, Telegram is working" in **under 3 minutes with 4 clicks**.

The current setup wizard is 6 steps and ~25 clicks. This spec condenses it to 4 logical steps and automates everything not requiring human input.

---

## User Story

> As Karen, after I download and install OpenTrApp, I want to see an obviously friendly welcome screen, answer the two or three questions only I can answer, watch the app set itself up, and then open Telegram and say hi to my new assistant. I never want to see a terminal, a component name, a file path, or a container.

---

## The 4 Steps

```
┌─────────────┐     ┌───────────┐     ┌────────────┐     ┌─────────┐
│  1. Welcome │ ──► │ 2. Connect│ ──► │ 3. Install │ ──► │ 4. Ready│
│  (1 click)  │     │ (2 inputs)│     │  (0 input) │     │ (1 link)│
└─────────────┘     └───────────┘     └────────────┘     └─────────┘
```

Replaces the current steps: Welcome, Prerequisites (System Check), Submodules (Assistant Modules), Configuration, Setup Components (Setting Up), Complete.

The current "System Check" and "Assistant Modules" steps are folded into step 3 (Install), which runs their checks automatically and shows a single unified progress experience.

---

## Step 1: Welcome

### Layout

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│                                                         │
│            [illustration-welcome.svg                    │
│             friendly logo + shield,                  │
│             320×240]                                    │
│                                                         │
│                                                         │
│          Welcome to OpenTrApp                       │
│                                                         │
│       Your personal AI assistant, safe on your         │
│       computer. Let's get you set up — it takes         │
│       about 3 minutes.                                  │
│                                                         │
│                  [  Get Started  ]                      │
│                                                         │
│                                                         │
│           Already set up? [ Skip to dashboard ]         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Copy

- **Title:** "Welcome to OpenTrApp"
- **Subtitle:** "Your personal AI assistant, safe on your computer. Let's get you set up — it takes about 3 minutes."
- **Primary CTA:** "Get Started" (large, brand primary color)
- **Secondary link:** "Already set up? Skip to dashboard" (visible only if detect a complete install)

### Visual elements

- **Hero illustration**: `onboarding/welcome.svg` (320×240)
- **Background**: subtle gradient from `bg-base` to `bg-surface`
- **Animation**: slide-up + fade-in for the content block

### Interaction

- Get Started → navigate to step 2
- Skip to dashboard → check if wizard actually complete; if so, go to `/`, else stay

### Accessibility

- h1 = "Welcome to OpenTrApp"
- Button has keyboard focus on load
- Illustration has alt text: "A friendly logo next to a shield, representing your safe AI assistant"

### State

- Single state. No loading, no error.

---

## Step 2: Connect

### Layout

```
┌────────────────────────────────────────────────────────┐
│ ●──●──○──○                                              │ ← progress: step 2 of 4
│                                                        │
│   Connect your accounts                                │
│                                                        │
│   Your assistant needs two things to work. Enter them  │
│   once and you're done. Nothing leaves your computer.  │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │ 🔑 Anthropic API key                             │ │
│  │                                                  │ │
│  │ The AI's brain. Also how you'll pay for its     │ │
│  │ thoughts (about $5–20/month for typical use).    │ │
│  │                                                  │ │
│  │ [sk-ant-api03-_____________________________]     │ │
│  │                                                  │ │
│  │ Don't have one yet?                              │ │
│  │ [ Show me how to get one (2 min) ]               │ │
│  └──────────────────────────────────────────────────┘ │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │ 💬 Telegram bot                                  │ │
│  │                                                  │ │
│  │ How you'll talk to your assistant.               │ │
│  │                                                  │ │
│  │ [1234567890:ABCdef______________________]        │ │
│  │                                                  │ │
│  │ Need to create one?                              │ │
│  │ [ Walk me through it (3 min) ]                   │ │
│  └──────────────────────────────────────────────────┘ │
│                                                        │
│  [ ← Back ]              Skip [ Continue → ]           │
└────────────────────────────────────────────────────────┘
```

### Copy

- **Title:** "Connect your accounts"
- **Subtitle:** "Your assistant needs two things to work. Enter them once and you're done. Nothing leaves your computer."
- **Anthropic card:** title "Anthropic API key", hint "The AI's brain. Also how you'll pay for its thoughts (about $5–20/month for typical use).", link "Show me how to get one (2 min)"
- **Telegram card:** title "Telegram bot", hint "How you'll talk to your assistant.", link "Walk me through it (3 min)"
- **Back:** "← Back"
- **Skip:** "Skip" (less prominent)
- **Continue:** "Continue →" (primary, disabled until at least one key entered OR user clicks skip)

### Visual elements

- Progress bar at top: 4 dots (2nd filled)
- Key icon + message icon on each card
- Inline validation: green checkmark when key format matches pattern
- Link icons on the "how to get one" links

### Interaction

- **Paste detection:** if user pastes a string starting with `sk-ant-` into the Telegram field (or vice versa), auto-swap to the correct field.
- **Inline validation:** real-time regex check for key formats. Show green check when valid; don't block if invalid (user might paste weird formats).
- **Existing keys:** if `.env` already has valid keys, pre-populate (masked as `••••••••last4chars`) with a "Change" link.
- **Show me how to get one:** opens a modal with:
  - Screenshots (from `help-screenshots/anthropic-key/`)
  - Numbered steps
  - A "Done, let me enter it" button to close the modal
- **Walk me through it (Telegram):** similar modal with `help-screenshots/telegram-bot/`
- **Skip:** sets a flag `skipped_keys: true`, continues to install (assistant will be paused until keys added later)

### Accessibility

- Form fields have labels, hints, and `aria-describedby` pointing to their help text
- Paste detection announces via `aria-live` when a swap happens ("That looks like an Anthropic key; moved to the right field")

### State

- Pristine: both fields empty, Continue enabled (via Skip)
- One entered: Continue still works
- Both entered + valid: green checkmarks, Continue emphasized
- Validation error (rare): inline red hint, doesn't block

---

## Step 3: Install

### Layout

```
┌────────────────────────────────────────────────────────┐
│ ●──●──●──○                                              │ ← progress: step 3 of 4
│                                                        │
│           [illustration-installing.svg                 │
│            concentric rings pulsing,                   │
│            320×240]                                    │
│                                                        │
│     Setting up your assistant                          │
│                                                        │
│     This usually takes 2–3 minutes.                    │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │  ✓  Checked your computer                        │ │
│  │  ✓  Downloaded the AI parts                      │ │
│  │  ⋯  Building your assistant... 45s               │ │
│  │  ·  Testing safety checks                        │ │
│  └──────────────────────────────────────────────────┘ │
│                                                        │
│                                                        │
│                                                        │
│  ▸ Show technical details                              │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### Copy

- **Title:** "Setting up your assistant"
- **Subtitle:** "This usually takes 2–3 minutes."
- **Step labels:**
  - "Checked your computer" (was: prerequisites check)
  - "Downloaded the AI parts" (was: submodule init)
  - "Building your assistant" (was: component setup)
  - "Testing safety checks" (was: security verification)
- **Technical details toggle:** "Show technical details" (collapsed)

### Visual elements

- **Pulsing concentric rings illustration** while running (`installing.svg` with CSS animation)
- **Checklist with animated states:**
  - Pending: gray dot `·`
  - Running: small spinner `⋯`
  - Done: green check `✓`
  - Failed: amber `⚠` with inline retry
- **Time estimates** next to running item (updates live)
- **No raw build output** by default

### Interaction

- All 4 sub-steps run **automatically** and **in parallel where possible**:
  - Step A (Check computer): Podman detection, disk space, network
  - Step B (Download AI parts): `git submodule update --init`
  - Step C (Build assistant): parallel builds of vault + forge + pioneer containers
  - Step D (Safety checks): run 24-point audit

- Steps A and B can run in parallel; step C waits; step D waits for C.

- **Auto-retry on transient failures** (per automation spec). User sees "Building your assistant (retry)..." briefly.

- **If a step truly fails** after retries: that line turns amber with "Let's try again" button (Level 2 friendly retry).

- **If all retries fail**: navigate to contact-support screen (Level 3).

- **Show technical details:** expands into a scrollable log panel showing the raw Podman build output. Developer-level view, collapsed by default.

### Time estimates

- Show live timers for each running step
- Show a total ETA at the top (calculated based on step estimates): "About 2 minutes remaining"

### Success

- When all steps complete (all green checks), auto-advance to step 4 after 1 second. Brief scale animation on the checklist as it settles.

### Failure handling

| Sub-step | If fails | Behavior |
|----------|----------|----------|
| Check computer | Missing container runtime | Inline: "You'll need Podman or Docker installed first" + install button/link per OS |
| Download AI parts | Network error | Auto-retry. Level 2 if fails. |
| Build assistant | Container build fails | Auto-retry (proxy timing bug). Level 2 if fails. |
| Safety checks | Audit fails | Auto-retry. Level 3 if fails (unlikely but serious). |

### Accessibility

- Status announces via `aria-live`: "Step 2 of 4 complete. Now building your assistant."
- Each step has an `aria-current` attribute

### State

- All pending → all running (sequential or parallel) → all done
- On failure, see failure handling above

---

## Step 4: Ready

### Layout

```
┌────────────────────────────────────────────────────────┐
│ ●──●──●──●                                              │ ← progress: step 4 of 4 (all filled)
│                                                        │
│          [illustration-ready.svg                       │
│           confetti + logo waving,                   │
│           celebratory colors,                          │
│           320×240]                                     │
│                                                        │
│        Your assistant is ready! 🎉                     │
│                                                        │
│     Say hi on Telegram to get started.                 │
│                                                        │
│      ┌─────────────────────────────────────┐           │
│      │ 💬  Open Telegram                   │           │
│      └─────────────────────────────────────┘           │
│                                                        │
│       Not now [ Go to dashboard ]                      │
│                                                        │
│                                                        │
│  💡 Tip: You can ask your assistant things like        │
│     "What's the weather?" or "Plan my Tuesday."        │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### Copy

- **Title:** "Your assistant is ready! 🎉"
- **Subtitle:** "Say hi on Telegram to get started."
- **Primary CTA:** "Open Telegram" (opens `https://t.me/{bot_username}` if we have it, else generic telegram.org)
- **Secondary CTA:** "Go to dashboard"
- **Tip:** "You can ask your assistant things like \"What's the weather?\" or \"Plan my Tuesday.\""

### Visual elements

- **Celebration illustration** `ready.svg` with confetti
- **Scale + fade-in** entrance with spring easing (celebration feel)
- **Optional sound** (if OS allows): gentle success chime
- **Optional system notification**: "Your assistant is ready! Say hi on Telegram."

### Interaction

- **Open Telegram:** deep-link if bot username is derivable from token, else generic link
- **Go to dashboard:** mark `wizardCompleted: true`, navigate to `/`
- **Auto-advance:** if user takes no action for 5 seconds, a small countdown appears showing "Taking you to dashboard in 5..." — gives user agency.

### Accessibility

- Focus lands on "Open Telegram" button
- Celebration animation respects `prefers-reduced-motion`

### State

- Single state. Success always displayed here.

---

## Progress Indicator

Persistent at top of steps 2–4:

```
●──●──○──○    Connect · Install · Ready
```

4 dots connected by lines. Current step filled, completed steps filled, future steps empty.

Below the dots, optionally show step labels on wider screens.

---

## Navigation Rules

- **Back:** always available on step 2 (goes to Welcome). Disabled during Install and Ready.
- **Skip:** only on step 2 (to skip keys).
- **Esc:** no effect. Users can't abort mid-wizard by accident.
- **Window close:** persists `setupProgress` so next launch resumes.

---

## Data Flow

```
User input ──► React state
                │
                ├──► On "Continue": save to .env via Tauri
                │    (writeConfig command, existing)
                │
                └──► On completion: updateSettings({ wizardCompleted: true })
```

---

## Copy / Text Bank (Final)

All strings to use in the wizard. Implementer should not invent new text; use these verbatim or propose changes in PR comments.

```json
{
  "welcome.title": "Welcome to OpenTrApp",
  "welcome.subtitle": "Your personal AI assistant, safe on your computer. Let's get you set up — it takes about 3 minutes.",
  "welcome.cta": "Get Started",
  "welcome.skipToDashboard": "Already set up? Skip to dashboard",

  "connect.title": "Connect your accounts",
  "connect.subtitle": "Your assistant needs two things to work. Enter them once and you're done. Nothing leaves your computer.",
  "connect.anthropic.label": "Anthropic API key",
  "connect.anthropic.hint": "The AI's brain. Also how you'll pay for its thoughts (about $5–20/month for typical use).",
  "connect.anthropic.cta": "Show me how to get one (2 min)",
  "connect.telegram.label": "Telegram bot",
  "connect.telegram.hint": "How you'll talk to your assistant.",
  "connect.telegram.cta": "Walk me through it (3 min)",
  "connect.continue": "Continue",
  "connect.skip": "Skip",
  "connect.back": "Back",
  "connect.pasteSwap": "That looks like {type}; moved to the right field.",

  "install.title": "Setting up your assistant",
  "install.subtitle": "This usually takes 2–3 minutes.",
  "install.step.check": "Check your computer",
  "install.step.download": "Download the AI parts",
  "install.step.build": "Build your assistant",
  "install.step.safety": "Test safety checks",
  "install.details": "Show technical details",
  "install.eta": "About {minutes} {minutes, plural, one {minute} other {minutes}} remaining",

  "ready.title": "Your assistant is ready! 🎉",
  "ready.subtitle": "Say hi on Telegram to get started.",
  "ready.openTelegram": "Open Telegram",
  "ready.goToDashboard": "Go to dashboard",
  "ready.tip": "You can ask your assistant things like \"What's the weather?\" or \"Plan my Tuesday.\"",
  "ready.autoAdvance": "Taking you to dashboard in {seconds}..."
}
```

---

## Acceptance Criteria

- [ ] Total time from "Get Started" to "Your assistant is ready!" on reference hardware (M2 Mac or modern Linux): **under 3 minutes** with valid API key + Telegram token.
- [ ] Clicks from Welcome to Ready: **4 total** (Get Started → enter keys → Continue → Open Telegram).
- [ ] Zero raw Podman output visible unless "Show technical details" toggled.
- [ ] No developer terminology (no "container", "proxy", "manifest", "submodule", "component").
- [ ] Setup wizard resumes from last step on app crash mid-wizard.
- [ ] All failure states route through Level 2 friendly retry before escalating.
- [ ] Score on UX rubric: **≥ 9/10 on every applicable principle**.

---

## Test Plan

### Playwright E2E

```ts
test('happy path: welcome → connect → install → ready', async ({ page }) => {
  // Navigate to /setup
  // Click Get Started
  // Enter valid API key format
  // Enter valid Telegram token format
  // Click Continue
  // Wait for install to complete (mock backend to respond in <10s)
  // Verify "Your assistant is ready!" heading
  // Verify "Open Telegram" button is focused
});

test('skip keys: can complete setup without keys', async ({ page }) => {
  // Navigate, click Get Started
  // Click Continue (with empty fields) or Skip
  // Install runs with empty .env
  // Ready screen appears
});

test('paste swap: pasting anthropic key in telegram field auto-swaps', async ({ page }) => {
  // ...
});

test('no developer jargon visible in any step', async ({ page }) => {
  // Navigate through each step
  // Assert banned terms absent at each step
});
```

### Unit tests

- `wizardState` reducer transitions correctly on step advance
- Paste-swap logic correctly identifies Anthropic vs Telegram formats
- Auto-retry wrapper retries correct number of times

### Manual QA

- Run on Linux, macOS, Windows
- Run on a slow network (throttle in DevTools to 3G)
- Run with Podman not installed (expect install guidance on step 3)
- Run with existing `.env` (expect pre-populated masked values)

---

## Files to Change / Create

| Action | File | Change |
|--------|------|--------|
| Modify | `app/src/pages/Setup.tsx` | Refactor from 6 steps to 4; change STEP_ORDER |
| Modify | `app/src/components/wizard/WelcomeStep.tsx` | Use new illustration, updated copy |
| Modify | `app/src/components/wizard/ConfigStep.tsx` | Rename to ConnectStep; apply new UX |
| Remove / Merge | `app/src/components/wizard/PrerequisitesStep.tsx` | Logic moves into InstallStep |
| Remove / Merge | `app/src/components/wizard/SubmodulesStep.tsx` | Logic moves into InstallStep |
| Modify | `app/src/components/wizard/SetupComponentsStep.tsx` | Rename to InstallStep; merge checks + builds + safety |
| Modify | `app/src/components/wizard/CompleteStep.tsx` | Rename to ReadyStep; add celebration |
| Create | `app/src/components/wizard/WizardProgress.tsx` | Shared 4-dot progress bar |
| Create | `app/src/components/wizard/HowToModal.tsx` | Modal with screenshots (for "how to get key" guides) |

---

## Next

Read `08-home-dashboard.md` — the screen Karen lands on after setup completes.
