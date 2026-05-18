# User Mode: Home Dashboard

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`, `03-information-architecture.md`
**Screen:** `/` (user mode root)
**Rubric target:** 9+/10 across all principles

---

## Purpose

Answer the question Karen has when she opens the app:

> **"Is my assistant running safely, and is there anything I need to do?"**

This is the only screen she looks at 90% of the time. Everything else serves this single purpose.

The home dashboard is **not** a "see all components" view. It's a **state-of-your-assistant** view.

---

## User Story

> As Karen, when I open OpenTrApp, I want to see at a glance whether everything is OK or if I need to do something. If it's OK, I close the window and get back to my day. If it's not, I want one button that fixes it — or a clear way to get help.

---

## Layout

```
┌───────────────────────────────────────────────────────────────┐
│                                                               │
│  []                                                          │ ← sidebar (always visible)
│                                                               │
│  🏠                                                            │
│  🛡️                                                            │
│  🔍                                                            │
│  ⚙️                                                            │
│  💬                                                            │
│                                                               │
└───────────────────────────────────────────────────────────────┘

        │
        │ main content area
        ▼

┌───────────────────────────────────────────────────────────────┐
│                                                               │
│   ┌───────────────────────────────────────────────────────┐  │
│   │                                                       │  │
│   │   [status-safe.svg 160×160]                           │  │
│   │                                                       │  │
│   │   Your assistant is running safely                    │  │
│   │                                                       │  │
│   │   It's been active for 2 hours today.                 │  │
│   │                                                       │  │
│   │   [ Pause ]     [ Open Telegram → ]                   │  │
│   │                                                       │  │
│   └───────────────────────────────────────────────────────┘  │
│                                                               │
│   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│   │ 🛡️ Security│ │ 📊 Activity│ │ 💰 Spending │           │
│   │             │ │             │ │             │           │
│   │    Safe     │ │  12 tasks   │ │  $3.20 /    │           │
│   │             │ │  today      │ │  $20 month  │           │
│   │             │ │             │ │   ▰▰▰░░░░░░ │           │
│   └─────────────┘ └─────────────┘ └─────────────┘           │
│                                                               │
│   ┌───────────────────────────────────────────────────────┐  │
│   │ 💡 Tip of the day                                     │  │
│   │                                                       │  │
│   │ Did you know? You can ask your assistant to           │  │
│   │ "summarize the news from today."                      │  │
│   │                                                       │  │
│   │ [ Try this →  ]   Explore more ideas →                │  │
│   └───────────────────────────────────────────────────────┘  │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

---

## Sections (Top to Bottom)

### 1. Hero Status Card

**The most important element on the screen.** Large, central, unmissable.

**Contents:**
- Illustration (`status-{state}.svg`) — 160×160
- Headline: 3xl semibold, state-dependent
- Sub-line: secondary text, state-dependent
- Action buttons (usually 2): state-dependent

**State variants:**

#### Running safely (green)
```
[status-safe.svg]
Your assistant is running safely
It's been active for 2 hours today.
[ Pause ]  [ Open Telegram → ]
```

#### Paused by user (gray)
```
[status-paused.svg]
Your assistant is paused
You paused it 15 minutes ago.
[ Resume ]  [ Open Telegram → ]
```

#### Something needs attention (amber)
```
[status-warning.svg]
Something needs attention
Your spending is approaching its limit.
[ View details ]  [ Adjust limit ]
```

#### Not running — error (red)
```
[status-error.svg]
Your assistant isn't running
We're having trouble. Let's try to fix it.
[ Try to fix ]  [ Get help ]
```

#### Not set up (first time)
```
[status-offline.svg]
Your assistant isn't set up yet
Let's get you started.
[ Run setup ]
```

#### API key invalid
```
[status-warning.svg]
Your AI key needs updating
Your key isn't working anymore. Open preferences to update.
[ Open preferences ]
```

The card **must** always show an illustration matching state and a clear action (or two). No ambiguity.

### 2. Three Stat Tiles

**Always visible below the hero.** Small, informative, glanceable.

#### 🛡️ Security Tile
- Title: "Security"
- Big value: "Safe" | "Needs attention" | "Unknown"
- Sub-line: "Last checked 2 min ago"
- Click: → `/security` (Security Monitor screen)

#### 📊 Activity Tile
- Title: "Activity"
- Big value: "12 tasks today" | "Quiet today" | "None yet"
- Sub-line: "Most recent: planned a trip"
- Click: → `/security` (activity timeline is part of security monitor)

#### 💰 Spending Tile
- Title: "Spending"
- Big value: "$3.20 / $20 this month"
- Progress bar (16% filled)
- Sub-line: "~$0.11/day on average"
- Click: → `/preferences` (to see/change limit)

If any tile has a concerning state (security warning, over-budget), the tile shows a colored accent border (amber/red).

### 3. Tip of the Day (Optional Polish)

A rotating tip from the use-case gallery.

- Deterministic pick (day-of-year % pool)
- "Try this" opens Telegram deep-link with prefilled message
- "Explore more ideas" → `/discover` (use-case gallery)

If the user is in an alert state (security issue, spending over), the Tip card is replaced by an **action card** with clear CTAs.

### 4. Proactive Alerts Banner (Conditional)

If any condition requires attention, show a banner ABOVE the hero:

```
┌────────────────────────────────────────────────────────┐
│ ⚠️  Your spending is at 85% of your monthly limit.     │
│                                                        │
│   [ View usage ]  [ Adjust limit ]   Dismiss           │
└────────────────────────────────────────────────────────┘
```

Banner variants:
- **Warning (amber):** spending, pending skill scan
- **Danger (red):** container crashed, API key invalid, security audit failed
- **Info (blue):** update available, new skill released

Multiple alerts stack. User can dismiss each (persists dismissal for current session).

---

## Data Sources

### Frontend reads from Tauri commands:

- `get_assistant_status() → 'running' | 'paused' | 'error' | 'not_setup' | 'offline'`
- `get_security_status() → { state: 'safe' | 'warning' | 'error', lastCheckedAt: number }`
- `get_activity_summary() → { tasksToday: number, lastTask?: string, lastTaskAt?: number }`
- `get_spending_summary() → { monthCents: number, monthLimitCents: number | null, dayAvgCents: number }`
- `get_active_alerts() → Alert[]` (from backend state)

### Backend additions needed:

- **Activity tracking**: persist agent task events to Tauri store. Backend emits on each Telegram message processed, each skill run. Schema defined in `09-security-monitor.md`.
- **Spending tracking**: vault-proxy logs each Anthropic API call. Calculate cost via `tokens * price_per_token` per model. Aggregate daily/monthly. Schema:
  ```
  spending/{YYYY-MM}/{DD}: { tokens: int, cost_cents: int, calls: int }
  ```
  A Rust command `get_spending_summary()` reads these entries.
- **Assistant status aggregation**: combines container health, API key validity, proxy readiness into a single top-level state.
- **Alerts subsystem**: evaluates conditions on a schedule (every 60s) and stores active alerts. Frontend polls via `get_active_alerts()`.

---

## Auto-Refresh

- Status polled every 10 seconds (from `autoRefreshInterval` preference)
- On Tauri event `status-changed`, refresh immediately
- When app returns from background, refresh immediately

---

## Loading States

### First open (data not yet loaded)

```
┌─────────────────────────────────────────┐
│  [skeleton hero card — pulse animation] │
├─────────────────────────────────────────┤
│  [skeleton] [skeleton] [skeleton]       │
└─────────────────────────────────────────┘
```

Use the `<SkeletonCard>` / `<SkeletonText>` components (expand from existing).

### Subsequent refreshes

Silent — update in place without loading spinners.

---

## Error States

If data fetch fails:

- Hero: fall back to "not_setup" state with "Couldn't check assistant status" text
- Tiles: each tile shows "—" with "Check again" tooltip
- If ALL data fetches fail: show Level 2 friendly retry screen

---

## Empty States

### No activity yet
```
📊 Activity
None yet
Your assistant has no tasks today.
```

### No spending data
```
💰 Spending
$0.00 this month
You haven't spent anything yet.
```

---

## Actions (Primary CTAs)

The hero card's primary actions are context-dependent:

| State | Primary action | Secondary action |
|-------|----------------|-----------------|
| Running safely | Pause | Open Telegram |
| Paused | Resume | Open Telegram |
| Error | Try to fix (auto-retry) | Get help |
| Not set up | Run setup | — |
| API key issue | Open preferences | — |
| Updating | — | — (buttons disabled) |

Secondary actions throughout the screen (stat tiles, tip card, alerts) are already defined above.

---

## Copy Bank

```json
{
  "hero.running.title": "Your assistant is running safely",
  "hero.running.subline": "It's been active for {duration} today.",
  "hero.running.pauseBtn": "Pause",
  "hero.running.openTelegramBtn": "Open Telegram",

  "hero.paused.title": "Your assistant is paused",
  "hero.paused.subline": "You paused it {relativeTime} ago.",
  "hero.paused.resumeBtn": "Resume",

  "hero.error.title": "Your assistant isn't running",
  "hero.error.subline": "We're having trouble. Let's try to fix it.",
  "hero.error.fixBtn": "Try to fix",
  "hero.error.helpBtn": "Get help",

  "hero.notSetup.title": "Your assistant isn't set up yet",
  "hero.notSetup.subline": "Let's get you started.",
  "hero.notSetup.setupBtn": "Run setup",

  "hero.warning.spendingApproaching.title": "Something needs attention",
  "hero.warning.spendingApproaching.subline": "Your spending is approaching its limit.",
  "hero.warning.apiKeyInvalid.title": "Your AI key needs updating",
  "hero.warning.apiKeyInvalid.subline": "Your key isn't working anymore. Open preferences to update.",

  "tile.security.title": "Security",
  "tile.security.safe": "Safe",
  "tile.security.warning": "Needs attention",
  "tile.security.unknown": "Unknown",
  "tile.security.lastChecked": "Last checked {relativeTime} ago",

  "tile.activity.title": "Activity",
  "tile.activity.count": "{count, plural, one {# task} other {# tasks}} today",
  "tile.activity.quiet": "Quiet today",
  "tile.activity.none": "None yet",
  "tile.activity.lastTask": "Most recent: {description}",

  "tile.spending.title": "Spending",
  "tile.spending.value": "${month} / ${limit} this month",
  "tile.spending.noLimit": "${month} this month",
  "tile.spending.daily": "~${dayAvg}/day on average",

  "tip.title": "Tip of the day",
  "tip.tryBtn": "Try this",
  "tip.exploreBtn": "Explore more ideas"
}
```

---

## Visual Elements

- Hero uses `status-{state}.svg` illustrations
- Stat tiles use Heroicons (shield, chart-bar, currency-dollar)
- Progress bar in spending tile uses bg-success for normal, bg-warning for 80%+, bg-danger for 100%+
- Rounded-xl (1rem) corners on all cards
- Hover on tiles: subtle border color change, cursor pointer
- Enter animations: slide-up + fade-in staggered (hero → tiles → tip)

---

## Accessibility

- h1 = hero title
- Tiles are `<button>` or `<Link>` with descriptive labels ("Security — Safe. Click to view details.")
- Progress bar has `role="progressbar"` with `aria-valuenow`, `aria-valuemin`, `aria-valuemax`
- Live region for status updates (`aria-live="polite"`): announces state changes
- Keyboard: tab order is hero buttons → tile 1 → tile 2 → tile 3 → tip CTA

---

## Acceptance Criteria

- [ ] Hero visible within 200ms of screen load (after data fetch)
- [ ] All state variants covered and visually distinct
- [ ] Stat tiles update in real-time when state changes
- [ ] Proactive alerts appear within 10s of backend state change
- [ ] Zero developer terminology visible
- [ ] Rubric score ≥ 9/10 on all applicable principles
- [ ] Keyboard navigable (tab order correct)

---

## Files to Change / Create

| Action | File | Notes |
|--------|------|-------|
| Replace | `app/src/pages/Dashboard.tsx` | Rewrite as Home dashboard per this spec |
| Create | `app/src/components/user/HeroStatusCard.tsx` | The big central status card |
| Create | `app/src/components/user/StatTile.tsx` | Reusable tile component |
| Create | `app/src/components/user/TipOfTheDay.tsx` | Rotating tip card |
| Create | `app/src/components/user/ProactiveAlertsBanner.tsx` | Top alerts |
| Create | `app/src/hooks/useAssistantStatus.ts` | Polls status |
| Create | `app/src/hooks/useSpendingSummary.ts` | Polls spending |
| Create | `app/src/hooks/useActivitySummary.ts` | Polls activity |
| Create | `app/src/hooks/useAlerts.ts` | Polls alerts |
| Create | `app/src-tauri/src/commands/assistant_status.rs` | Aggregate status |
| Create | `app/src-tauri/src/commands/spending.rs` | Calculate from proxy logs |
| Create | `app/src-tauri/src/commands/activity.rs` | Read persisted events |
| Create | `app/src-tauri/src/commands/alerts.rs` | Evaluate conditions |

---

## Next

Read `09-security-monitor.md` — where Karen goes when she wants to know what her assistant has been doing.
