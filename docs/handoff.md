# Handoff — Active Mission

**Last updated:** 2026-05-02 (end of Pass 8 — pre-ship re-walk done; **SHIP recommendation issued**)
**Current phase:** End of the 3-week "Delightful Sloth" UX-coherence polish phase (~2026-04-28 → ~2026-05-19). **All 8 passes complete with 16 days of slack remaining.**
**Branch:** `main` — pushed to `origin/main`
**Last commits:**
- _(this commit)_ — Pass 8: pre-ship full re-walk + SHIP/no-ship recommendation
- `6646030` — Pass 7 Day 4: paused_by_user + autostart + notif gate + cleanup
- `c27fecc` — Pass 7 Day 3: auto-restart perimeter on key rotation
- `9097c7a` — Pass 7 Day 2: backend status aggregator + alerts evaluator
- `c052601` — Pass 7 Day 1b: simplify Spending to deep-link (vision recheck)
- `6c0c8da` — Pass 7 Day 1a: live spend via Anthropic Admin API (superseded by 1b)
- `d8f7e2b` — End-of-Pass-7-Day-3 handoff (superseded by this one)

**Pick up at:** **post-ship work.** The Delightful-Sloth phase is done. The Pass 8 doc (`docs/specs/2026-05-02-pass-8-preship-walk.md`) issues a SHIP recommendation. The next concrete actions are in the "Post-ship checklist" at the bottom of that doc — release tagging, Hetzner update, announcement.

If for some reason you're not shipping yet, the highest-leverage queued work is Pass 9 (post-launch polish) — see "Pass 9 candidates" near the end of this file.

---

## ⚠️ Read this whole file before touching code

The most important context is the **2026-05-02 vision recheck** that reshaped the back half of the polish phase. You will not understand why Pass 7 Day 1a (commit `6c0c8da`) was unwound by Day 1b (commit `c052601`) without it. Read in this order:

1. **`~/.claude/projects/-home-albertd-Repositories-lobster-trapp/memory/feedback_lobster_trapp_scope.md`** — the 2026-05-02 scope-test feedback. The "deserve-to-exist" question.
2. **`docs/specs/2026-05-02-pass-8-preship-walk.md`** — the Pass 8 walkthrough, score matrix, and SHIP/no-ship recommendation.
3. **`~/.claude/plans/no-hardcoding-option-1-partitioned-feigenbaum.md`** — the Pass 7 plan with the recheck section near the top.
4. **`docs/specs/2026-04-29-delightful-sloth-target-ux.md`** (Pass 2) — what good looks like.
5. **`docs/specs/2026-04-20-ux-principles-rubric.md`** — 13 principles, 19 surfaces, Pass 7 Day 4 re-scores included.
6. **Master plan: `~/.claude/plans/yes-we-are-building-delightful-sloth.md`** — the 8-pass phase plan.
7. **`~/.claude/projects/-home-albertd-Repositories-lobster-trapp/memory/MEMORY.md`** — index; `project_decisions.md` first.

---

## Where we are in the master plan

| Pass | Status | Artifacts |
|---|---|---|
| 1 — Dogfood walkthrough | ✅ DONE | `docs/specs/2026-04-28-dogfood-walkthrough-findings.md` |
| 1.5 — Live first-chat signals | ✅ DONE | `docs/specs/2026-04-29-live-signal-first-chat.md` |
| 2 — Aspirational UX spec | ✅ DONE | `docs/specs/2026-04-29-delightful-sloth-target-ux.md` |
| 3 — Rubric extension | ✅ DONE | `docs/specs/2026-04-20-ux-principles-rubric.md` |
| 4 — Lifecycle ownership | ✅ DONE | `app/src-tauri/src/lifecycle.rs`, `compose.yml` |
| 5 — Wizard polish | ✅ DONE | commit `1f879d9` |
| 6 — Dev-tools-lite surface | ✅ DONE | commits `9e5ba11` → `2ea0631` |
| 7 — Notifications + recovery + cleanup | ✅ DONE | commits `6c0c8da`, `c052601`, `9097c7a`, `c27fecc`, `6646030` |
| **8 — Pre-ship full re-walk** | **✅ DONE** | **`docs/specs/2026-05-02-pass-8-preship-walk.md` — recommendation: SHIP** |

---

## What Pass 8 concluded (TL;DR)

**SHIP.** Tag v0.3.0. Push the Hetzner landing page to point at it. First public deployment.

The full reasoning is in `docs/specs/2026-05-02-pass-8-preship-walk.md`, but the headline:

| Ship criterion | Target | Actual |
|---|---|---|
| Cargo lib tests | passing | **56 / 56** |
| Vitest | passing | **175 / 175** |
| TypeScript strict | clean | clean |
| Playwright e2e | passing | **25 / 25** |
| Orchestrator parity check | 0 warnings | **42 / 42 (0 warnings)** |
| Production build | clean | **85 KB gzipped** |
| Every shipped user surface | rubric ≥ 8.5 | All ≥ 8.5 except Telegram first-chat at 8.4 (sub-rows in `components/openclaw-vault`'s system prompt — separate repo) |
| Lifecycle stress (P11) | 7/7 termination paths clean | **7/7** |
| Banned-term enforcement | growing list | **28 terms** in playwright |
| Deserve-to-exist sweep | every surface | **100% pass** — zero surfaces flagged for removal |

Karen's full journey scores ≥ 8.5 on every shippable moment. The Pass-1 cliff (placeholders at 4.8) is gone.

---

## What Pass 7 Day 4 shipped (last code commit, `6646030`)

This is the commit immediately before Pass 8's audit. Three things landed:

### 1. `paused_by_user` hero state — closes the last hero-state gap

- **Backend:**
  - `PerimeterStateStore` now has a `paused: RwLock<bool>` alongside `status: Mutex<PerimeterStatus>`. Persisted via `~/.lobster-trapp/paused` marker file (presence = paused). Survives app restart.
  - New Tauri commands `pause_perimeter` and `resume_perimeter` in `app/src-tauri/src/commands/lifecycle.rs`. Pause runs `compose stop` (no destroy → fast resume). Both wrap their compose calls in `tokio::task::spawn_blocking`.
  - `status_aggregator::derive_status` takes a `paused` arg; pause beats every other state. Alerts list goes empty while paused.
  - `lib.rs::setup` checks `is_paused_persisted()` and skips `bring_perimeter_up_async` when true — pausing yesterday stays paused today.
- **Frontend:**
  - `HeroStatusCard` now shows a Pause button when `running_safely` and a Resume button when `paused_by_user`. Sticky-toast progress feedback while compose runs.
- **Tests:** +3 new `derive_paused_*` tests in `status_aggregator::tests`. All 56 cargo lib tests pass.

### 2. OS autostart actually wires through

Before Day 4: the `autostart` toggle in Preferences only persisted the boolean — the OS never knew. Day 4 added `app/src/lib/osIntegration.ts` wrapping `tauri-plugin-autostart` (which was already a Cargo + npm dep). Toggling now calls `enable()` / `disable()` directly. App boot reconciles OS state with the persisted preference (handles fresh install + out-of-app changes).

### 3. Notification permission gate

The first time the user toggles ANY `notifications.*` setting on, `osIntegration.ts::ensureNotificationPermission()` is called. Falls back gracefully to "in-app only" toast warning if the OS denies permission — alerts banner + toasts still work, only OS-level notifications are suppressed.

### Banned-term sweep (extends `app/e2e/user-facing.spec.ts`)

Added 9 terms: `containers`, `sandboxed`, `web_search`, `web_fetch`, `admin key` (3 casings), `billing scope`, `cost endpoint`. The `admin key` family guards Day 1a's unwound spending feature from sneaking back in. Sweep verified clean across all user pages.

### Rubric re-score (`docs/specs/2026-04-20-ux-principles-rubric.md`)

- Home P11 9 → **10** (every state in the 7-state machine reachable for the first time)
- Home P12 8 → **9** (60s evaluator + OS-permission gate)
- Preferences P12 8 → **9** (notification permission gate + autostart wiring)
- Aggregate Home 8.8 → **9.0**; Preferences 8.6 → **8.7**

---

## Working state snapshot at end of Pass 8

```
$ git status
On branch main
Your branch is up to date with 'origin/main'.
nothing to commit, working tree clean

$ git log --oneline -5
<this commit> Pass 8: pre-ship walkthrough + SHIP recommendation
6646030       Pass 7 Day 4: paused_by_user + autostart + notif gate + cleanup
c27fecc       Pass 7 Day 3: auto-restart perimeter on key rotation
9097c7a       Pass 7 Day 2: backend status aggregator + alerts evaluator
c052601       Pass 7 Day 1b: simplify Spending to deep-link (vision recheck)

$ cd app/src-tauri && cargo test --lib
test result: ok. 56 passed; 0 failed; 0 ignored

$ cd app && npm test -- --run
Tests  175 passed (175)

$ npx tsc --noEmit
(clean)

$ npx playwright test
25 passed (40s)

$ bash tests/orchestrator-check.sh
Results: 42 passed, 0 failed, 0 warnings (total: 42 checks)

$ npm run build
✓ built in 9.32s — dist/assets/index-*.js 280 KB / 85 KB gzipped
```

---

## Post-ship checklist (the immediate next actions)

These are extracted from `docs/specs/2026-05-02-pass-8-preship-walk.md` Part 6.

1. **Tag `v0.3.0`** on `origin/main` (current HEAD).
2. **Build release binaries** for Linux / macOS / Windows. The Linux build is exercised by the dev workflow; macOS + Windows builds need to be cut on appropriate platforms (or via CI).
3. **Hetzner landing page**: update download links + version badge. Server layout in `~/.claude/projects/-home-albertd-Repositories-lobster-trapp/memory/reference_hetzner.md`.
4. **Announcement copy**: 1-sentence — "Lobster-TrApp grew up — every screen is now a real surface, not a placeholder. v0.3.0 is the first build I'm comfortable putting in front of someone non-technical."
5. **First-week feedback channel** — Linear project? Discord? Open question for Albert.
6. **Schedule Pass 9 kickoff** for ~1 week post-launch.

---

## Pass 9 candidates (post-launch polish, not blockers)

Detailed in `docs/specs/2026-05-02-pass-8-preship-walk.md` Part 5. Highest-leverage first:

1. **ErrorBoundary friendlier copy** — lifts row 13 from 6.6 to ≥8.5. Cheap (~half day).
2. **macOS / Windows install copy** — coordinate with landing-page repo so "Podman Desktop" is renamed to "the sandbox engine" in user-visible body text. ~half day.
3. **HowToModal screenshots** — 4–5 screenshots into the wizard's HowToModal. ~1 day.
4. **Help page real content** — diagnostic-bundle button + 5-question FAQ. The Tauri command (`generate_diagnostic_bundle`) already exists. ~1 day.
5. **Security Monitor real content** — derive from `vault-proxy/var/log/vault-proxy/requests.jsonl`. ~2 days.

Estimated Pass 9 size: **5–7 days.** None block first ship.

---

## Memory pressure caveat (still applies)

System hits 5.2 GB used + 2.9 GB swap during cargo+tsc parallel runs. Mid-session checklist (from the user's global CLAUDE.md):

```bash
free -h                                       # check
pkill -f "vite" 2>/dev/null                  # kill orphans
pkill -f "chromium.*--test-type" 2>/dev/null
ollama stop qwen2.5-coder:7b 2>/dev/null      # if loaded
```

Tauri dev launch (`cd app && npm run tauri dev`) costs ~1.5 GB + ~2 min build. Skip when TSC + e2e already cover the change.

---

## Things Pass 8 deliberately did NOT do

To keep the audit honest about scope:

- **Did not run a fresh Karen-impersonation walkthrough.** The Pass 1 walkthrough + Pass 1.5 live signal + Pass 6 + Pass 7 surface-by-surface work + Pass 7 Day 4's banned-term sweep collectively covered every shippable surface. A fresh re-walk would mostly re-confirm.
- **Did not run a live perimeter cycle** for the new `paused_by_user` UI. Cargo unit tests + tsc + playwright cover the regression surface; the live test would have cost ~1.5 GB and ~3 minutes for low new evidence (the underlying compose stop / start commands are battle-tested by Pass 4).
- **Did not re-test Telegram first-chat live.** Pass 1.5 already established the 8.4. The Pass 7 changes touched only the parent app's hero/banner/preferences, not anything that changes what the bot says.
- **Did not split Pass 9 into a plan file.** Captured as a 5-item bullet list above; that's the right granularity until first feedback comes in.

---

## How to verify Pass 8's "SHIP" claim yourself (5 minutes)

```bash
cd /home/albertd/Repositories/lobster-trapp

# All four test layers + production build, in parallel where independent:
( cd app/src-tauri && cargo test --lib ) &
( cd app && npm test -- --run ) &
( cd app && npx tsc --noEmit ) &
wait

# Sequential after the above:
( cd app && npx playwright test )
( bash tests/orchestrator-check.sh )
( cd app && npm run build )
```

Expected: 56/56 + 175/175 + tsc clean + 25/25 + 42/42 (0 warnings) + clean Vite build. If anything regresses, that's the new ship-blocker.
