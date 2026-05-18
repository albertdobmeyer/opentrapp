# Developer Mode: Dashboard Overview

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`, `03-information-architecture.md`
**Route:** `/dev`
**Audience:** Power users, security researchers, contributors

---

## Purpose

Give developers a dense, information-rich control surface for OpenTrApp's internals. Everything a developer would need to understand, debug, extend, or fork is accessible from one shell.

**The dev dashboard reuses as much of the current component-based UI as possible.** The existing `ComponentDetail`, `CommandPanel`, `WorkflowPanel`, and `ConfigPanel` components are already well-suited; they get rehomed into the dev shell with minor refactors.

---

## User Story

> As a developer, when I toggle on Advanced Mode, I want to see every component's state, every command's output, every configuration, every log, and every security check on one screen. I want keyboard shortcuts, real-time updates, and the ability to edit configs, allowlists, and shell levels without leaving the app.

---

## Entry

The user enters dev mode via:
1. Settings → Advanced Mode toggle (persisted in store as `mode: 'developer'`)
2. Keyboard shortcut `Cmd/Ctrl+Shift+D`
3. System tray → "Advanced Mode" menu item

On first entry ever, show the welcome dialog (spec 03).

---

## Layout

```
┌────────────────────────────────────────────────────────────────────┐
│ OpenTrApp · Advanced Mode          [Exit Advanced] [⌘⇧D]    │  top bar (40px)
├─────────────────┬──────────────────────────────────────────────────┤
│                 │                                                  │
│ SYSTEM          │                                                  │
│ · Overview      │                                                  │
│ · Logs          │                                                  │
│                 │                                                  │
│ COMPONENTS      │            Main content area                     │
│ · opencli-container│            (tabbed workspace or                  │
│ · openskill-forge │             selected screen)                     │
│ · moltbook-     │                                                  │
│   pioneer       │                                                  │
│                 │                                                  │
│ SECURITY        │                                                  │
│ · Audit         │                                                  │
│ · Allowlist     │                                                  │
│ · Shell levels  │                                                  │
│                 │                                                  │
│ INSPECTION      │                                                  │
│ · Manifests     │                                                  │
│                 │                                                  │
│ ─────           │                                                  │
│ ⚙ Settings      │                                                  │
│                 │                                                  │
│ 240px           │                                                  │
└─────────────────┴──────────────────────────────────────────────────┘
```

### Top bar

- Left: App name + mode badge ("Advanced Mode")
- Right: "Exit Advanced" button (→ toggles mode off, goes to `/`), keyboard shortcut hint
- Height: 40px, dense padding, `bg-app` background

### Left sidebar

- Width: 240px fixed
- Background: `bg-surface`
- Nav items grouped by category with uppercase labels
- Active route highlighted with bg-raised + left border accent
- Icon + label per item (Lucide icons)

### Main content area

- Full remaining width, no max-width constraint
- Padding: `p-4` (dense vs user mode's p-8)
- Scrollable vertically
- Each sub-screen has its own layout (see `14-component-operations.md` for per-screen details)

---

## Routes

```
/dev                            Overview (dashboard landing)
/dev/logs                       Unified logs viewer (NEW)
/dev/components                 Component grid (rehomed)
/dev/components/:id             Component detail (rehomed)
/dev/security                   Security audit runner (NEW)
/dev/allowlist                  Allowlist editor (NEW)
/dev/shell-levels               Shell level configuration (NEW)
/dev/manifests                  Manifest inspector (NEW)
/dev/preferences                Full technical preferences (expanded from user prefs)
```

---

## /dev (Overview Landing)

**The first screen a developer sees.** High-density summary.

```
┌────────────────────────────────────────────────────────────────┐
│ System Overview                                                │
│                                                                │
│ ┌─────────────────────┐ ┌─────────────────────┐               │
│ │ opencli-container      │ │ openskill-forge       │               │
│ │ ● running           │ │ ● ready             │               │
│ │ v0.1.0 · runtime    │ │ v0.1.0 · toolchain  │               │
│ │                     │ │                     │               │
│ │ mem 180M / cpu 2%   │ │ mem 42M / cpu 0.1%  │               │
│ │ uptime 2h 14m       │ │ uptime 2h 14m       │               │
│ │ 4 user cmds         │ │ 3 user cmds         │               │
│ │ 2 workflows         │ │ 4 workflows         │               │
│ │                     │ │                     │               │
│ │ [ Open ] [ Stop ]   │ │ [ Open ] [ Stop ]   │               │
│ └─────────────────────┘ └─────────────────────┘               │
│                                                                │
│ ┌─────────────────────┐ ┌─────────────────────┐               │
│ │ openagent-social    │ │ Security status     │               │
│ │ ◐ placeholder       │ │ ✓ 24/24 checks pass │               │
│ │ v0.0.1 · network    │ │ Last audit 2m ago   │               │
│ └─────────────────────┘ └─────────────────────┘               │
│                                                                │
│ Recent activity                                                │
│                                                                │
│ 15:30:42  [vault]    INFO  Telegram message received           │
│ 15:30:43  [proxy]    INFO  anthropic.com 200 (242ms)           │
│ 15:30:45  [vault]    INFO  Tool call: web_fetch weather.com    │
│ 15:30:46  [proxy]    INFO  weather.com 200 (118ms)             │
│ 15:30:48  [vault]    INFO  Telegram message sent               │
│ ...                                                            │
│ [ View full logs → ]                                           │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### Component cards (dev view)

- Canonical name visible (e.g., "opencli-container", not "My Assistant")
- Version + role badge
- Live metrics: memory, CPU, uptime (polled)
- Command/workflow counts
- Actions: Open (→ component detail), Stop (stops container)

### Security status card

- 24-point audit pass/fail count
- Last audit timestamp
- Click → `/dev/security`

### Recent activity tail

- Last 20 log entries across all components
- Compact, monospaced
- Click entry → jump to that component's logs
- "View full logs" → `/dev/logs`

---

## Keyboard Shortcuts

Global in dev mode:

- `Cmd/Ctrl + K` → Command palette (fuzzy search components, commands, configs)
- `Cmd/Ctrl + L` → Jump to `/dev/logs`
- `Cmd/Ctrl + 1` → `/dev` (Overview)
- `Cmd/Ctrl + 2` → `/dev/components`
- `Cmd/Ctrl + 3` → `/dev/logs`
- `Cmd/Ctrl + 4` → `/dev/security`
- `Cmd/Ctrl + Shift + R` → Refresh all component statuses
- `Cmd/Ctrl + Shift + D` → Exit Advanced Mode
- `?` → Show keyboard shortcuts help dialog

Vim-style (optional, nice-to-have):
- `g` then `o` → overview
- `g` then `c` → components
- `g` then `l` → logs
- `g` then `s` → security

Implement via `react-hotkeys-hook` (new dependency, lightweight) or plain keyboard event listener.

---

## Command Palette

Opened with `Cmd/Ctrl + K`. Modal dialog with:
- Search input at top
- Results below, fuzzy matched
- Keyboard nav (up/down arrows, enter to select)

Indexable content:
- All components (by name, id)
- All commands (per component)
- All configs (per component)
- All workflows
- All route paths

Selecting a result navigates to the corresponding screen.

---

## Mode Indicator

Persistent top bar indicator: "Advanced Mode" badge with a subtle background + border.

Click the badge → opens a small tooltip explaining what mode they're in, shortcut to toggle, link to `/preferences` (user mode).

---

## Color & Density

Dev mode deliberately looks different from user mode to signal "you are in a different context."

- Background: `bg-app` (darkest variant)
- Surfaces: `bg-surface`
- Smaller type (base → sm)
- Tighter spacing (space-6 → space-3)
- Sharp corners (radius-lg → radius-md)
- Flatter cards (shadow-md → shadow-xs)
- Monospace for code/IDs/paths
- High-contrast text (`text-primary` on `bg-app`)

---

## State Persistence

- Current route remembered on navigation
- Sidebar collapse state (future)
- Command palette recent searches (localStorage)
- Logs viewer filters (per component)

---

## Accessibility

- h1 on every screen
- Landmark regions: `<nav>`, `<main>`, `<header>`
- Keyboard shortcuts documented in a `?` help dialog
- Sidebar is a `<nav>` with role="navigation"

---

## Files to Change / Create

| Action | File | Notes |
|--------|------|-------|
| Create | `app/src/pages/dev/DevOverview.tsx` | Landing screen |
| Create | `app/src/components/dev/DevLayout.tsx` | Layout shell with sidebar + top bar |
| Create | `app/src/components/dev/DevSidebar.tsx` | Left nav |
| Create | `app/src/components/dev/DevTopBar.tsx` | Top bar with mode badge |
| Create | `app/src/components/dev/ComponentCardDev.tsx` | Compact card w/ metrics |
| Create | `app/src/components/dev/CommandPalette.tsx` | Cmd+K modal |
| Create | `app/src/hooks/useComponentMetrics.ts` | Poll memory/CPU/uptime |
| Create | `app/src/hooks/useKeyboardShortcuts.ts` | Shortcuts setup |
| Modify | `app/src/App.tsx` | Add `/dev/*` routes |
| Install | `react-hotkeys-hook` | For shortcut handling |

---

## Acceptance Criteria

- [ ] Dev mode clearly visually distinct from user mode
- [ ] All routes in sidebar navigate correctly
- [ ] Command palette opens on Cmd/Ctrl+K and filters instantly
- [ ] Overview shows live component metrics updating every 10s
- [ ] Exit Advanced button returns to `/`
- [ ] Mode preference persists across restarts

---

## Next

Read `14-component-operations.md` for per-screen details.
