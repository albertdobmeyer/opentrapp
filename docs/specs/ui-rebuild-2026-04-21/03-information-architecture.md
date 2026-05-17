# Information Architecture

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`
**Purpose:** Define routing, navigation, mode toggle mechanics, system tray, and the complete sitemap for both modes.

---

## The Dual-Mode Concept

OpenTrApp has two first-class UI modes inside a single Tauri app:

| Mode | Audience | Default? | Routes |
|------|----------|----------|--------|
| **User Mode** | Karen (non-technical) | Yes | `/`, `/security`, `/preferences`, `/help`, `/discover` |
| **Developer Mode** | Power users | No | `/dev/*` (own subtree) |

The mode is a **single boolean setting** (`mode: 'user' | 'developer'`) stored in Tauri store. The app reads it on startup and renders the appropriate route tree.

**Users do not see "mode" as a concept.** Karen never hears the word "developer mode" during normal use. Developers discover it via the Settings toggle or keyboard shortcut.

---

## Sitemap

### User Mode

```
/                    Home Dashboard
├── /security        Security Monitor (activity + alerts)
├── /discover        Use-Case Gallery (what can my assistant do?)
├── /preferences     Preferences (settings, minimal)
└── /help            Help & Support (FAQ, contact)
```

**Sidebar navigation (always visible in user mode):**

1. 🏠 Home
2. 🛡️ Security
3. 🔍 Discover
4. ⚙️ Preferences
5. 💬 Help

Each item is an icon + label. Labels use **friendly single words**. No sub-menus. No collapsibles.

### Developer Mode

```
/dev                            Developer Dashboard Overview
├── /dev/components             Component grid (vault, forge, pioneer)
├── /dev/components/:id         Component detail (commands, configs, workflows, logs)
├── /dev/logs                   Unified logs viewer (cross-component stream)
├── /dev/manifests              Manifest inspector (read/validate)
├── /dev/security               Security audit (24-point check runner)
├── /dev/allowlist              Proxy allowlist editor
├── /dev/shell-levels           Shell level configuration
└── /dev/preferences            Full preferences (technical settings)
```

**Sidebar navigation in dev mode:** different, denser layout.

1. **SYSTEM**
   - Overview
   - Logs
2. **COMPONENTS**
   - openclaw-vault
   - clawhub-forge
   - moltbook-pioneer
3. **SECURITY**
   - Audit
   - Allowlist
   - Shell levels
4. **INSPECTION**
   - Manifests
5. **⚙ Settings**

---

## Mode Switch UX

### Entry points to toggle

1. **Settings → Preferences → "Advanced Mode" section** (bottom of preferences page)
   - Friendly explanation: "Advanced Mode unlocks detailed views for developers, security researchers, and power users. Most people won't need this."
   - Toggle: "Enable Advanced Mode"
   - Immediate effect: route changes to `/dev` on toggle-on; back to `/` on toggle-off

2. **Keyboard shortcut**: `Cmd/Ctrl + Shift + D`
   - Works from any screen
   - Toggles the mode directly
   - Persisted to store

3. **System tray menu**: "Open Advanced Mode" item (dev-savvy users can pin this)

### Entry point to exit dev mode (from dev UI)

- Always-visible "Advanced Mode" badge in the dev mode top bar with an "Exit" button. Returns to user mode home.

### First-time transition

When a user first enables Advanced Mode, show a one-time **welcome dialog**:

> **Welcome to Advanced Mode**
>
> You're now seeing OpenTrApp's full technical controls. This view shows you every component, log, configuration, and security check.
>
> **Things to know:**
> - Changes here can break your setup. Be careful.
> - You can return to the friendly view anytime with Cmd+Shift+D.
> - This mode is hidden by default for a reason — you probably don't need it.
>
> [Got it, let me explore] [Actually, go back]

Persist a `hasSeenAdvancedModeIntro: true` flag in store. Show dialog only once.

---

## Routing Implementation

### Current state

`app/src/App.tsx` uses `react-router-dom` with a top-level `<Routes>`. A `<Layout>` wraps most routes; `/setup` is outside the layout.

### Target state

```tsx
// app/src/App.tsx
function App() {
  const { settings } = useAppContext();
  const mode = settings.mode ?? 'user';

  return (
    <ToastProvider>
      <ErrorBoundary>
        <Router>
          <Routes>
            {/* Setup wizard — no layout, outside modes */}
            <Route path="/setup" element={<Setup />} />

            {/* Dev mode routes */}
            {mode === 'developer' && (
              <Route path="/dev" element={<DevLayout />}>
                <Route index element={<DevOverview />} />
                <Route path="components" element={<DevComponentsList />} />
                <Route path="components/:id" element={<DevComponentDetail />} />
                <Route path="logs" element={<DevLogs />} />
                <Route path="manifests" element={<DevManifests />} />
                <Route path="security" element={<DevSecurity />} />
                <Route path="allowlist" element={<DevAllowlist />} />
                <Route path="shell-levels" element={<DevShellLevels />} />
                <Route path="preferences" element={<DevPreferences />} />
              </Route>
            )}

            {/* User mode routes */}
            <Route element={<UserLayout />}>
              <Route index element={<Home />} />
              <Route path="security" element={<SecurityMonitor />} />
              <Route path="discover" element={<UseCaseGallery />} />
              <Route path="preferences" element={<Preferences />} />
              <Route path="help" element={<HelpSupport />} />
            </Route>

            {/* Fallback */}
            <Route path="*" element={<NotFound />} />
          </Routes>
        </Router>
      </ErrorBoundary>
    </ToastProvider>
  );
}
```

**Important:** Mode affects which routes are registered. If the user navigates to `/dev/*` while in user mode, they hit NotFound. This is intentional — `/dev` is a "secret" subtree.

### Route redirects

- `/` → if `mode === 'developer'`, redirect to `/dev`
- `/dev` → if `mode === 'user'`, redirect to `/`

Alternative: render both trees always, but guard the dev routes with a `<ProtectedRoute>` component that checks mode. Use this approach if the mode switch needs to be instant without a rerender flash.

### Toggle side effect

When user toggles mode:
1. Update store
2. Navigate to the appropriate default route (`/` or `/dev`)
3. If entering dev mode for the first time, show welcome dialog

---

## Layouts

### `<UserLayout>` — User Mode Shell

```
┌────────────────────────────────────────────────────────────┐
│ ┌──────────┐ ┌────────────────────────────────────────────┐│
│ │          │ │                                            ││
│ │ Sidebar  │ │   Main content area                        ││
│ │  (80px)  │ │   (max-w-6xl, centered, p-8)               ││
│ │   icon   │ │                                            ││
│ │   only   │ │                                            ││
│ │          │ │                                            ││
│ └──────────┘ └────────────────────────────────────────────┘│
└────────────────────────────────────────────────────────────┘
```

**Sidebar width:** 80px (icon-only with labels below, 64x64 touch targets)
**Main area:** centered, max-w-6xl, generous padding
**Top bar:** none — just a logo in the sidebar and system tray integration
**Color:** `bg-base` background, `bg-surface` sidebar

### `<DevLayout>` — Developer Mode Shell

```
┌──────────────────────────────────────────────────────────────┐
│  [OpenTrApp · Advanced Mode]    [Exit Advanced] [⌘⇧D]  │ ← top bar (40px)
├───────────────┬──────────────────────────────────────────────┤
│               │                                              │
│   Sidebar     │   Main content                               │
│   (240px)     │   (full width, p-4)                          │
│   tree nav    │                                              │
│               │                                              │
│   dense       │   dense tables, panels, logs                 │
│   items       │                                              │
│               │                                              │
└───────────────┴──────────────────────────────────────────────┘
```

**Sidebar width:** 240px (tree nav with category headers)
**Main area:** full width, p-4, no max-width
**Top bar:** always visible, mode indicator + exit
**Color:** `bg-app` background (darker), `bg-surface` panels, high-contrast text

---

## System Tray Integration

### Implementation

Add to `app/src-tauri/tauri.conf.json`:

```json
{
  "bundle": { ... },
  "app": {
    "windows": [...],
    "trayIcon": {
      "iconPath": "icons/tray-icon.png",
      "menuOnLeftClick": false,
      "title": "OpenTrApp"
    }
  }
}
```

Add Rust tray handling in `app/src-tauri/src/lib.rs`.

### Tray menu (user mode)

```
🟢 Assistant running safely         ← status line (non-clickable)
─────────────────────────────────
📊 Open Dashboard                   → opens main window to /
🛡️  Security Check                  → runs security workflow
⏸️  Pause Assistant                 → stops the vault container
─────────────────────────────────
⚙️  Preferences
💬 Help
─────────────────────────────────
🔧 Advanced Mode                   → toggles + opens /dev
─────────────────────────────────
Quit OpenTrApp
```

### Tray status indicator

Tray icon shows the current state:

- **Green dot**: Assistant running safely
- **Amber dot**: Warning (e.g. skill pending scan, API rate limit approaching)
- **Red dot**: Error (container crashed, security audit failed)
- **Gray dot**: Paused / offline

Tooltip on hover: "Assistant: running safely" etc.

### Click behavior

- Single left click: open main window to Home
- Right click: show menu
- Status line at top of menu is always the current overall state

---

## Window Behavior

### Launch

- First launch: open the main window (setup wizard)
- Subsequent launches: do not open window; start in system tray
- User opens window via: tray icon, OS shortcut, or file association (optional)

### Close button

- **Default**: hide window to tray (doesn't quit the app)
- **With `Cmd+Q` / explicit Quit**: stop all containers, quit app

### Minimize behavior

- Minimize: hide to tray after 5 seconds of inactivity (optional, preference-gated)
- User preference: "Keep window open when minimized" (default OFF)

---

## Notifications

Using `@tauri-apps/plugin-notification` (to be installed).

### User mode notifications

Send system notifications for:
- **First-run setup complete**: "Your assistant is ready! Say hi on Telegram."
- **Security alerts**: "Your assistant tried to visit a blocked site." (user-configurable)
- **Monthly spending approaching limit**: "You've used 80% of your monthly spending limit."
- **Update available**: "A new version is ready."
- **Assistant paused unexpectedly**: "Something isn't working — open OpenTrApp to check."

### Dev mode notifications

Send for:
- Container crash (automatic)
- Long-running workflow completion
- Security audit failure
- Failed command streams

### Preferences

Preferences screen has per-category notification toggles. Default: all ON for user mode; NONE for dev mode (developers prefer in-app surfacing).

---

## Keyboard Shortcuts

### User mode (minimal, discoverable)

- `Cmd/Ctrl + ,` → Open Preferences
- `Cmd/Ctrl + /` → Open Help
- `Cmd/Ctrl + Shift + D` → Toggle Advanced Mode (hidden)
- `Esc` → Close modals

### Dev mode (keyboard-first)

- `Cmd/Ctrl + K` → Command palette (search components, commands, configs)
- `Cmd/Ctrl + L` → Jump to logs
- `Cmd/Ctrl + 1..9` → Jump to components by index
- `Cmd/Ctrl + Shift + R` → Refresh all component statuses
- `Cmd/Ctrl + Shift + D` → Exit Advanced Mode
- `g` then `c` → Go to components (Vim-style)
- `g` then `l` → Go to logs

Show a "Keyboard Shortcuts" help dialog with `?` key.

---

## State Management

### Tauri Store

Extend `app/src/lib/settings.ts`:

```ts
export interface AppSettings {
  // Existing
  monorepoPathOverride: string | null;
  autoRefreshInterval: number;
  wizardCompleted: boolean;
  lastViewedComponentId: string | null;

  // NEW
  mode: 'user' | 'developer';
  hasSeenAdvancedModeIntro: boolean;
  autostart: boolean;
  notifications: {
    securityAlerts: boolean;
    spendingLimit: boolean;
    updates: boolean;
  };
  spendingLimit: {
    monthly: number | null;  // cents
    alertThreshold: number;  // fraction (0.8 = alert at 80%)
  };
  theme: 'dark';  // placeholder for future light mode
  minimizeToTray: boolean;
  closeToTray: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  monorepoPathOverride: null,
  autoRefreshInterval: 10000,
  wizardCompleted: false,
  lastViewedComponentId: null,
  mode: 'user',
  hasSeenAdvancedModeIntro: false,
  autostart: true,
  notifications: {
    securityAlerts: true,
    spendingLimit: true,
    updates: true,
  },
  spendingLimit: {
    monthly: 2000,  // $20
    alertThreshold: 0.8,
  },
  theme: 'dark',
  minimizeToTray: false,
  closeToTray: true,
};
```

### AppContext

Expand `AppContext` to expose:
- `mode`, `setMode(mode: 'user' | 'developer')`
- `showAdvancedModeIntro` (derived from `hasSeenAdvancedModeIntro`)

---

## Sitemap Diagram (Final)

```
Tauri App
│
├── System Tray (always, in OS)
│   ├── Status indicator (green/amber/red/gray)
│   └── Context menu (Open / Security / Pause / Preferences / Advanced / Quit)
│
└── Main Window
    │
    ├── /setup  (first-run wizard)
    │
    ├── User Mode (default)
    │   ├── /                → Home Dashboard
    │   ├── /security        → Security Monitor
    │   ├── /discover        → Use-Case Gallery
    │   ├── /preferences     → Preferences
    │   └── /help            → Help & Support
    │
    └── Developer Mode (toggle)
        ├── /dev             → Overview
        ├── /dev/components  → Component list
        │   └── /:id         → Component detail
        ├── /dev/logs        → Logs viewer
        ├── /dev/manifests   → Manifest inspector
        ├── /dev/security    → Security audit
        ├── /dev/allowlist   → Allowlist editor
        ├── /dev/shell-levels → Shell configuration
        └── /dev/preferences → Full preferences
```

---

## Next

Read `04-visual-assets-plan.md` to see what visuals this IA needs.
