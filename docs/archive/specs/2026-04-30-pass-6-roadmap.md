# Pass 6 — Dev-tools-lite Surface (Option B)

**Date authored:** 2026-04-30 (start of Pass 6 work)
**Phase:** Week 2 of the 3-week "Delightful Sloth" UX-coherence polish phase
**Status:** Spec locked; implementation begins Day 1 (today)
**Estimate:** 5 days
**Branch:** `main`

---

## Why this doc exists

Pass 6 implements against existing source-of-truth specs but layers in a **scope cut** the page-level specs don't address. The page specs (`docs/specs/ui-rebuild-2026-04-21/user-mode/{08,10,12}.md`) describe the target state assuming several backend subsystems exist (activity tracking, spending aggregation, alerts evaluator, status aggregator). Most of those subsystems do NOT exist yet — only the perimeter-state half (Pass 4) is shipped.

This doc records:
1. **What ships in Pass 6** vs. what slips to Pass 7.
2. **How the 6-state hero spec maps onto the 5-state perimeter backend** (they're not 1:1).
3. **Per-day slicing** of the 5-day budget so each day produces a shippable, honest surface.
4. **Done-criteria** per day so progress is verifiable.

---

## Source-of-truth specs (do not duplicate; link)

| What | Where |
|---|---|
| Phase plan / out-of-scope | `~/.claude/plans/yes-we-are-building-delightful-sloth.md` (master plan) |
| Cross-cutting target-state UX | `docs/specs/2026-04-29-delightful-sloth-target-ux.md` (Pass 2) |
| Hero state machine matrix | `2026-04-29-delightful-sloth-target-ux.md` lines 92-105 |
| Home dashboard page spec | `docs/specs/ui-rebuild-2026-04-21/user-mode/08-home-dashboard.md` |
| Preferences page spec | `docs/specs/ui-rebuild-2026-04-21/user-mode/10-preferences.md` |
| Discover / use-case gallery | `docs/specs/ui-rebuild-2026-04-21/user-mode/12-use-case-gallery.md` |
| 13-principle rubric + score matrix | `docs/specs/2026-04-20-ux-principles-rubric.md` |
| Pass 4 backend interface (perimeter state) | `app/src-tauri/src/lifecycle.rs:38-90`; `app/src-tauri/src/commands/lifecycle.rs` |

Read those before code; this doc *only* covers what's adapted, scope-cut, or sequenced for Pass 6.

---

## What ships in Pass 6 vs what's deferred to Pass 7

### Ships in Pass 6

- **Real Home page** with hero state machine wired to live `get_perimeter_state` + `perimeter-state-changed` event.
- **Three stat tiles** (Security / Activity / Spending) on Home — Security tile derives from perimeter state; Activity and Spending tiles render **honest stub copy** ("None yet" / "$0.00 this month") because the backend tracking doesn't exist yet.
- **Real Preferences page** for: Anthropic key (rotate), Telegram bot (rotate), spending limit (display-only initially), notifications, autostart, close-to-tray, theme. Uses existing `useSettings` + `writeConfig` for `.env` updates. Errors route through `classifyError` (Pass 5).
- **Real Discover page** rendering a use-case gallery from existing fixtures (`app/src/data/useCases.ts` or equivalent). Each card opens a Telegram deep-link with prefilled message via `settings.telegramBotUrl`.
- **Tip-of-the-day card** on Home (deterministic day-of-year pick from the same use-case fixture).
- **Proactive alerts banner** on Home — displays alerts from a frontend-evaluated rule set (no backend evaluator). The 60s backend evaluator slips to Pass 7.
- **Friendlier placeholder copy** on Security and Help (replaces the spec-path-leaking `UserPlaceholder` with a static "We're still building this — talk to your assistant on Telegram while we finish" page).
- **Rubric re-score** of rows 14–18.

### Deferred to Pass 7 cleanup

- **Activity tracking subsystem** — persisting agent task events to Tauri store (per `08-home-dashboard.md` Backend additions §1).
- **Spending aggregation subsystem** — reading vault-proxy `Anthropic API call` log entries, computing `tokens × price_per_token`, persisting daily/monthly totals (per `08-home-dashboard.md` Backend additions §2).
- **Alerts evaluator** — 60-second backend rule check that populates an alerts queue (per `08-home-dashboard.md` Backend additions §4). Pass 6 ships a frontend-only evaluator over the data we already have.
- **Status aggregator** — combines container health + API key validity + proxy readiness into a single `assistant_status` enum (per `08-home-dashboard.md` Backend additions §3). Pass 6 reuses `PerimeterState` directly.
- **Add-tool / Get-more-abilities affordance** — Moment 6 in the Pass 2 spec calls for either folding into Discover or adding a 6th sidebar item. Pass 6 leaves Discover at "browse + deep-link"; install-from-Discover slips to Pass 7.
- **Skeleton loading states** — the spec calls for `<SkeletonCard>` / `<SkeletonText>` on first open. Pass 6 uses a brief spinner; skeleton polish slips to Pass 7.
- **Status illustrations** (`status-{state}.svg`) — the spec calls for 6 illustrations. Pass 6 reuses the existing pulsing-rings / lobster art from the wizard for now; bespoke illustrations slip to Pass 7.

### Out-of-scope this phase (master-plan locked, do not reopen)

- External-AI-as-warden via MCP.
- New capability sidecars (vault-calendar / voice / email).
- Backend orchestrator architecture changes.
- Pioneer code (target API frozen).

---

## Hero state mapping: 6-state spec → 5-state backend

The Pass 2 spec defines a 6-state hero (`running_safely` / `paused_by_user` / `something_needs_attention` / `error_perimeter` / `not_setup` / `error_key`). The Pass 4 backend exposes 5 states (`NotSetup` / `Starting` / `RunningSafely` / `Recovering` / `Stopped`). Pass 6 maps as follows:

| Hero state (spec) | Trigger (backend) | Visual |
|---|---|---|
| `running_safely` | `PerimeterState::RunningSafely` | green; "Your assistant is running safely" |
| `starting` *(new in Pass 6)* | `PerimeterState::Starting` OR `Recovering` during initial bring-up | amber; "Your assistant is starting…" |
| `recovering` | `PerimeterState::Recovering` AND watchdog has seen `RunningSafely` previously this session | amber; "Your assistant is taking a moment — back in a few seconds" |
| `error_perimeter` | `PerimeterState::Stopped` AND wizard previously completed | red; "Your assistant didn't fully recover. Let's restart it together" + `Try to fix` button |
| `not_setup` | `PerimeterState::NotSetup` (typically: wizard never run) | gray; "Your assistant isn't set up yet" + `Run setup` |
| `paused_by_user` | NOT YET REACHABLE — backend doesn't track user-initiated stop. Pass 6 does NOT render this state. Slips to Pass 7 once "Pause" affordance is wired. |
| `error_key` | NOT YET REACHABLE — backend doesn't validate the Anthropic key. Slips to Pass 7 once the alerts evaluator catches 401 responses from `api.anthropic.com`. |

Distinguishing `starting` from `recovering`: the frontend hook tracks whether `RunningSafely` has been observed at least once during this session. If yes, future `Recovering` states render as `recovering`. If no, they render as `starting`.

---

## Per-day slicing

### Day 1 (today, 2026-04-30) — Home hero + tiles skeleton

**Files to create / replace:**
- `app/src/pages/user/Home.tsx` — replace `UserPlaceholder` with the real layout.
- `app/src/components/user/HeroStatusCard.tsx` — the central card with state-dependent copy + CTAs.
- `app/src/components/user/StatTile.tsx` — reusable tile component.
- `app/src/hooks/useHero.ts` — listens to `perimeter-state-changed`, fetches initial state via `get_perimeter_state`, derives the 5 reachable hero states using the mapping above.

**Wire-up:**
- Hero gets state from `useHero()`.
- Security tile reads from `useHero()` directly (`safe` if `running_safely`, `Needs attention` if `recovering`/`error_perimeter`, `Unknown` otherwise).
- Activity tile renders honest stub: "None yet" + sub-line "Your assistant has no tasks today."
- Spending tile renders honest stub: "$0.00 this month" + sub-line "We're still wiring this up."
- All three tiles are clickable (Security → `/security`, Activity → `/security`, Spending → `/preferences`) but the click-targets are still placeholder pages until Day 5.

**Done when:**
- `Home.tsx` no longer renders `UserPlaceholder`. The phrase `Coming in Phase E.2.` appears nowhere on the page.
- `podman stop vault-agent` while app is open drives the hero amber within 30s (one watchdog tick).
- `podman start vault-agent` (or wait for `restart: unless-stopped`) drives the hero back to green.
- Banned-term sweep on the Home page is clean.
- Existing wizard E2E + cargo tests still pass.
- One commit, pushed to `origin/main`.

### Day 2 — Preferences

**Files to create / replace:**
- `app/src/pages/user/Preferences.tsx` — replace `UserPlaceholder`.
- `app/src/components/user/preferences/KeyRotationCard.tsx` — reused for both Anthropic + Telegram (mostly mirrors `ConnectStep`'s key cards).
- `app/src/components/user/preferences/SpendingLimitCard.tsx` — display-only spending data + slider for monthly cents limit.
- `app/src/components/user/preferences/NotificationsCard.tsx`, `AutostartCard.tsx`, `CloseToTrayCard.tsx`, `ThemeCard.tsx` — small toggle cards.

**Wire-up:**
- Reads from existing `useSettings` hook.
- Writes via existing `update` for app settings; writes `.env` via `writeConfig("openclaw-vault", ".env", ...)` for key rotations.
- Save errors route through `classifyError()` (Pass 5).
- Spending limit edits update `settings.spendingLimit.monthly` only — the `$X this month` display stays at the stub from Day 1.

**Done when:**
- Preferences renders all 6 sections per `10-preferences.md` (Section 6 — Advanced Mode toggle — kept since it already works).
- Rotating the Anthropic key updates `.env` and the Hero pulls fresh state on next watchdog tick.
- Banned-term sweep clean.
- One commit, pushed.

### Day 3 — Discover

**Files to create / replace:**
- `app/src/pages/user/Discover.tsx` — replace `UserPlaceholder` with use-case gallery.
- `app/src/components/user/UseCaseCard.tsx` — single card.
- `app/src/data/useCases.ts` — verify or create the fixture (per `12-use-case-gallery.md`'s example library).

**Wire-up:**
- Each card has a "Try this" CTA that opens `settings.telegramBotUrl` with the prefilled message via `@tauri-apps/plugin-shell`'s `open()`.
- Favorite-toggle persists to `settings.favoriteUseCaseIds` (already in the AppSettings schema).
- Search box filters client-side (no backend).

**Done when:**
- Discover renders ≥10 use-cases with categories per the page spec.
- Clicking "Try this" opens Telegram with the prefilled message.
- Favorite + search both work.
- Banned-term sweep clean.
- One commit, pushed.

### Day 4 — Tip-of-the-day + proactive alerts banner

**Files to create / replace:**
- `app/src/components/user/TipOfTheDay.tsx` — new card on Home below the tiles.
- `app/src/components/user/ProactiveAlertsBanner.tsx` — new banner above the hero (conditional render).
- `app/src/hooks/useAlerts.ts` — frontend-only evaluator that observes `useHero` + `useSettings` and emits alerts (e.g., "Anthropic key not set", "Spending limit reached").

**Wire-up:**
- Tip is deterministic: `useCases[dayOfYear % useCases.length]`.
- Alerts have `id`, `severity`, `title`, `body`, `cta?`, `dismissable`. Dismissal persists to `settings.dismissedAlerts` (already in the AppSettings schema).
- Initial alert rules:
  - Anthropic key missing in `.env` → severity `danger`, CTA `Open Preferences`.
  - Telegram token missing in `.env` → severity `warning`, CTA `Open Preferences`.
  - Hero is `error_perimeter` → severity `danger`, CTA `Try to fix`.
- All other alert categories from `08-home-dashboard.md` (spending, container crashed, security audit failed) slip to Pass 7's backend evaluator.

**Done when:**
- Tip rotates daily.
- Alerts banner shows correctly when Anthropic key is removed from `.env` (manual test: edit `.env`, refresh, see the banner).
- Banner dismissal persists across app restart.
- Banned-term sweep clean.
- One commit, pushed.

### Day 5 — Security/Help placeholders + polish + re-score

**Files to update:**
- `app/src/pages/user/SecurityMonitor.tsx` — replace `UserPlaceholder` with friendlier static copy.
- `app/src/pages/user/Help.tsx` — same.

**Friendlier placeholder copy** (suggested; can iterate):

```
We're still building this section.

Your assistant is already running safely behind a sandbox perimeter — we just
haven't finished the dashboard for it yet. In the meantime, talk to your
assistant on Telegram while we finish.

[ Open Telegram ]
```

The spec-path leak (`docs/specs/ui-rebuild-2026-04-21/...`) and the "Coming in Phase E.2.X" phrasing are gone.

**Re-score:** rebuild rows 14–18 in the score matrix in `2026-04-20-ux-principles-rubric.md`. Targets:

- Row 14 (Home): from 3.8/10 → ≥8.5/10.
- Row 15 (Security placeholder): from 4.9/10 → ≥6.5/10.
- Row 16 (Discover): from 4.9/10 → ≥8.5/10.
- Row 17 (Preferences): from 4.9/10 → ≥8.5/10.
- Row 18 (Help placeholder): from 4.9/10 → ≥6.5/10.

**Cleanup:**
- Pre-existing baseline-failing E2E tests (`navigation.spec.ts`, `smoke.spec.ts`, `user-facing.spec.ts`) should re-green once the placeholders are gone. If any still fail, fix or `.skip` with a TODO pointing to the actual blocker.
- Update `docs/handoff.md` with Pass 6 done + Pass 7 spec.
- Update `project_status.md` and `project_decisions.md`.
- Final commit + push.

---

## Done-criteria (Pass 6 acceptance)

- [ ] All 5 user-mode pages no longer render `UserPlaceholder`.
- [ ] The phrase `Coming in Phase E.2.` appears nowhere in user-facing copy.
- [ ] Hero state machine reflects perimeter health within 30s of any container state change.
- [ ] Banned-term lists in `app/e2e/wizard.spec.ts` AND `app/e2e/user-facing.spec.ts` pass against all 5 surfaces.
- [ ] Pre-existing baseline-failing tests in navigation/smoke/user-facing are resolved or explicitly `.skip`-ed with a tracking link.
- [ ] Wizard E2E (4/4) + cargo lib (33/33) + orchestrator-check (41/41) all pass.
- [ ] Rubric re-score lands rows 14, 16, 17 ≥ 8.5; rows 15, 18 ≥ 6.5.
- [ ] One commit per day at minimum, all pushed to `origin/main`.

---

## Open decisions (settle as they come up)

- **Container "Pause" affordance.** The `paused_by_user` hero state needs a backend command to stop containers without recreating them, plus a way to remember the pause was user-initiated. Easy to add (one new Tauri command + a flag in `PerimeterStateStore`). If Day 4 has slack, fold this in; otherwise Pass 7.
- **Status illustrations.** The spec calls for `status-{state}.svg` × 6. Pass 6 reuses the wizard's pulsing-rings as a stand-in. If a visual designer cycle materializes mid-Pass-6, swap them in; otherwise Pass 7.
- **Sidebar nav rewrite.** The current sidebar may have codename leaks or off-spec labels. Pass 6 audits but doesn't restructure. If audit flags issues, fix in-place; if it's a bigger pattern, Pass 7.
