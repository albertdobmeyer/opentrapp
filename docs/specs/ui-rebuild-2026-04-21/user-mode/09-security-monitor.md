# User Mode: Security Monitor

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`
**Screen:** `/security`
**Rubric target:** 9+/10 across all principles

---

## Purpose

Answer: **"What has my assistant been doing, and is anything concerning?"**

This screen builds trust. Karen wants to know the AI hasn't gone rogue. She doesn't understand terms like "blocked HTTPS request" but she does understand "your assistant tried to visit a blocked website."

---

## User Story

> As Karen, I want to look at what my assistant has been doing today — what it read, what it did, what websites it tried to visit — and I want to feel confident that anything suspicious was stopped.

---

## Layout

```
┌──────────────────────────────────────────────────────────────┐
│  Security & Activity                                         │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 🛡️  Your assistant is safe                             │ │
│  │                                                        │ │
│  │ Last safety check: 2 minutes ago                       │ │
│  │ All 24 safety checks passed.                           │ │
│  │                                                        │ │
│  │ [ Check now ]                                          │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Recent activity                          [Today ▼]     │ │
│  │                                                        │ │
│  │ 3:15 PM  📬  Received a message from you              │ │
│  │ 3:15 PM  💭  Thought about "plan my Tuesday"          │ │
│  │ 3:16 PM  🌐  Checked weather.com                      │ │
│  │ 3:16 PM  📝  Wrote a response                         │ │
│  │ 3:16 PM  📬  Sent you a reply                         │ │
│  │                                                        │ │
│  │ 1:02 PM  📬  Received a message from you              │ │
│  │ 1:02 PM  💭  Thought about "summarize the news"       │ │
│  │ 1:03 PM  🌐  Checked nytimes.com, bbc.co.uk, +3 more   │ │
│  │ 1:04 PM  📝  Wrote a response                         │ │
│  │ 1:04 PM  📬  Sent you a reply                         │ │
│  │                                                        │ │
│  │ [ Show older ]                                         │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 🛡️ Safety incidents (last 30 days)                    │ │
│  │                                                        │ │
│  │ None. Nothing suspicious happened.                     │ │
│  │                                                        │ │
│  │ Your assistant is only allowed to visit websites you   │ │
│  │ trust. Here's what's on that list:                     │ │
│  │                                                        │ │
│  │ [ wikipedia.org ] [ anthropic.com ] [ +12 more ]       │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 📦 Installed skills                          25 total  │ │
│  │                                                        │ │
│  │ ✓  All clean — all scanned for safety                  │ │
│  │                                                        │ │
│  │ [ View skills ]                                        │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Sections

### 1. Safety Summary Card (top)

**Purpose:** Answer "am I safe right now?" at a glance.

**States:**

#### All safe (green)
- 🛡️ Your assistant is safe
- "Last safety check: 2 minutes ago / All 24 safety checks passed."
- Green accent border

#### Some concern (amber)
- 🛡️ Something needs attention
- "Last safety check found 1 issue — let's take a look."
- Amber accent, "View details" CTA

#### Not checked recently (gray)
- 🛡️ Checking safety…
- "Running your monthly safety scan."
- Shows spinner
- Auto-dismisses when done

**Actions:** "Check now" triggers a manual safety audit. Shows inline progress and updates in place.

### 2. Recent Activity Timeline

**Purpose:** Show what the assistant has been doing, translated for Karen.

**Entry types (user-facing icons + labels):**

| Internal event | User icon | User label |
|----------------|-----------|------------|
| telegram.message.received | 📬 | Received a message from you |
| telegram.message.sent | 📬 | Sent you a reply |
| agent.reasoning.start | 💭 | Thought about "{summary}" |
| agent.tool.web_fetch | 🌐 | Checked {domain} |
| agent.tool.web_fetch (multiple) | 🌐 | Checked {domain}, {domain}, +{n} more |
| agent.tool.file_write | 📝 | Wrote a response |
| agent.tool.file_read | 📖 | Read a file |
| agent.tool.bash | ⚙️ | Ran a task |
| skill.installed | 📦 | Installed a new skill |
| skill.executed | 🧩 | Used skill "{name}" |

**Timestamp format:**
- Today: "3:15 PM"
- Yesterday: "Yesterday 3:15 PM"
- Earlier this week: "Monday 3:15 PM"
- Older: "April 19, 3:15 PM"

**Grouping:** Activity is grouped by conversation (each "Received message" starts a new group). Visual indent to show causality.

**Filter:** Date range dropdown — Today / This week / This month / Custom.

**Pagination:** "Show older" button loads next 50 events.

**Empty state:**
```
[empty-activity.svg 200×200]

Nothing yet today
When your assistant helps you out,
you'll see what it did here.
```

### 3. Safety Incidents

**Purpose:** Show what was blocked, what was scanned, what tried to break out.

**Normal state (the 99%):**
```
🛡️ Safety incidents (last 30 days)

None. Nothing suspicious happened.

Your assistant is only allowed to visit websites you
trust. Here's what's on that list:

[ wikipedia.org ] [ anthropic.com ] [ +12 more ]
```

**Incident state (when blocks happened):**
```
🛡️ Safety incidents (last 30 days)

3 blocks in the last 30 days.

2 days ago  🚫  Blocked a visit to malicious-ad.net
            The website wasn't on your trusted list.
            [ Dismiss ] [ Add to trusted ]

5 days ago  🚫  Blocked a shell command
            A skill tried to run a command outside its sandbox.
            [ View details ]

...

[ Show all incidents ]
```

**Incident categories:**
- Network block: non-allowlisted domain attempt
- Skill block: sandboxed command escape attempt
- File block: access to user files outside sandbox
- Permission block: capability-requiring action

**User affordances:**
- Each incident shows time, what happened in plain English, why it matters
- "Dismiss" = acknowledge, stops showing as recent
- "Add to trusted" (network blocks only) = opens a confirmation dialog, on confirm adds domain to allowlist
- "View details" = expands to show more context (still friendly)

### 4. Allowlist Summary

**Purpose:** Reassure that the assistant is bounded.

**Display:**
- Count and first 2–3 domains as chips
- "+12 more" opens a full list modal
- Each chip has a `×` to remove (with confirmation)
- "+ Add trusted site" button (opens input modal)

### 5. Installed Skills Summary

**Purpose:** Show that all skills are safe.

**Display:**
- Count of installed skills
- "All clean — all scanned for safety" badge
- CTA "View skills" → opens a skills detail screen (future, not in v0.2.0)

If any skill failed its scan:
- 🚫 "1 skill blocked from installing: malicious-skill. [View details]"

---

## Data Model (New — to be implemented)

### Activity event schema

Stored in Tauri store under `activity_log:{YYYY-MM-DD}` keys.

```ts
interface ActivityEvent {
  id: string;                      // UUID
  timestamp: number;               // epoch ms
  category: 'telegram' | 'agent' | 'security' | 'skill' | 'system';
  type: string;                    // specific event type
  summary: string;                 // short human-readable summary
  details?: Record<string, unknown>; // optional, for "view details"
  domain?: string;                 // for web fetch events
  skillName?: string;              // for skill events
}
```

### Incident event schema

Stored under `incidents:{YYYY-MM}` keys.

```ts
interface Incident {
  id: string;
  timestamp: number;
  severity: 'info' | 'warning' | 'danger';
  type: 'network_block' | 'skill_block' | 'file_block' | 'permission_block';
  userMessage: string;             // plain English
  technicalMessage?: string;       // shown in dev mode
  relatedDomain?: string;
  dismissed?: boolean;
}
```

### Backend events

The backend needs to emit new Tauri events:

- `activity:appended` — new activity row
- `incidents:appended` — new incident

Frontend listens and updates timeline live.

---

## Data Sources

### Frontend reads:

- `get_activity_timeline(dateRange): ActivityEvent[]`
- `get_incidents(dateRange): Incident[]`
- `get_allowlist(): string[]`
- `get_skills_summary(): { count: number, allClean: boolean }`
- `run_safety_audit(): SafetyAuditResult` (triggered by "Check now")

### Backend changes required:

1. **Activity tracking system** — new Rust module that:
   - Subscribes to vault-proxy request logs
   - Parses telegram events, web fetches
   - Persists to Tauri store
   - Emits Tauri events on new activity

2. **Incident tracking** — vault-proxy logs blocked requests; new Rust module:
   - Parses proxy block logs
   - Persists to Tauri store
   - Emits Tauri events

3. **Allowlist CRUD API** — read/write `components/opencli-container/proxy/allowlist.txt` via commands.

4. **Skills scan state** — query openskill-forge for scan results.

---

## Auto-Refresh

- Poll `get_activity_timeline(today)` every 30 seconds
- Listen to `activity:appended` Tauri event for live updates
- Listen to `incidents:appended` for live updates
- Safety audit result refreshes on demand only

---

## Loading States

### First load

```
[skeleton: safety card]
[skeleton: activity timeline with 5 skeleton rows]
[skeleton: incidents card]
```

### Running a manual safety check

Safety card shows inline spinner and "Checking safety…" text. Results update in place.

---

## Empty States

### Fresh install, no activity yet

Show empty state illustration (`empty-activity.svg`) with a "Try sending your first message" CTA linking to "Open Telegram."

### No incidents (the normal state)

Show positive empty state: "None. Nothing suspicious happened."

---

## Error States

- If activity fetch fails: show friendly retry for that section only
- If safety audit fails: show Level 2 retry screen for that action

---

## Copy Bank

```json
{
  "page.title": "Security & Activity",

  "safety.safe.title": "Your assistant is safe",
  "safety.safe.subline": "Last safety check: {relativeTime}\nAll {count} safety checks passed.",
  "safety.concern.title": "Something needs attention",
  "safety.concern.subline": "Last safety check found {count, plural, one {# issue} other {# issues}} — let's take a look.",
  "safety.checking.title": "Checking safety…",
  "safety.checking.subline": "Running your safety scan.",
  "safety.checkNowBtn": "Check now",

  "activity.title": "Recent activity",
  "activity.filter.today": "Today",
  "activity.filter.week": "This week",
  "activity.filter.month": "This month",
  "activity.filter.custom": "Custom range",
  "activity.empty.title": "Nothing yet today",
  "activity.empty.subline": "When your assistant helps you out, you'll see what it did here.",
  "activity.showOlder": "Show older",

  "activity.event.telegram.received": "Received a message from you",
  "activity.event.telegram.sent": "Sent you a reply",
  "activity.event.agent.reasoning": "Thought about \"{summary}\"",
  "activity.event.agent.webFetch.single": "Checked {domain}",
  "activity.event.agent.webFetch.multiple": "Checked {domains} and {count} more",
  "activity.event.agent.fileWrite": "Wrote a response",
  "activity.event.agent.fileRead": "Read a file",
  "activity.event.agent.bash": "Ran a task",
  "activity.event.skill.installed": "Installed a new skill",
  "activity.event.skill.executed": "Used skill \"{name}\"",

  "incidents.title": "Safety incidents (last 30 days)",
  "incidents.empty.title": "None. Nothing suspicious happened.",
  "incidents.empty.subline": "Your assistant is only allowed to visit websites you trust. Here's what's on that list:",
  "incidents.count": "{count, plural, one {# incident} other {# incidents}} in the last 30 days.",
  "incidents.showAll": "Show all incidents",

  "incident.networkBlock.title": "Blocked a visit to {domain}",
  "incident.networkBlock.subline": "The website wasn't on your trusted list.",
  "incident.networkBlock.dismiss": "Dismiss",
  "incident.networkBlock.addTrust": "Add to trusted",
  "incident.skillBlock.title": "Blocked a {action}",
  "incident.skillBlock.subline": "A skill tried to run a command outside its sandbox.",
  "incident.viewDetails": "View details",

  "allowlist.addBtn": "+ Add trusted site",
  "allowlist.moreBtn": "+{count} more",

  "skills.title": "Installed skills",
  "skills.countTotal": "{count} total",
  "skills.allClean": "All clean — all scanned for safety",
  "skills.someBlocked": "{count, plural, one {# skill} other {# skills}} blocked from installing",
  "skills.viewBtn": "View skills"
}
```

---

## Visual Elements

- Safety card uses colored accent border (green/amber/gray)
- Activity timeline: vertical timeline with connector line, icons in circles on the left
- Each activity group has a subtle background
- Incidents card: uses `empty-alerts.svg` illustration when empty
- Allowlist chips use `pill-neutral` style
- Skills card with ✓ or 🚫 badge

---

## Accessibility

- h1 = "Security & Activity"
- Each activity row has `role="article"` with descriptive text
- Safety check result announces via `aria-live`
- Incident actions (Dismiss, Add to trusted) are keyboard-accessible buttons

---

## Acceptance Criteria

- [ ] Activity timeline shows events within 3 seconds of them occurring in backend
- [ ] No raw event types visible (only friendly labels)
- [ ] Domain names are user-visible but **never** file paths, proxy URLs, or container IDs
- [ ] Incidents use plain English; technical details only in dev mode
- [ ] Empty states are positive and reassuring
- [ ] Rubric score ≥ 9/10

---

## Files to Change / Create

| Action | File | Notes |
|--------|------|-------|
| Create | `app/src/pages/user/SecurityMonitor.tsx` | New page |
| Create | `app/src/components/user/SafetyCard.tsx` | Safety summary |
| Create | `app/src/components/user/ActivityTimeline.tsx` | Event timeline |
| Create | `app/src/components/user/ActivityRow.tsx` | Single event row |
| Create | `app/src/components/user/IncidentsCard.tsx` | Incidents section |
| Create | `app/src/components/user/AllowlistChips.tsx` | Chip display |
| Create | `app/src/components/user/AllowlistModal.tsx` | Full list / add |
| Create | `app/src/hooks/useActivityTimeline.ts` | Timeline data + live updates |
| Create | `app/src/hooks/useIncidents.ts` | Incidents data + live updates |
| Create | `app/src/hooks/useAllowlist.ts` | Allowlist CRUD |
| Create | `app/src-tauri/src/commands/activity.rs` | Activity read/write |
| Create | `app/src-tauri/src/commands/incidents.rs` | Incidents read/dismiss |
| Create | `app/src-tauri/src/commands/allowlist.rs` | Allowlist CRUD |
| Create | `app/src-tauri/src/activity_tracker.rs` | Parses proxy logs → events |

---

## Next

Read `10-preferences.md` — Karen's settings.
