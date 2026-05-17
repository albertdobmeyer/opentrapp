# User Mode: Preferences

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`, `05-automation-strategy.md`
**Screen:** `/preferences`
**Rubric target:** 9+/10 across all principles

---

## Purpose

Expose **only** the settings Karen cares about. Everything technical lives in dev mode. This screen must have **as few controls as possible** — good defaults mean Karen rarely visits.

---

## User Story

> As Karen, I want to update my API key when I rotate it, change my monthly spending limit, or turn off notifications. I don't want to see anything about containers, paths, or developer tools unless I explicitly ask.

---

## Layout

```
┌──────────────────────────────────────────────────────────────┐
│  Preferences                                                 │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 🔑 Your keys                                            │ │
│  │                                                        │ │
│  │ Anthropic API key                                      │ │
│  │ ••••••••last4chars                 [ Change ]          │ │
│  │                                                        │ │
│  │ Telegram bot token                                     │ │
│  │ ••••••••last4chars                 [ Change ]          │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 💰 Spending                                            │ │
│  │                                                        │ │
│  │ Monthly limit                                          │ │
│  │   ○ $5     ● $20     ○ $50     ○ Custom: [____]        │ │
│  │                                                        │ │
│  │ Alert me when I reach                                  │ │
│  │   50% ────●──────────── 100%        (currently 80%)    │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 🔔 Notifications                                       │ │
│  │                                                        │ │
│  │ ☑ Security alerts                                      │ │
│  │ ☑ Spending warnings                                    │ │
│  │ ☑ App updates                                          │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ ⚡ Startup                                              │ │
│  │                                                        │ │
│  │ ☑ Start OpenTrApp when I turn on my computer       │ │
│  │ ☑ Keep it running in the background                    │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 🔄 Re-run setup                                        │ │
│  │                                                        │ │
│  │ Run through the setup wizard again.                    │ │
│  │ Useful if you got a new computer.                      │ │
│  │                                                        │ │
│  │ [ Re-run setup ]                                       │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 🔧 Advanced Mode                                       │ │
│  │                                                        │ │
│  │ Unlocks detailed views for developers, security        │ │
│  │ researchers, and power users.                          │ │
│  │                                                        │ │
│  │ Most people won't need this.                           │ │
│  │                                                        │ │
│  │ ☐ Enable Advanced Mode                                 │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  OpenTrApp v0.2.0                                        │
│  Made with care for people who want AI without the stress.   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Sections

### 1. Your keys

Shows current keys (masked) with a "Change" button per key.

**Change flow:**
- Click "Change" → modal appears
- Modal contains:
  - Label and hint
  - Input field (prepopulated with masked value, or empty if user clicks "Clear")
  - Link to "how to get one" (opens the same modal from onboarding)
  - "Cancel" and "Save" buttons
- On Save:
  - Validate format
  - Write to `.env`
  - Trigger a connection test (call `validate_api_key()` command)
  - If valid: toast "Key updated" and restart the vault container automatically
  - If invalid: show inline error in modal

### 2. Spending

**Monthly limit:** Radio buttons for preset amounts + custom input.
**Alert threshold:** Slider from 50–100%. Default 80%.

When user changes the limit:
- If current spend > new limit: warning "You've already spent more than this new limit. We'll alert you immediately."
- If current spend > alert threshold of new limit: notification fired immediately

### 3. Notifications

Three toggles, default all ON. Each toggle reflects the Tauri permission status; if system notifications aren't permitted, show:

```
⚠️ OpenTrApp needs permission to send notifications.
   [ Grant permission ]
```

### 4. Startup

Two toggles:
- "Start OpenTrApp when I turn on my computer" — uses `tauri-plugin-autostart`
- "Keep it running in the background" — controls close-to-tray behavior

### 5. Re-run setup

A single button that:
- Asks for confirmation ("Re-run setup? Your keys and preferences will be kept.")
- On confirm: sets `wizardCompleted: false`, navigates to `/setup`

### 6. Advanced Mode (at the bottom)

A collapsed, de-emphasized section with an honest explanation and a toggle.

On toggle ON:
- Show welcome dialog (from spec 03)
- Navigate to `/dev`

On toggle OFF (from preferences):
- Navigate to `/`

---

## Save behavior

Preferences auto-save on change. No explicit "Save" button. Show a small toast for important changes:

- "Monthly limit updated to $50" ✓
- "API key updated" ✓
- "Notifications turned off" ✓
- "Starting with your computer" ✓

Minor changes (slider positions) update silently.

---

## Copy Bank

```json
{
  "page.title": "Preferences",
  "section.keys.title": "Your keys",
  "section.keys.anthropic.label": "Anthropic API key",
  "section.keys.telegram.label": "Telegram bot token",
  "section.keys.changeBtn": "Change",
  "section.keys.modal.title.anthropic": "Update your Anthropic API key",
  "section.keys.modal.title.telegram": "Update your Telegram bot token",
  "section.keys.modal.saveBtn": "Save",
  "section.keys.modal.cancelBtn": "Cancel",
  "section.keys.modal.howToLink": "How do I get one?",
  "section.keys.updatedToast": "Key updated",
  "section.keys.validationError": "That doesn't look like a valid key. Double-check and try again.",

  "section.spending.title": "Spending",
  "section.spending.limit.label": "Monthly limit",
  "section.spending.limit.custom": "Custom",
  "section.spending.threshold.label": "Alert me when I reach",
  "section.spending.threshold.sublabel": "({percent}% of your limit)",
  "section.spending.exceedWarning": "You've already spent more than this new limit. We'll alert you immediately.",

  "section.notifications.title": "Notifications",
  "section.notifications.security": "Security alerts",
  "section.notifications.spending": "Spending warnings",
  "section.notifications.updates": "App updates",
  "section.notifications.permissionNeeded": "OpenTrApp needs permission to send notifications.",
  "section.notifications.grantBtn": "Grant permission",

  "section.startup.title": "Startup",
  "section.startup.autostart": "Start OpenTrApp when I turn on my computer",
  "section.startup.background": "Keep it running in the background",

  "section.resetup.title": "Re-run setup",
  "section.resetup.subtitle": "Run through the setup wizard again. Useful if you got a new computer.",
  "section.resetup.btn": "Re-run setup",
  "section.resetup.confirm.title": "Re-run setup?",
  "section.resetup.confirm.body": "Your keys and preferences will be kept.",
  "section.resetup.confirm.cancelBtn": "Cancel",
  "section.resetup.confirm.okBtn": "Re-run setup",

  "section.advanced.title": "Advanced Mode",
  "section.advanced.description": "Unlocks detailed views for developers, security researchers, and power users.",
  "section.advanced.warning": "Most people won't need this.",
  "section.advanced.toggle": "Enable Advanced Mode",

  "footer.version": "OpenTrApp v{version}",
  "footer.tagline": "Made with care for people who want AI without the stress."
}
```

---

## Removed from user preferences (moved to dev mode)

These settings exist today but are removed from user mode. They live only in `/dev/preferences`:

- Monorepo Path / App Data Location (moved entirely; default is auto-detected)
- Status Refresh Interval (moved)
- Manifest inspector settings (moved)
- Any component-specific toggles (moved)

---

## Acceptance Criteria

- [ ] No more than 6 sections
- [ ] No developer jargon visible
- [ ] Every change triggers appropriate feedback (toast, spinner, etc.)
- [ ] API key change triggers container restart automatically
- [ ] Auto-save works without "Save" button
- [ ] Advanced Mode is visibly de-emphasized
- [ ] Rubric score ≥ 9/10

---

## Files to Change / Create

| Action | File | Notes |
|--------|------|-------|
| Replace | `app/src/pages/Settings.tsx` | Rewrite as Preferences per this spec |
| Create | `app/src/components/user/KeyChangeModal.tsx` | Modal for updating keys |
| Create | `app/src/components/user/SpendingLimitPicker.tsx` | Radio + custom input |
| Create | `app/src/components/user/NotificationToggles.tsx` | Grouped toggles |
| Create | `app/src/components/user/StartupToggles.tsx` | Autostart + tray toggles |
| Create | `app/src-tauri/src/commands/validate_api_key.rs` | Test key against Anthropic API |
| Create | `app/src-tauri/src/commands/autostart.rs` | Enable/disable autostart |

---

## Next

Read `11-help-and-support.md`.
