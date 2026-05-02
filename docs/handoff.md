# Handoff — Active Mission

**Last updated:** 2026-05-02 (end of Pass 7 Day 3 — Auto-restart on key rotation shipped)
**Current phase:** Week 2 of the 3-week "Delightful Sloth" UX-coherence polish phase (~2026-04-28 → ~2026-05-19) leading to first public deployment. **Passes 1–6 + Pass 7 Days 1–3 complete. Pass 7 Day 4 (the cleanup + slack day) is next.**
**Branch:** `main` — pushed to `origin/main`
**Last commits:**
- `c27fecc` — Pass 7 Day 3: auto-restart perimeter on key rotation
- `9097c7a` — Pass 7 Day 2: backend status aggregator + alerts evaluator + reachable error_key hero state
- `c052601` — Pass 7 Day 1b: simplify Spending to deep-link + drop Activity tile (vision recheck)
- `6c0c8da` — Pass 7 Day 1a: live spend via Anthropic Admin API (superseded by 1b)
- `a63994d` — End-of-Pass-6 handoff (superseded by this one)

**Pick up at:** Pass 7 Day 4 — the final cleanup day. Order of operations is in this file under "Day 4 — what to ship". After Day 4 the only remaining work in the 3-week window is Pass 8 (pre-ship full re-walk).

---

## ⚠️ Read this whole file before touching code

This session's headline event was a **mid-pass vision recheck on 2026-05-02** that crystallized a sharper scope test for the entire product. It dropped Pass 7's planned scope from ~4 days to ~2.5 and unwound a feature (admin-key-based Spending) that had already shipped Day 1a. **You will not understand why some of the recent commits look like opposites of each other unless you read the recheck section.** Read in this order:

1. **`~/.claude/projects/-home-albertd-Repositories-lobster-trapp/memory/feedback_lobster_trapp_scope.md`** — the 2026-05-02 scope-test feedback. The "deserve-to-exist" question: *does this feature duplicate something the user already has (Anthropic Console, Telegram), or does it solve a problem unique to running OpenClaw safely on a personal machine?*
2. **`~/.claude/plans/no-hardcoding-option-1-partitioned-feigenbaum.md`** — the Pass 7 plan, with the vision-recheck section near the top documenting the 5 decisions and their rationale.
3. **Master plan: `~/.claude/plans/yes-we-are-building-delightful-sloth.md`** — the 8-pass phase plan. Day 4 closes Pass 7; Pass 8 is the pre-ship re-walk.
4. **`docs/specs/2026-04-29-delightful-sloth-target-ux.md`** (Pass 2) — what good looks like for each user moment + the four cross-cutting target behaviors.
5. **`docs/specs/2026-04-20-ux-principles-rubric.md`** (extended in Pass 3) — 13 principles, 19 surfaces. Day 4 re-scores against this.
6. **`~/.claude/projects/-home-albertd-Repositories-lobster-trapp/memory/MEMORY.md`** — project memory index. Read `project_decisions.md` first (top entry is the 2026-05-02 recheck).

---

## Where we are in the master plan

| Pass | Status | Artifacts |
|---|---|---|
| 1 — Dogfood walkthrough | ✅ DONE | `docs/specs/2026-04-28-dogfood-walkthrough-findings.md` |
| 1.5 — Live first-chat signals | ✅ DONE | `docs/specs/2026-04-29-live-signal-first-chat.md` |
| 2 — Aspirational UX spec | ✅ DONE | `docs/specs/2026-04-29-delightful-sloth-target-ux.md` |
| 3 — Rubric extension | ✅ DONE | `docs/specs/2026-04-20-ux-principles-rubric.md` (extended) |
| 4 — Lifecycle ownership | ✅ DONE | `app/src-tauri/src/lifecycle.rs`, `compose.yml`, `commands/lifecycle.rs` |
| 5 — Wizard polish | ✅ DONE | commit `1f879d9` |
| 6 — Dev-tools-lite surface | ✅ DONE | commits `9e5ba11` → `2ea0631` |
| 7 — Notifications + recovery + cleanup | 🟡 IN PROGRESS — Days 1–3 done, Day 4 NEXT | commits `6c0c8da`, `c052601`, `9097c7a`, `c27fecc`; plan in `~/.claude/plans/no-hardcoding-option-1-partitioned-feigenbaum.md` |
| 8 — Pre-ship full re-walk | ⏸ NEXT after Day 4 | (re-score against rubric AND apply the deserve-to-exist test per vision recheck) |

---

## Vision Recheck (2026-05-02) — the most important part of this handoff

While running Day 1a's smoke test (testing real Anthropic spending via the Admin API), Albert reread Anthropic's own warning on admin keys ("manage workspaces, set rate limits, change billing information, and more") and articulated a sharper scope test:

> "Our app's job is to guide a non-technical user towards getting the right API key with handholding. We are helping them set up the framework and ecosystem necessary to participate in the OpenClaw system… our job is NOT to rebuild or replace existing monitoring software for the user."

Crystallized as the **deserve-to-exist test** — defer to existing tools (Anthropic Console for spend/usage/billing; Telegram for chat history) when they cover something well; build only what's unique to running OpenClaw safely on a personal machine.

**Five decisions approved 2026-05-02 (all already executed in commits 1b/2/3 except where noted):**

1. **Drop the admin-key Spending path.** SpendingTile is now a pure deep-link to https://console.anthropic.com/cost. ✅ Done in 1b.
2. **Drop the spending-limit alert + Preferences slider.** Anthropic Console handles billing alerts. ✅ Done in 1b.
3. **Drop the planned "Anthropic API key" → "AI account key" rename.** Karen needs the Anthropic-specific name to find it on Anthropic. ✅ Done in 3 (just didn't rename).
4. **Drop the Activity tile from Home.** Telegram thread is the chronological assistant history. ✅ Done in 1b.
5. **Pass 8 pre-ship walk adopts "does this surface deserve to exist" alongside the existing rubric.** ⏳ To be applied during Pass 8, not Day 4. The candidate surfaces to re-evaluate are listed in the plan file's "Pass 8 — Pre-ship Walkthrough (augmented)" section.

**The principle is now in durable memory** at `feedback_lobster_trapp_scope.md` (with a pointer in `MEMORY.md`). It's a feedback-type memory because it changes how to approach work, not project state. Apply it before adding any new tile, page, alert, or background process.

---

## What Pass 7 shipped, day by day

### Day 1a (`6c0c8da`) — superseded by 1b

Admin-key-based Spending feature: 290 LOC of Rust (`commands/spending.rs`), `useSpending` hook, `BillingAccessSubsection` in Preferences, "Connected" tile state with real $X.XX, 13 unit tests. Worked end-to-end in static verification but admin-key risk wasn't worth the visibility benefit.

### Day 1b (`c052601`) — simplification commit (vision recheck)

Net **-1090 / +42 lines**, 15 files. Unwound:
- Deleted `app/src-tauri/src/commands/spending.rs` (~290 LOC), reverted lib.rs/mod.rs/lifecycle.rs registrations
- Deleted `app/src/hooks/useSpending.ts`
- Rewrote `SpendingTile.tsx` as a pure deep-link card (~30 LOC)
- Dropped `SpendingSection` + `BillingAccessSubsection` from `Preferences.tsx` (-325 LOC) — Preferences went 6 → 5 sections
- Dropped Activity tile from Home; grid went 3 → 2 tiles (Security + Spending-deep-link)
- Dropped `spendingLimit` field from `AppSettings` + `notifications.spendingLimit`
- Dropped `SpendingSummary` types + `getSpendingSummary` invoke wrapper
- Reverted `parseEnvKeys` to its original 2-field shape
- Removed `isAnthropicAdminKeyLike`, `"billing"` ErrorContext, `ANTHROPIC_ADMIN_API_KEY` in `redact_secrets`
- Updated one e2e assertion that tested the removed slider

Cargo lib went from 46 → **33** (back to baseline; +13 spending tests deleted along with the module). Vitest 175. E2E 25/25.

### Day 2 (`9097c7a`) — backend status aggregator + alerts evaluator

Net **+782 / -106 lines**, 8 files. New `app/src-tauri/src/status_aggregator.rs` (~520 LOC):
- `AssistantStatus` enum: `Ok | NotSetup | Starting | Recovering | ErrorPerimeter | ErrorKey | PausedByUser`
- `Alert` struct with `severity / title / body / cta / dismissable / suppress_during_wizard` fields
- `AssistantStatusStore` (Tauri-managed Mutex)
- `spawn_status_evaluator(handle, 60s)` tokio task — reads from `PerimeterStateStore` (the existing 30s container watchdog), reads `.env` for key presence, optionally probes Anthropic auth
- Auth probe: `GET https://api.anthropic.com/v1/models` (free, auth-required, no token cost). 5-min TTL cache; key rotation invalidates immediately because cache keys on the literal key value
- Inconclusive probes (network/5xx/timeout) stay optimistic — better quiet than false alarm
- 4 alert rules: missing-anthropic-key, invalid-anthropic-key, missing-telegram-token, perimeter-error. **No spending-limit rule (per vision recheck).** Threat-blocked rule deferred (would need vault-proxy log access)
- 20 unit tests covering each `derive_status` branch, each `build_alerts` rule, env-parse edge cases

Frontend rewires:
- `lib/tauri.ts`: `AssistantStatus` union, `BackendAlert`, `AssistantStatusSnapshot`, `getAssistantStatus()`
- `useHero` subscribes to `assistant-status-changed` instead of `perimeter-state-changed`. `HeroState` gains `error_key` + `paused_by_user`. The `hasBeenRunning` ref still flips first-time partial state from `recovering` → `starting`
- `useAlerts` becomes a thin subscriber to the backend event. All 4 rules are server-side now. Frontend's only job is `suppress_during_wizard` filtering and `dismissedAlerts` persistence
- `HeroStatusCard`: new `error_key` branch ("Your Anthropic key isn't working — Update your key" CTA → /preferences) and new `paused_by_user` branch (placeholder, "Paused — resume from the tray menu")
- `Home.tsx` `securityFromHero`: handles new states (error_key keeps Sandbox=Safe since perimeter is fine; paused_by_user shows "Sandbox is stopped on purpose")

**One script fix worth knowing**: `tests/orchestrator-check.sh` section 6's regex scan only walked `app/src-tauri/src/commands/*.rs`. Since `status_aggregator.rs` lives at `app/src-tauri/src/status_aggregator.rs` (top-level, not in `commands/`), the script saw `get_assistant_status` invoked from frontend with no Rust handler and warned. Fix: changed the glob to `app/src-tauri/src/**/*.rs` recursively. Catches future top-level command modules too.

Cargo lib went 33 → **53** (+20 status_aggregator tests).

### Day 3 (`c27fecc`) — auto-restart on key rotation

Net **+110 / -16 lines**, 5 files.

- New Tauri command `restart_perimeter` in `commands/lifecycle.rs`. Reuses `bring_perimeter_down_sync` + `run_compose` helpers. Down/up sequence runs inside `tokio::task::spawn_blocking` so the tokio reactor isn't stalled for the ~20s typical restart. Returns `Result<(), String>` with a Karen-friendly error message if the up phase fails (most likely a malformed key the user just saved)
- New `restartPerimeter()` invoke wrapper in `lib/tauri.ts`
- `Preferences.tsx KeysSection.save()` rewired:
  - Drops edit UI immediately on key write (refresh masks)
  - Sticky info toast "Restarting your assistant…" (`duration: 0`)
  - Awaits `restartPerimeter()`
  - On success: dismisses info toast, shows success "Your assistant is back online — your new key is active"
  - On failure: dismisses info toast, shows sticky error toast classified via the existing `UNKNOWN_FALLBACK` path
- **API ripple:** `ToastContext.addToast` now returns the generated id (was `void`) so Preferences can dismiss the sticky "Restarting…" toast cleanly. All other callers continue to work because they ignore the return value. Type signature `AddToastFn = (toast: Omit<Toast, "id">) => string`
- Up budget for the restart cycle: 60s (down has its own internal 30s budget). Generous headroom for slower laptops without hanging the UI indefinitely

**Manual smoke test deferred** to Day 4's live walkthrough. Static verification is clean and the wiring is straightforward (down + up cycle), but the actual key-rotation → vault-agent-restart → Telegram-still-works happy path hasn't been observed live yet.

---

## Day 4 — what to ship

**Goal:** finish Pass 7. Order by leverage; stop when the day runs out (the recheck saved budget so this should land cleanly):

### 1. `paused_by_user` hero state — the only remaining hero state hole

Backend:
- New Tauri command `pause_perimeter` in `commands/lifecycle.rs` — `compose stop` (preserves containers, vs `down` which removes them) + sets a `paused: true` flag in `PerimeterStateStore` so the watchdog knows this state is intentional, not a crash
- The status_aggregator's `derive_status` already has a `PausedByUser` arm; wire it to read the `paused` flag rather than always returning `ErrorPerimeter` for stopped containers
- New `resume_perimeter` command — `compose start` + clear the flag

Frontend:
- `HeroStatusCard` already has the `paused_by_user` branch (Day 2 stubbed it). Add a "Pause" affordance on the hero CTA when state is `running_safely` (small ghost button next to Open Telegram). Add a "Resume" affordance on the hero CTA when state is `paused_by_user` (replace the placeholder text)
- `restartPerimeter` should NOT trigger when paused — it'd silently un-pause
- Tray menu's status item should reflect "Paused" when paused

Verification: pause from hero → all 4 containers status: stopped, watchdog reports them stopped, but no perimeter-error alert fires, hero shows "Your assistant is paused", resume from hero → containers come back up

### 2. OS-level autostart wiring

`tauri-plugin-autostart` is already a dep (Cargo.toml line 25, `tauri_plugin_autostart::init` already in the plugin chain in lib.rs). Just need to register/unregister at OS level when the user toggles `settings.autostart` in Preferences.

- `useSettings`'s `update` callback for `autostart` → call `enable()` or `disable()` from `@tauri-apps/plugin-autostart`
- On first load: if `settings.autostart === true` and the OS-level state is unset, call `enable()` to sync them
- StartupSection currently shows a "applies on next reboot" message that should change to "Will start with your computer" / "Won't start automatically" on toggle

### 3. Notification OS permission gate

Tauri notification plugin requires OS permission. Currently the toggles in NotificationsSection (`securityAlerts`, `updates`) persist but don't prompt for OS permission.

- On first toggle of any notification preference from off → on, check OS-level permission via `@tauri-apps/plugin-notification`'s `isPermissionGranted()`
- If not granted, call `requestPermission()`. Surface result via toast
- If user denies, leave the toggle on but show a small "Permission denied — turn on in OS settings" inline note

### 4. Banned-term sweep

Apply the deserve-to-exist principle while you sweep. Notable candidates to confirm absent from user-facing surfaces:
- "admin key", "billing scope", "cost endpoint" — should all be gone after Day 1b but verify
- The Pass-1.5 gaps that were never added to `app/e2e/user-facing.spec.ts` line ~13: bare `container`, `sandboxed`, `web_search`, `web_fetch`. Add to the regression list if not already

### 5. Rubric re-score

Update `docs/specs/2026-04-20-ux-principles-rubric.md` for the surfaces touched in Pass 7:
- Hero card now reaches `error_key` state — re-score Home with the new state included
- Spending tile is now a deep-link — re-score against the deserve-to-exist principle as well (Pass 8 will do the full sweep but a Day 4 spot-score helps calibrate)
- Preferences lost a section — confirm the re-rendered Preferences still scores ≥8.5

Configuration row stays at 8.0 (the rename was dropped per recheck) — that's acceptable, document it.

### 6. Manual end-to-end walkthrough

This is the smoke test that Day 1a, 2, 3 deferred. Run the actual app:
1. Launch dev app: `cd app && npm run tauri dev` (~1.5GB memory + ~2 min build — check `free -h` first)
2. Walk Karen's full journey: Home → click Spending tile (opens Console in browser) → Preferences → rotate Anthropic key → observe restart toast sequence → Telegram bot still replies on next message
3. Trigger error_key: rotate to `sk-ant-api03-WRONG` → within ~5 min the hero flips to error_key, alert banner shows "Your Anthropic key isn't working" → rotate back → recovers
4. Pause from hero: → containers stop → no error alert → resume from hero → containers come back

If anything feels clunky, that's a Pass 8 P0.

### 7. Handoff + memory update

- Supersede this handoff with end-of-Pass-7 state
- Update `project_status.md` (it's currently stale — still says "Pass 7 next")
- Append Day 4 line to `project_decisions.md` if any decisions are made

### 8. Commit + push

Single commit per logical chunk (paused_by_user / autostart / permission gate). Push to origin/main as you go (project pattern).

---

## Locked decisions (most-recent-first)

- **2026-05-02 — Vision recheck.** Lobster-TrApp is onboarding scaffolding, not a dashboard. Defer to Anthropic Console and Telegram for what they cover. Five concrete decisions executed (admin-key drop, spending-limit drop, no rename, Activity tile drop, Pass 8 deserve-to-exist test). Saved at `feedback_lobster_trapp_scope.md`.
- **2026-04-30 — Pass 6 Option B.** Home + Discover + Preferences real; Security + Help friendlier placeholders. Done.
- **2026-04-29 — Watchdog reports state; auto-restart owned by `restart: unless-stopped`.** Don't add a competing supervisor. (Day 3's `restart_perimeter` command is user-initiated, not auto — different thing.)
- **External-AI-as-warden via MCP is OUT OF SCOPE for this phase** (deferred to v0.3+).
- **No new sidecars** (vault-calendar/voice/email) this phase.
- **Backend orchestrator architecture changes** are out — engine is solid.
- **Pioneer code** is frozen (target API acquired by Meta).

---

## Working state at handoff time

- **All 4 containers up** (running 2 days continuously per `podman ps`).
- **Working tree clean.** `git status` clean as of `c27fecc`.
- **Pushed:** `origin/main` at `c27fecc`.
- **Cargo build clean.** Same 2 pre-existing dead-code warnings on unused `WorkflowStatus`/`StepStatus` variants.
- **Cargo test green.** 53/53 passing (33 pre-Pass-7 + 20 new status_aggregator tests).
- **Vitest green.** 175/175.
- **E2E green.** 25/25.
- **Orchestrator-check.** 42/42 with **0 warnings**.
- **TSC clean.**
- **Memory pressure note.** During this session memory was tight (~5GB used / 7.2GB total + 2–3GB swap). Tauri dev launch will push it close to the limit — check `free -h` before `npm run tauri dev` and close anything non-essential first.

---

## Memory updates (the next instance should read these BEFORE opening code)

- **`feedback_lobster_trapp_scope.md` (NEW 2026-05-02)** — the deserve-to-exist test. Read first.
- **`project_decisions.md`** — top entry is the vision recheck. Read second.
- **`project_status.md`** — currently STALE (says "Pass 7 next"). Refresh it as part of Day 4's memory update step.

---

## Quick verification commands at session start

```bash
# Confirm working state
git log --oneline -5
git status --short
podman ps --format "table {{.Names}}\t{{.Status}}"
podman inspect vault-agent vault-proxy vault-forge vault-pioneer \
  --format "{{.Name}}: {{.HostConfig.RestartPolicy.Name}}"

# Confirm Rust still builds
cd app/src-tauri && cargo build && cargo test --lib

# Confirm frontend still type-checks
cd ../app && npx tsc --noEmit && npm test

# Confirm orchestrator-check still passes
cd .. && bash tests/orchestrator-check.sh
```

Expected:
- `git status` clean.
- 4 containers up with `unless-stopped` policy.
- Cargo build clean (only pre-existing workflow warnings).
- 53/53 cargo tests, 175/175 vitest, 42/42 orchestrator-check (0 warnings).
- TSC clean.

---

## Historical handoffs (preserved in git history)

- `a63994d` — 2026-04-30 (end of Pass 6): Pass 6 done + Pass 7 deferred backlog. **Superseded** by this commit.
- `3cc2b4e` — 2026-04-29 (afternoon): Pass 4 wrap-up + Pass 5 spec.
- `d55bdbd` — 2026-04-26: v0.2.0 ship + two-track v0.3 mission.
- `95cec0c` — 2026-04-25: morning, fix-first mandate (F11 root cause).
- `8d2e8cc` — 2026-04-24: morning, mission pivot (Karen → prosumer, harness Phase 0-2).
- `2d25299` — Phase E.2.1 → E.2.2 (paused).
- `b480607` — Phase E.2.0 → E.2.1.
- `88688c2` — Phases A–D + v0.1.0 release.

This handoff supersedes the prior end-of-Pass-6 doc (`a63994d`).

---

## tl;dr — the first 30 minutes of the next session

1. Read this handoff to the end (you're almost there).
2. Read `feedback_lobster_trapp_scope.md` and the top entry of `project_decisions.md`. The vision recheck reframes how to evaluate every feature decision going forward.
3. Skim `~/.claude/plans/no-hardcoding-option-1-partitioned-feigenbaum.md` — Day 1b/2/3 sections are historical; Day 4 + Pass 8 are forward-looking.
4. Run the verification commands above. Confirm: clean working state + 4 containers up + 53/175/25/42 all green.
5. **Highest-leverage Day 4 starting points**, in priority order:
   - **`paused_by_user` hero state** (#1 above): closes the last hero state gap. The frontend already has the branch — needs `pause_perimeter` + `resume_perimeter` commands and the `paused: true` flag in `PerimeterStateStore`. ~2 hours.
   - **OS autostart wiring** (#2): the plugin and the setting both exist; just need to call `enable()`/`disable()` from the toggle handler. ~1 hour.
   - **Notification permission gate** (#3): single new code path on first toggle off → on. ~1 hour.
   - **Banned-term sweep + rubric re-score + manual walkthrough + handoff update + commit** (#4–#8): the remainder of the day.
6. Apply the deserve-to-exist test as you go. If you find yourself building anything that duplicates Anthropic Console or Telegram, stop and link out instead.
7. Pass 8 starts after Day 4 — **augmented Pass 8 asks both "is this surface polished?" AND "does this surface deserve to exist?"** Some Pass 6 surfaces may not survive that question (the Activity tile already didn't, mid-Pass-7).

The 3-week window has comfortable slack remaining. Pass 7 was budgeted ~4 days, used ~3 (Day 1a + 1b + 2 + 3). Day 4 + Pass 8 in the remaining ~10 days of the window leaves plenty of room.
