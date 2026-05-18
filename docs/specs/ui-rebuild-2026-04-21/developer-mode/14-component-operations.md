# Developer Mode: Component Operations

**Prerequisite reading:** `13-dev-dashboard-overview.md`
**Routes:** `/dev/components`, `/dev/components/:id`, `/dev/logs`, `/dev/security`, `/dev/allowlist`, `/dev/shell-levels`, `/dev/manifests`, `/dev/preferences`
**Audience:** Developers. Density and completeness over simplicity.

---

## Purpose

Detail every developer-mode screen. Most reuse existing components (CommandPanel, WorkflowPanel, ConfigPanel) with minor density tweaks. New screens fill gaps: unified logs, manifest inspector, allowlist editor, shell level selector, security audit runner.

---

## /dev/components

### Layout

Grid of all components (canonical names, dense metadata).

```
opencli-container    v0.1.0  runtime   ● running   [Open →]
openskill-forge     v0.1.0  toolchain ● ready     [Open →]
openagent-social  v0.0.1  network   ◐ placeholder [Open →]
```

Click row → `/dev/components/:id`.

### Priority

- **MVP:** Required. Dense list of components.
- **Files:** `app/src/pages/dev/DevComponentsList.tsx`

---

## /dev/components/:id

### Reuse existing ComponentDetail

The current `app/src/pages/ComponentDetail.tsx` is mostly dev-suitable already. Refactor to:

1. Extract user-mode logic (the contextual guidance, role-based header) into a wrapper
2. Dev-mode uses raw component name, canonical role, version, full commands/configs/workflows
3. All commands visible (both `tier: user` and `tier: advanced`)
4. All configs editable with schema validation
5. Additional dev-only tabs:
   - **Logs** (filtered to this component)
   - **Manifest** (live view of parsed YAML)
   - **Metrics** (live memory, CPU, network)

### Priority

- **MVP:** Required. Leverages existing code.
- **Files:** Refactor `app/src/pages/ComponentDetail.tsx` → `app/src/pages/dev/DevComponentDetail.tsx`

---

## /dev/logs (NEW)

### Purpose

Unified, filterable, real-time log stream from all components.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Logs                              [ Clear ] [ Export ]       │
│                                                              │
│ Filter: [ all ▼ ] [ all levels ▼ ]  Search: [____________]  │
│                                                              │
│ ┌──────────────────────────────────────────────────────────┐│
│ │ 15:30:42.123 [vault]   INFO  Telegram message received   ││
│ │ 15:30:43.456 [proxy]   INFO  anthropic.com 200 242ms     ││
│ │ 15:30:45.789 [vault]   INFO  Tool: web_fetch weather.com ││
│ │ 15:30:46.012 [proxy]   INFO  weather.com 200 118ms       ││
│ │ 15:30:48.345 [vault]   INFO  Telegram message sent       ││
│ │ 15:31:10.678 [forge]   WARN  Skill scan took 12s         ││
│ │ ...                                                       ││
│ │                                                          ││
│ │ [auto-scroll at bottom]                                  ││
│ └──────────────────────────────────────────────────────────┘│
│                                                              │
│ ☑ Auto-scroll    ☐ Pause    Showing 342 lines                │
└──────────────────────────────────────────────────────────────┘
```

### Features

- Filter by component (all / vault / forge / pioneer / proxy)
- Filter by level (all / INFO / WARN / ERROR)
- Search box with substring match and highlighting
- Auto-scroll toggle (keeps newest at bottom)
- Pause toggle (freezes display, still receiving)
- Clear (clears local display, doesn't stop)
- Export (saves current filter view to `.log` file)
- Line click → reveals full event details in side panel

### Backend

Backend needs to:
1. Capture stdout/stderr from each container
2. Route through a shared log bus (Rust channel)
3. Emit Tauri event `log:appended` for each line
4. Persist N latest lines (ring buffer, configurable, default 10,000 lines) to disk for app restart resilience

### Priority

- **P1:** Can ship dev mode without initially, but valuable.
- **Files:** `app/src/pages/dev/DevLogs.tsx`, `app/src-tauri/src/log_bus.rs`, `app/src-tauri/src/commands/logs.rs`

---

## /dev/security (NEW)

### Purpose

Explicit UI to run and view the 24-point security audit from the vault. Replaces the hidden "Security Check" workflow invocation.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Security Audit                        [ Run audit ]          │
│                                                              │
│ Last run: 2026-04-21 15:28 (2 minutes ago)                   │
│ Result: ✓ 24/24 passed                                       │
│                                                              │
│ ┌──────────────────────────────────────────────────────────┐│
│ │ [✓] 1.  Network: vault-proxy hostname resolves           ││
│ │ [✓] 2.  Network: TCP connect to vault-proxy:8080         ││
│ │ [✓] 3.  Filesystem: root is read-only                    ││
│ │ [✓] 4.  Capabilities: ping blocked (NET_RAW dropped)     ││
│ │ ...                                                       ││
│ │ [✓] 24. Config integrity (no tampering)                  ││
│ └──────────────────────────────────────────────────────────┘│
│                                                              │
│ [ Export full report ] [ Schedule recurring audits ]         │
│                                                              │
│ Audit history                                                │
│ 15:28 PASS 24/24                                             │
│ 12:01 PASS 24/24                                             │
│ 09:00 PASS 24/24                                             │
│ Yesterday 19:00 FAIL 23/24 (check 2: proxy timing)           │
│ ...                                                          │
└──────────────────────────────────────────────────────────────┘
```

### Features

- Manual "Run audit" triggers the existing vault workflow
- Full check list with pass/fail per item
- Clicking a failed check shows raw output in expanded panel
- Export report as Markdown or JSON
- Schedule recurring audits (daily/weekly) via Tauri scheduled task
- History list with click-to-view-past-report

### Priority

- **MVP:** Required. Existing workflow wrapped in dedicated UI.
- **Files:** `app/src/pages/dev/DevSecurity.tsx`, reuses existing `run_workflow` command

---

## /dev/allowlist (NEW)

### Purpose

CRUD editor for the vault-proxy allowlist (list of domains the agent can reach).

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Allowlist                          [ + Add domain ]          │
│                                                              │
│ 14 domains allowed                                           │
│                                                              │
│ ┌──────────────────────────────────────────────────────────┐│
│ │ anthropic.com                                       [×]  ││
│ │ api.anthropic.com                                   [×]  ││
│ │ wikipedia.org                                       [×]  ││
│ │ en.wikipedia.org                                    [×]  ││
│ │ weather.com                                         [×]  ││
│ │ ...                                                       ││
│ └──────────────────────────────────────────────────────────┘│
│                                                              │
│ Recently blocked                                             │
│                                                              │
│ tracker.ads.com        2h ago    [ Add to allowlist ]        │
│ malicious-site.io      5h ago    [ Add to allowlist ]        │
│                                                              │
│ ⚠️ Changes require restarting the proxy. [ Apply & restart ] │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Features

- Read current allowlist from `components/opencli-container/proxy/allowlist.txt`
- Add domain (validates pattern: domain or wildcard)
- Remove domain (with confirmation)
- "Recently blocked" list pulled from proxy logs — one-click add to allowlist
- Apply & restart: write file, restart vault-proxy container
- Validation: warn if adding overly broad patterns like `*.com`

### Priority

- **P1:** Can ship dev mode without initially; useful.
- **Files:** `app/src/pages/dev/DevAllowlist.tsx`, `app/src-tauri/src/commands/allowlist.rs`

---

## /dev/shell-levels (NEW)

### Purpose

Switch the vault's shell level (Hard / Split / Soft) — the main security knob.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Shell Levels                                                 │
│                                                              │
│ Current: ● Soft Shell                                        │
│                                                              │
│ ┌──────────────────────────────────────────────────────────┐│
│ │ ○ Hard Shell                                             ││
│ │   Maximum restriction. Agent can only respond to         ││
│ │   messages — no tool use, no file writes, no web.        ││
│ │   Use when: you want the agent to chat only.             ││
│ │                                                          ││
│ │ ○ Split Shell                                            ││
│ │   Supervised. Tools are allowed but each invocation      ││
│ │   asks for user approval.                                ││
│ │   Use when: you want control over every action.          ││
│ │                                                          ││
│ │ ● Soft Shell (default)                                   ││
│ │   Agent operates autonomously within the security        ││
│ │   boundaries (allowlist, sandbox).                       ││
│ │   Use when: you trust the configured boundaries.         ││
│ └──────────────────────────────────────────────────────────┘│
│                                                              │
│ Changing the shell level restarts the vault container.       │
│                                                              │
│ [ Apply changes ]                                            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Features

- Radio selection of shell level
- Explanation of each
- Apply triggers config write + vault restart

### Priority

- **P2:** Defer if needed. Current shell = Soft by default works for MVP.
- **Files:** `app/src/pages/dev/DevShellLevels.tsx`, uses existing vault configs

---

## /dev/manifests (NEW)

### Purpose

Inspect parsed component.yml manifests. Read-only tree view with schema validation.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Manifest Inspector                                           │
│                                                              │
│ Component: [ opencli-container ▼ ]                              │
│                                                              │
│ ┌──────────────────────────────────────────────────────────┐│
│ │ ▼ identity                                               ││
│ │     id: "opencli-container"                                 ││
│ │     name: "OpenCli Container"                               ││
│ │     version: "0.1.0"                                     ││
│ │     role: "runtime"                                      ││
│ │                                                          ││
│ │ ▼ status                                                 ││
│ │   ▼ states (6)                                           ││
│ │       ● running                                          ││
│ │       ● stopped                                          ││
│ │       ...                                                ││
│ │   ▼ probes (2)                                           ││
│ │       ...                                                ││
│ │                                                          ││
│ │ ▼ commands (13)                                          ││
│ │   ▼ start — user tier, danger: safe                      ││
│ │   ▼ stop — user tier, danger: safe                       ││
│ │   ...                                                    ││
│ │                                                          ││
│ │ ✓ Validates against schema                               ││
│ └──────────────────────────────────────────────────────────┘│
│                                                              │
│ [ Copy as JSON ] [ Copy as YAML ] [ Open file in editor ]    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Features

- Dropdown to select component
- Collapsible tree view
- Schema validation badge at bottom (valid/invalid/warnings)
- Copy buttons
- "Open in editor" uses Tauri shell to open file externally

### Priority

- **P1:** Can ship without; valuable for contributors.
- **Files:** `app/src/pages/dev/DevManifests.tsx`

---

## /dev/preferences (expanded from user preferences)

### Purpose

Full technical preferences. Everything user mode hides lives here.

### Layout

Sections:
1. **Mode** — toggle back to user mode
2. **App data location** — path override (user mode hides this)
3. **Component refresh interval** — slider 2–60s
4. **Log retention** — buffer size, persistence
5. **Auto-heal behavior** — retries, backoff, escalation thresholds
6. **Developer preferences** — keyboard shortcuts customization, dense mode, show dev warnings
7. **Debug flags** — enable verbose logging, enable raw streams, show error traces

### Features

- All settings auto-save
- Some require restart (indicated)
- Reset to defaults button at bottom

### Priority

- **MVP:** Required.
- **Files:** `app/src/pages/dev/DevPreferences.tsx`

---

## Reused Components from Existing UI

These components already exist and are used as-is (or with minor density tweaks):

| Existing component | Used in dev mode route | Change needed |
|-------------------|-------------------------|---------------|
| `CommandPanel` | `/dev/components/:id` | Show all tiers; minor density tweaks |
| `WorkflowPanel` | `/dev/components/:id` | Show all workflows; minor density |
| `ConfigPanel` | `/dev/components/:id` | Same |
| `StatusBadge` | Multiple | Same |
| `HealthBadge` | Multiple | Same |
| `DynamicIcon` | Sidebar | Same |

New components for dev mode:

- `LogViewer` — the large real-time log component
- `ManifestTree` — collapsible tree
- `AuditChecklist` — 24-point check display
- `AllowlistEditor` — domain CRUD
- `ShellLevelSelector` — radio group with explanations

---

## Priority Matrix (What To Ship When)

| Screen | Priority | Blocker? |
|--------|----------|----------|
| /dev (Overview) | MVP | Yes |
| /dev/components | MVP | Yes (sidebar nav) |
| /dev/components/:id | MVP | Yes (reused from existing) |
| /dev/preferences | MVP | Yes |
| /dev/security | MVP | Yes (core selling point) |
| /dev/logs | P1 | No (cool-to-have) |
| /dev/allowlist | P1 | No |
| /dev/manifests | P1 | No |
| /dev/shell-levels | P2 | No |

Ship MVP + P1 for initial dev mode release. P2 can follow later.

---

## Acceptance Criteria

- [ ] All MVP routes implemented and navigable from sidebar
- [ ] Component detail shows full info (commands, configs, workflows, all tiers)
- [ ] Security audit runs and displays 24 check results
- [ ] Preferences page allows all technical settings
- [ ] Command palette works (Cmd+K)
- [ ] Keyboard shortcuts documented in `?` dialog
- [ ] Dev mode visually distinct from user mode

---

## Testing

### Playwright tests

- Toggle into dev mode, verify route changes
- Navigate each sidebar item, verify pages load
- Run security audit, verify results display
- Open command palette, search, navigate
- Exit dev mode, verify return to user mode

### Unit tests

- Manifest tree renders correctly for each component
- Log filter logic (by component, by level, by substring)
- Audit checklist correctly parses workflow output

---

## End of Spec Folder

Now go back and read `00-HANDOFF.md` for the implementation plan.
