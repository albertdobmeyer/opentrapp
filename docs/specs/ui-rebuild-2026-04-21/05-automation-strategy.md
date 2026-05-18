# Automation Strategy

**Prerequisite reading:** `01-vision-and-personas.md`
**Purpose:** Define what the app auto-detects, auto-heals, and auto-configures so Karen makes as few decisions as possible. "As few clicks as possible, as much automated as possible."

---

## The Automation Test

Before asking the user any question, ask:

1. **Can we auto-detect this?** (e.g., find Podman via `which podman`)
2. **Can we assume a good default?** (e.g., refresh interval of 10 seconds)
3. **Can we defer the decision?** (e.g., ask later, when the user actually needs the feature)
4. **Does this decision require human judgment?** (e.g., API key — only the user has it)

**Only ask if the answer is "none of the above."**

Current setup asks 25+ times. Target: **3 questions** (API key, Telegram token, spending limit).

---

## Automation Categories

### 1. Auto-Detect

Find what already exists on the user's system without asking.

| Thing | How to detect | Fallback if not found |
|-------|---------------|----------------------|
| **Container runtime** (Podman/Docker) | `which podman` → `which docker` → query version | Guide user to install (with platform-specific links + one-click opener) |
| **Monorepo path** (where opentrapp lives) | Canonical parent of Tauri binary | Auto-create at `~/.opentrapp/` |
| **Submodule status** | Check git submodule summary | Auto-init via `git submodule update --init --recursive` |
| **Existing `.env` keys** | Read `components/opencli-container/.env` if present | Show empty form |
| **Telegram bot already paired** | Read `pairing-status.json` in vault workspace | Show connect button |
| **OS & architecture** | Tauri API `os.platform()` / `os.arch()` | N/A (always succeeds) |
| **Network connectivity** | Fetch test to a known endpoint | Show offline state |
| **Existing skills** | Scan forge's skill directory | Show empty state |

**Implementation note:** All auto-detects happen in parallel on app startup, showing a single "Checking your computer..." loader (~1-2 seconds total). Individual failures don't block startup; they surface later as specific issues.

### 2. Auto-Heal

Silently fix transient failures without user awareness.

#### Container startup retry

Most "Something went wrong" errors during setup are the vault-proxy timing race (documented in handoff). Handle automatically:

```ts
async function setupWithRetry(componentId: string, commandId: string): Promise<Result> {
  const MAX_RETRIES = 2;
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    const result = await runCommand(componentId, commandId);
    if (result.success) return result;

    if (attempt < MAX_RETRIES) {
      await sleep(2000 * (attempt + 1));  // exponential-ish backoff
      // User sees: still "Setting up..." — no indication of retry
      continue;
    }
    return result;  // final failure surfaces to UI
  }
}
```

Apply the same pattern to:
- Proxy readiness checks during setup
- Container start after system wake
- API key validation on first use

**Do not retry destructive operations** (resets, deletes).

#### Setup wizard resume

If the app crashes mid-setup, the wizard auto-detects where to resume:

- Welcome + Connect complete → resume at Installing
- Installing incomplete → resume at Installing, rebuild only failed components

Persist `setupProgress: { step: string, completedSteps: string[] }` in Tauri store. Check on app launch. If user already completed setup 5 times ago, don't re-resume; respect `wizardCompleted: true`.

#### Configuration validation

After setup:
- Periodically (every 24h) validate the Anthropic API key against the API (a cheap `models.list()` call via proxy)
- If invalid: show a tray alert, link to Preferences to update
- If valid: silent

#### Auto-restart on crash

If vault-agent container crashes:
- First attempt: auto-restart via compose, silent
- Second crash within 5 minutes: show alert "Your assistant stopped — we're trying to restart it"
- Third crash within 5 minutes: stop trying, open contact-support flow

---

### 3. Auto-Configure

Set sensible defaults without asking.

| Setting | Default | Rationale |
|---------|---------|-----------|
| Auto-start on boot | ON | Invisible security wrapper = always on |
| Close window → minimize to tray | ON | Background service behavior |
| Minimize window → stay in tray | ON | User doesn't want window clutter |
| Status refresh interval | 10 seconds | Balance responsiveness vs CPU |
| Security alerts | ON | Karen wants to know |
| Monthly spending limit | $20 | Safe default; user can raise |
| Spending alert threshold | 80% | Early warning |
| Update notifications | ON | Apple/Google expect updates |
| Notification sounds | Follows OS | Respect OS preference |

The setup wizard never asks about these. They appear in Preferences, pre-populated.

---

### 4. Auto-Prompt

Ask only when necessary. Minimize questions. Present each question with:

1. **Why it's needed** (1 sentence)
2. **Where to get it** (link + screenshots)
3. **What happens if I skip** (clear)

Questions we must ask:

| Question | Why | Skippable? |
|----------|-----|------------|
| Anthropic API key | The AI needs it | Yes, skip → assistant paused |
| Telegram bot token | That's your chat channel | Yes, skip → paired later |
| Spending limit | Safety default is $20 | Yes, use default |

Questions we must NOT ask (auto-detected or defaulted):

- Install location
- Container runtime (auto-detect)
- Proxy port
- Shell level
- Security profile
- Refresh interval
- Component order
- Notification preferences (default all ON)

---

## Setup Wizard Automation

Condensed from 6 steps to 4:

```
1. Welcome (1 click)
2. Connect (2 inputs: API key + Telegram, or skip both)
3. Installing (0 input — all auto)
4. Ready (1 click to open Telegram)
```

### Step 2: Connect — optimizations

- **If `.env` already has keys**: mark them valid, pre-populate (masked), allow "Update" but don't require.
- **Paste detection**: when user pastes, auto-detect which field it belongs to (API keys start with `sk-ant-`, Telegram tokens have a specific format). If pasted into wrong field, auto-swap.
- **Copy-paste friendly**: show visible paste target text (`sk-ant-...`) so user knows what to paste.
- **Skip option**: prominent "Set these up later" link at the bottom. Goes directly to Installing with empty keys. Assistant stays paused.

### Step 3: Installing — optimizations

- **Parallelize** container builds (vault, forge, pioneer) — estimated time drops from 8 min sequential to 3–4 min parallel.
- **Show real-time parallel progress** — 3 progress bars side-by-side.
- **Hide all technical output** behind "Show details" (off by default).
- **Auto-retry on proxy timing race** (the bug we've seen).
- **Celebrate completion** — confetti + "Your assistant is ready!"

### Step 4: Ready — optimizations

- **Open Telegram link** pre-deep-linked to the user's bot (using their bot username extracted from the token).
- **Auto-close wizard** and show system tray first-run notification.
- **Pre-run first security audit** silently in background, show result on Home dashboard.

---

## Home Dashboard Automation

### Proactive alerts

The app detects states requiring user action and surfaces them:

| State | Trigger | Alert text | CTA |
|-------|---------|------------|-----|
| Spending approaching limit | 80% of monthly | "You've used 80% of your spending limit this month." | "Adjust limit" / "View usage" |
| API key expired/invalid | Validation fails | "Your API key needs updating." | "Update key" |
| Update available | Updater check | "A new version of OpenTrApp is ready." | "Update now" / "Remind me later" |
| Security alert | Blocked site attempt | "Your assistant tried to visit a blocked site." | "View details" / "Dismiss" |
| Pending skill scan | New skill from forge | "A new skill is waiting to be scanned." | "Scan now" |
| Assistant paused unexpectedly | Container crashed | "Your assistant stopped. We tried to restart it." | "Open support" |

Alerts appear as banners at the top of the Home Dashboard and (optionally) as system notifications.

### Rotating tip of the day

Surfaces use cases without requiring user to visit Discover page:

```
[Tip of the day]

💡 Did you know? You can ask your assistant to
   "summarize the news from today."

[Try this →]
```

Rotates from a pool of 20–30 use cases. Click → opens Telegram with prefilled message.

Implementation: deterministic pick based on day-of-year % pool size. No tracking.

---

## Developer Mode Automation

Developers appreciate automation too, but expect transparency:

- **Auto-detect and list** all components on startup (already exists)
- **Auto-refresh** statuses on a tunable interval (already exists)
- **Auto-resolve** workflow step dependencies (already exists via orchestrator)
- **Auto-collect** logs into the unified logs viewer (NEW — see spec 14)
- **Auto-validate** manifests against schema on save (NEW)

Developers opt OUT of automation with explicit controls:

- "Disable auto-retry" toggle in dev preferences
- "Show raw output" default (invert of user mode)
- "Manual refresh only" option

---

## Implementation Priorities

### Phase E.1 (foundations)

- [ ] Auto-detect: container runtime (expand existing)
- [ ] Auto-detect: existing `.env` keys (expand existing)
- [ ] Auto-heal: container startup retry wrapper
- [ ] Auto-configure: default all new settings per table above
- [ ] Setup wizard: parallel container builds

### Phase E.2 (user mode)

- [ ] Home dashboard: proactive alerts from backend state
- [ ] Home dashboard: tip-of-the-day rotator
- [ ] Setup wizard: paste detection / auto-swap
- [ ] Setup wizard: Telegram deep-link on Ready

### Phase E.3 (polish)

- [ ] Auto-validate API key periodically
- [ ] Auto-restart on crash with escalation
- [ ] Setup wizard resume on crash mid-flow

---

## Anti-Patterns to Avoid

1. **Don't auto-do destructive things.** Never auto-delete containers, reset configs, or clear data without explicit user action.
2. **Don't auto-install skills.** Skills are user-controlled even after they pass forge scan.
3. **Don't hide failure.** If auto-heal fails, surface it. Don't silently discard errors.
4. **Don't assume user preferences.** Sensible defaults are fine; auto-changing user choices is not.
5. **Don't auto-update without consent.** Offer updates, don't install them silently.
6. **Don't ask questions we can derive.** If `.env` exists and has valid keys, don't re-prompt.
7. **Don't fetch private data without explicit user action.** Network calls to check API key validity are OK (the user gave us that key); scraping the user's chat history is not.

---

## Measurement

Track these metrics to verify automation is working:

- **Clicks from install to Telegram**: target < 15 (currently ~25)
- **Time from double-click to running**: target < 3 min (currently ~5–8)
- **Questions asked in setup**: target ≤ 3 (currently 6+)
- **Auto-heal success rate**: percentage of container starts that succeed without user-visible retry
- **Default-setting override rate**: percentage of users who change auto-configured defaults (low = good defaults)

These are observable in the app (e.g., by instrumenting the setup flow) or via user interviews.

---

## Next

Read `06-failure-ux-strategy.md` to understand what happens when automation can't save us.
