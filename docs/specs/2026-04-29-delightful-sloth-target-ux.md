# Delightful Sloth — Aspirational UX Target

**Date opened:** 2026-04-29
**Author:** Claude (Opus 4.7)
**Status:** Pass 2 of the 3-week "Delightful Sloth" polish phase
**Predecessors:** Pass 1 dogfood (`docs/specs/2026-04-28-dogfood-walkthrough-findings.md`); Pass 1.5 live first-chat (`docs/specs/2026-04-29-live-signal-first-chat.md`)
**Master plan:** `~/.claude/plans/yes-we-are-building-delightful-sloth.md`
**Builds on:** `docs/specs/2026-04-19-product-identity-spec.md` (extended, not replaced); `docs/specs/2026-04-20-ux-principles-rubric.md`; `docs/specs/ui-rebuild-2026-04-21/user-mode/{07-12}.md`

---

## Why this spec exists

Pass 1 walked Karen's 8 user moments and produced a friction punch-list. Pass 1.5 ran the live Telethon harness and revised Moment 3 from 5.5 → 8.0. We now know with evidence:

- The wizard is **already** at 9.5/10.
- The bot voice is **already** at ~8.0/10 — friendlier than feared.
- The cliff between Telegram (8.0) and the post-wizard pages (4.8) is **steeper, not shallower**, than code-reading suggested.
- The lifecycle gap (app ≠ perimeter) is real and load-bearing for everything after.

What's missing isn't the *current state* (we have two findings docs for that) and isn't the *page-level design* (the `ui-rebuild-2026-04-21/user-mode/` specs cover Home, Security, Discover, Preferences, and Help in detail). What's missing is the **target-state synthesis** — the single document that says, for each of the 8 user moments, "this is what good looks like" and, for the four cross-cutting behaviors that don't live on any single screen (lifecycle, notifications, dev-tools-lite, crash recovery), the rules that bind those screens together.

This spec is that document. It doesn't redesign anything. It names the target each Pass 4–7 implementation must hit so we can score Pass 8 against something.

---

## What this spec adds (vs the existing canon)

| Existing doc | What it covers | What this spec adds |
|---|---|---|
| `2026-04-19-product-identity-spec.md` | Mission, persona, dev→user translation, 3-screen architecture, copy patterns | Concrete 8-moment journey rules + the four cross-cutting behaviors that the identity spec doesn't yet name. |
| `2026-04-20-ux-principles-rubric.md` | 10 principles for scoring screens | Three new principles (P11–P13) emerging from Pass 1+1.5 to ratify in Pass 3. |
| `ui-rebuild-2026-04-21/user-mode/{07–12}.md` | Page-level specs for wizard, Home, Security, Preferences, Help, Discover | "When does each page deserve to exist live?" + the lifecycle/notification rules the page mocks assume but never name. |
| Pass 1 + Pass 1.5 findings docs | Friction punch-list + live evidence | "What good looks like" — the inverse of the punch-list. |

This spec is therefore short on novel UI design (the ui-rebuild specs already did that work) and long on **rules and acceptance criteria** for behaviors that span screens.

---

## Three new principles to ratify in Pass 3

The 10-principle rubric scored well in Passes 1 + 1.5, but three behaviors emerged that no current principle quite covers. These are **proposed for Pass 3** (the rubric extension) — listed here so Pass 4–7 can target them now.

### P11 — The perimeter is alive iff the app is alive

**Rule:** App start ⇒ perimeter up. App close ⇒ perimeter down. App crash ⇒ perimeter cleanly torn down (no orphan containers). OS restart ⇒ app autostart respected; perimeter follows app, not the other way around.

**Why:** Pass 1 Phase 1 audit confirmed the app today is a control panel, not a lifecycle owner. SIGKILL leaks containers indefinitely; closing the app leaves containers running with no UI to manage them; relaunching the app shows "Your assistant is ready" even when the perimeter is offline. The product premise (a silent security perimeter that owns its own runtime) demands this invariant.

**How to score:** Stress-test (Pass 4 verification) — kill the Tauri process; containers must die within 5s. Reopen the app; perimeter must come back without user action. Kill one container while app running; watchdog must restart it within 30s.

### P12 — The app speaks only when it's helping

**Rule:** Notifications fire on **events that change Karen's options** — never on routine internal state changes. Three valid trigger categories:
1. **Threat-blocked** ("Blocked an attempt to read your address book — that skill won't be allowed to do that")
2. **State-recovery** ("Your assistant came back online" — only after a degraded state)
3. **Action-required** ("Anthropic key expired — open Preferences to update")

Forbidden: "Component vault-agent transitioned to state RUNNING," "Health check passed," "Container restarted," any notification Karen can't act on or doesn't care about.

**Why:** Pass 1 noted no notification system exists today; the tray placeholder says "initializing" and never updates. Vision quote (2026-04-26): "the app might give little updates and notifications once in a while to inform the enduser if something happens (injection attack prevented)." The opposite trap is over-notifying — the goal is "silent middleman," not "chatty middleman."

**How to score:** A 24-hour idle test should produce **zero** notifications. A skill-install test should produce **one** ("Skill safe to install"). A simulated injection block should produce **one** ("Blocked an attempt to..."). A perimeter restart should produce **one** state-recovery notice if and only if the perimeter was previously degraded.

### P13 — Dev-tools-lite surfaces feel native, not embedded

**Rule:** When Karen reaches for a power-user task (peek at activity, install a skill, download from the network), the surface that appears is **first-class** — same visual language, same copy voice, same layout grammar as the Home dashboard. It is NOT an iframe, a console window, an "Advanced" tab styled differently, or a wrapped CLI dump.

**Why:** Pass 1 found that the post-wizard sidebar destinations (Security, Discover, Preferences, Help) are placeholders rendering "Coming in Phase E.2.X" with raw spec paths. The architecture-driven temptation will be to "expose the manifest workflow" by showing the workflow output verbatim. The user-driven answer is that the workflow is invisible plumbing; the user sees only their question being answered (e.g., "this skill is safe to install — proceed?").

**How to score:** Take any dev-tools-lite page. Replace its title with the Home page's title. Same fonts, same colors, same copy voice? It passes. If a developer-affordance (raw stream, JSON dump, hex hash) is visible without explicit "Show technical details" disclosure, it fails.

---

## The four cross-cutting target behaviors

### Cross-cutting #1 — Lifecycle ownership

**Target:** P11. The app and perimeter share a single lifetime.

**What Karen sees today:** Nothing tells her the perimeter is up or down. The wizard's Ready screen says "your assistant is ready" once, and then forever after she has to infer state from whether her bot replies on Telegram. Pass 1 line 348 documented this gap explicitly.

**What good looks like:**

- **App launches** → splash for ≤2s while perimeter is brought up (or detected). Hero state on Home dashboard moves from `not setup` / `error` → `running safely` once health checks pass.
- **App closes** (window close, menu quit, OS log-out) → graceful `podman compose down`; brief progress toast "Putting your assistant to sleep…"; window stays open during shutdown.
- **App crashes** (SIGKILL, OOM, Tauri panic) → signal handler / `RunGuard` / supervised shutdown ensures containers die within 5s. The user has no UI to see this happen — it's a background contract.
- **OS reboot** → respect the `Start when computer boots` Preferences toggle (per `10-preferences.md` section 4). If on, app autostarts; perimeter follows. If off, Karen launches manually; perimeter follows.
- **One container dies** (vault-agent OOM, vault-proxy crash) → background watchdog detects within 30s, restarts. Hero status flips to amber `Recovering — your assistant is restarting itself…` and back to green when probes pass. Notification fires per P12 only if the recovery takes >60s OR happens twice in 5 minutes.
- **External tampering** (user runs `podman compose up` outside the app) → app detects, unifies state, **does not** kill the user's running compose stack. Hero shows neutral copy: "Detected your assistant is already running — leave it as-is."

**The hero state machine on Home dashboard** (per `08-home-dashboard.md` plus this spec):

| State | When | Hero illustration / copy | Primary action | Secondary action |
|---|---|---|---|---|
| `running_safely` | All health probes green; perimeter up; key valid | Calm lobster, steady glow. "Your assistant is running safely." | "Open Telegram" | "Pause" |
| `paused` | User-initiated pause; containers stopped on purpose | Sleeping lobster. "You paused your assistant. It's not running right now." | "Wake it up" | — |
| `recovering` | Watchdog is restarting one or more containers | Lobster with wrench. "Your assistant is taking a moment — back in a few seconds." | (disabled) "Try to fix" | "Show details" |
| `error_perimeter` | Perimeter down, repeated restart failures | Lobster with bandage. "Your assistant ran into trouble starting up. Let's try together." | "Try to fix" | "Get help" |
| `error_key` | API key missing / invalid / no credit | Lobster waiting. "Your assistant needs an updated key to think." | "Update key" (→ Preferences) | — |
| `not_setup` | Wizard not completed yet (first launch fallback) | Welcoming lobster. "Let's get your assistant set up." | "Start setup" (→ /setup) | — |

**Rule for P11 verification:** A `Pause` from this list always succeeds; a `Wake it up` always either succeeds or transitions to `error_perimeter` with one clear action.

### Cross-cutting #2 — Notification surface

**Target:** P12. The app speaks only when it's helping.

**Three trigger categories** with examples:

| Category | When | Example copy | Loudness |
|---|---|---|---|
| **Threat-blocked** | vault-proxy logs an `EXFIL_BLOCKED` or `BLOCKED` event AND the request was initiated by the agent (not by Karen) AND the destination would have been observably dangerous | "Blocked an attempt to upload your address book — your assistant won't be allowed to do that." | OS notification (sticky 8s) + persistent in-app banner on Home until dismissed |
| **State-recovery** | Perimeter went `error_perimeter` and recovered to `running_safely`; OR a single container had to be restarted ≥2× in 5 minutes | "Your assistant came back online." | OS notification (transient 4s); no in-app banner |
| **Action-required** | API key invalid OR Anthropic credit balance error returned by upstream OR the OS denied a permission Lobster-TrApp needs (notification, autostart) | "Your assistant key isn't working — open Preferences to update." | OS notification (sticky) + in-app banner with `Update key` button |

**Forbidden** (must never fire):

- Container restarts that resolve in <60s and weren't repeats
- Health-probe pass/fail transitions
- Anything with a component name (`vault-agent`, `clawhub-forge`)
- Anything with a numerical diagnostic (`exit code 137`, `HTTP 503`)
- Anything Karen can't act on within Lobster-TrApp's UI

**Default volume:** All three trigger categories ON by default. Karen can disable each individually in Preferences (`10-preferences.md` section 3) but the defaults respect P12 (the app earns the right to speak by being mostly silent).

**Notification permission gate:** First time the app would fire any notification, prompt OS permission. If Karen denies, fall back to in-app banner-only for `action-required`; threat-blocked and state-recovery silently log to the activity timeline (Security page) instead.

### Cross-cutting #3 — Dev-tools-lite surface

**Target:** P13. Native, not embedded.

The five user-mode sidebar destinations exist for **five questions** Karen actually asks. The dev-tools-lite framing is what unifies them: when Karen wants to peek at activity, install a skill, download something safe, change a setting, or get help, the answer must look like part of the same product.

| Sidebar destination | Karen's actual question | "Native" answer | "Embedded" anti-pattern (FORBIDDEN) |
|---|---|---|---|
| Home | "Is my assistant OK?" | Hero status card per the matrix above | Container status grid; service uptime tickers |
| Security | "What has my assistant been doing? Anything dangerous?" | Plain-language activity timeline + 0–2 incidents card | `vault-proxy.jsonl` tail; raw allowlist YAML editor |
| Discover | "What can I ask?" | Use-case gallery cards, deep-link to Telegram per `12-use-case-gallery.md` | Raw skill manifest browser; "available tools" list |
| Preferences | "Change something" | 6-section preferences per `10-preferences.md` | `.env` editor; manifest schema viewer |
| Help | "I'm stuck" | FAQ grid + diagnostic-bundle button per `11-help-and-support.md` | Log dump; GitHub issues iframe |

**The single rule connecting these:** the page deserves to exist **only when** it can answer Karen's question in her language, with her data. Until then, it's a *friendlier placeholder* (see Moment 4–7 below) — never a "Coming in Phase E.2.X" + raw `docs/specs/...` path.

**Dev mode is separate.** All ten dev-mode pages (DevAllowlist, DevComponents, DevLogs, etc.) ARE the embedded surface — that's their job. Karen never reaches them unless she explicitly enables Advanced Mode (per Preferences section 6). The toggle is one-way warm — flipping to Advanced shows a brief "Power-user mode unlocked" dialog with a "go back" affordance; flipping back is silent.

### Cross-cutting #4 — Crash & recovery posture

**Target:** P3 (errors guide) + P11 (lifecycle invariant) intersected.

**Three crash classes** with target experiences:

#### Class A — App crash with perimeter still up

Karen reopens the app. Perimeter is mostly fine (one or two containers may have grown stale). The app:

1. Boots in <2s.
2. Detects that the perimeter is already up (containers exist, named the same).
3. Runs health probes silently. If green: hero goes directly to `running_safely`, no banner — Karen sees "Your assistant is running safely" as if nothing happened.
4. If amber: hero goes to `recovering`, banner: "Your assistant is taking a moment — back in a few seconds." Auto-clears when green.
5. If red: hero goes to `error_perimeter`, banner: "Your assistant didn't fully recover. Let's restart it together." with a `Try to fix` button.

No mention of the prior crash. Karen doesn't need a "we crashed last time" notice to be informed; she needs the system to be running.

#### Class B — App crash with perimeter taken down

(SIGKILL → RunGuard tore down containers per P11.) Karen reopens. Hero starts in `not_setup` → quickly transitions through splash → `running_safely`. If startup fails: lands in `error_perimeter` with the same `Try to fix` flow.

A **state-recovery** notification per P12 fires only if the previous session ended in a way the app could detect (e.g., a journal file written before SIGTERM): "Your assistant is back online — last session ended unexpectedly." Otherwise silent.

#### Class C — Compute crash (laptop lid closed mid-task, OS suspend, hibernate)

Karen reopens lid. Containers may be in weird states (TCP connections dropped, in-flight Telegram poll lost). The app:

1. Notices the wall-clock jump on resume.
2. Forces a health-probe sweep within 5s.
3. Restarts any container that fails its probe (per the watchdog).
4. Hero transitions through `recovering` → `running_safely` if all OK.

No notification unless recovery takes >60s. Karen's Telegram chat resumes seamlessly from the bot's perspective (assuming the bot is built to handle this — out of parent-repo scope but worth flagging to `components/openclaw-vault`).

---

## What good looks like — 8 user moments

Each moment defines: target score (≥, where the rubric goes), cardinal experiences (what Karen feels), surfaces (which files own this), and acceptance signals (how Pass 8 verifies).

### Moment 1 — Discovery (lobster-trapp.com)

**Target score:** ≥8.5/10 (was 6.2 in Pass 1; landing page was the lowest-scoring user-facing surface).

**Cardinal experiences:**
- Karen reads the headline ("Your own AI assistant, safe on your computer") and the value prop lands in <8s.
- She sees the trust list and feels reassured *without* needing to know what a sandbox is.
- The diagram shows three concepts — Your Assistant / Skill Safety / Network Safety — never `Forge / Pioneer / Vault`.
- The download CTA either bundles the perimeter runner OR reassures her the wizard handles it. The phrase "Requires Podman or Docker installed on your system" never appears as a precondition.

**Surfaces:** `docs/index.html` (Hetzner-deployed static site).

**Acceptance signals (Pass 8):**
- Codename leak count = 0 in user-visible text. The 19-term GUI banned list (`app/e2e/user-facing.spec.ts:13-33`) plus this spec's additions — `container` (bare), `sandboxed`, `Podman`, `Docker`, `Forge`, `Pioneer`, `Vault` (when used as codenames), `web_search`, `web_fetch`, `tool_use` — should pass against the rendered HTML.
- Download links resolve to a working artifact OR to a "Sign up for early access" form during soft-launch.
- The "no terminal required" promise is consistent throughout — no later line in the page contradicts it.

**Out of scope this pass:** Re-shooting the hero illustration; reworking the trust-claim copy beyond term replacement.

### Moment 2 — First-run install + wizard

**Target score:** ≥9.5/10 across all four screens (was 9.5 already in Pass 1; we're polishing, not rebuilding).

**Cardinal experiences:**
- 3-minute time budget, achieved.
- Welcome → Connect → Install → Ready, no surprises.
- Per-step real-time copy ("Check your computer…" / "Building your assistant…" / "Test safety checks").
- Failures surface gracefully via `classifyError()` and the FriendlyRetry / ContactSupport ladder.
- The MissingRuntime card never says `sudo apt install podman podman-compose` directly — it offers a one-click install OR a "We'll guide you through it (2 min)" walkthrough.

**Surfaces:** `app/src/pages/Setup.tsx`, `app/src/components/wizard/{Welcome,Connect,Install,Ready}Step.tsx`, `app/src/lib/{errors,wizardUtils}.ts`.

**Acceptance signals (Pass 8):**
- All four wizard E2E tests in `app/e2e/wizard.spec.ts` pass with the extended banned-term list.
- The three thrown errors flagged in Pass 1 (`InstallStep.tsx:140-145`, `:204`, `:260`) route to specific `classifyError` patterns, not the `UNKNOWN_FALLBACK`.
- The `UNKNOWN_FALLBACK` itself becomes context-aware (per sub-step running): `Your computer check didn't work as expected.` not `Something didn't work as expected.` (rubric anti-pattern).
- The Telegram URL prefetch failure mode shows Karen her bot username as a fallback if `deriveTelegramBotUrl()` returns null, instead of opening generic `telegram.org`.
- A live-running validation sub-pass during Pass 5 OR Pass 8 confirms the wizard ≥9 across all four screens with real install timing (the dynamic-friction Pass 1 acknowledged it couldn't capture).

### Moment 3 — First chat (Telegram)

**Target score:** ≥9/10 (was 5.5 from code-reading; ~8.0 from Pass 1.5 live evidence; the gap to 9 is closing the three new P0s Pass 1.5 surfaced).

**Cardinal experiences:**
- Karen sends `/start` and sees a warm greeting that mirrors the `help` reply already in the bot's training (the voice exists; it just doesn't fire on first contact).
- Capability copy translates internal terminology ("sandbox", "container", "web_search/web_fetch tool") to plain English.
- Graceful failures stay graceful — the live evidence here is excellent and is the bar to maintain.

**Surfaces:** Mostly `components/openclaw-vault` (system prompt) — flagged for the submodule maintainer. The parent repo owns:
- `ReadyStep.tsx:94-97` example-prompts hint card (now load-bearing because the bot doesn't greet warmly itself)
- The Telegram URL prefetch failure path (Moment 2 surface)
- The banned-term list extension (`app/e2e/user-facing.spec.ts`)

**Acceptance signals (Pass 8):**
- Re-run `tests/e2e-telegram/test_ux_first_chat.py` after `components/openclaw-vault` system-prompt updates ship; banned-term hit count drops to 0.
- Median first-byte latency stays ≤6s; p95 ≤12s.
- A fresh-Telegram-account pairing test (separate from the existing pre-paired harness account) confirms the pairing-gate friction Pass 1 line 399 flagged is either resolved or surfaced to Karen with a one-line hint in the wizard.

### Moment 4 — Returning use

**Target score:** ≥8.5/10 (was ~4.8 in Pass 1 — placeholder Home page).

**Cardinal experiences:**
- Karen reopens the app days later. Within 2s she sees `Your assistant is running safely. It's been active for 2 hours today.` (or whatever the current activity summary is).
- The hero state machine carries her through any in-flight recovery (Class A or C crash above) without her noticing.
- Three stat tiles (Security / Activity / Spending) give her a 3-second status read.
- A single optional tip or alert banner appears only if there's something worth her attention.

**Surfaces:** `app/src/pages/user/Home.tsx` (currently `UserPlaceholder`); design at `docs/specs/ui-rebuild-2026-04-21/user-mode/08-home-dashboard.md`. Backend additions per that spec: activity tracking, spending aggregation, status aggregation.

**Acceptance signals (Pass 8):**
- `Home.tsx` no longer renders `UserPlaceholder`. The phrase `Coming in Phase E.2.` appears nowhere on the page.
- `spec: docs/specs/...` paths appear nowhere on the page.
- Hero state transitions are testable via a Tauri integration test that injects each of the 6 states and verifies the rendered text + button visibility.
- Visiting Home with the perimeter offline shows `error_perimeter`, NOT a stale `running_safely`.

### Moment 5 — Monitoring peek

**Target score:** ≥8/10 (was ~4.8 — placeholder).

**Cardinal experiences:**
- Karen clicks Security in the sidebar curious about what her assistant has been up to.
- She sees a plain-language activity timeline (`📬 Reply sent. 13:42` / `🌐 Checked weather.com. 13:41` / `💭 Thought about "plan my Tuesday". 13:40`), grouped per conversation.
- A safety summary at top: `24 checks passed today. Nothing suspicious.` (or the incident card for the rare exception).
- The page never says `vault-proxy`, `mitmproxy`, `seccomp`, `BLOCKED`, `EXFIL_BLOCKED` — those are translated to Karen's vocabulary.

**Surfaces:** `app/src/pages/user/SecurityMonitor.tsx`; design at `09-security-monitor.md`. Data source: the existing vault-proxy event log (which already exists and is logged per `project_decisions.md:100-112`).

**Acceptance signals (Pass 8):**
- Page renders ≥1 real activity event from the vault-proxy log within 3s of mount.
- Date filter (Today / Week / Month) works.
- A planted `BLOCKED` event from the security harness shows up in the incidents card with the right humane copy ("Blocked an attempt to visit malicious-ad.net — not on your trusted list") within 5s.
- Allowlist chips display the ≥3 most-recent additions; "Show all" expands.

**Pass 6 rebudget note:** This is the page where the case for "ship it real, not a friendlier placeholder" depends on whether vault-proxy log → activity-event translation can be done in Pass 6's budget. The infrastructure exists; the translation layer doesn't. Estimate: 1.5 days alone for this page.

### Moment 6 — Add a tool from the openclaw network

**Target score:** ≥8/10 (was ~4.8 — placeholder; capability exists in `clawhub-forge`).

**Cardinal experiences:**
- Karen wants the bot to do something new (e.g., manage her email).
- She finds the affordance via Discover ("Try this — Manage email") OR via a sidebar entry called something like `Get more abilities` (NOT `Add a skill from the OpenClaw network` — codename leak).
- The browse flow surfaces 5–10 relevant abilities with a one-line plain-English description each.
- Selecting one opens a guided modal: "We'll check this is safe to install (about 30 seconds)" → forge scan runs → result is **one** of three outcomes, each with humane copy:
  - `Safe to install` (green) → "Install" button
  - `Concern found` (amber) → "Here's what we found: [plain-English summary]. Install anyway?" with a "More details" disclosure for the technical reader
  - `Blocked — known malware pattern` (red) → "We won't let you install this. [plain-English reason.] [Browse alternatives.]"

**Surfaces:** New page TBD (not in the current 5 sidebar items). Could be folded into Discover's `Try this` flow OR added as a 6th sidebar item. Pass 6 decides. Backend wraps the existing `forge_install` workflow.

**Acceptance signals (Pass 8):**
- A guided install of a known-good skill takes <2 minutes from "Karen finds the affordance" to "skill is usable from Telegram."
- The forge stream output (raw scan log) is hidden behind `Show technical details`.
- The three outcome states each have their own E2E test in `app/e2e/`.

### Moment 7 — Download from the openclaw network

**Target score:** ≥8/10 (was ~4.8 — placeholder; same pattern as Moment 6).

**Cardinal experiences:** Same shape as Moment 6 — guided "we'll check this is safe to download" flow wrapping the existing forge `safe-download` workflow.

**Practical decision:** Until Pass 6, **collapse Moment 7 into Moment 6** in the UI. Karen doesn't distinguish "install a skill" from "download a thing from the network" — both are "I want my assistant to be able to do X." Folding them keeps the dev-tools-lite surface from over-multiplying. Reopen Moment 7 as a separate moment only if user research reveals Karen actually thinks of them differently.

### Moment 8 — Crash & recovery

**Target score:** ≥8/10 (was ~1 — Pass 4 territory).

**Cardinal experiences:**
- Karen kills the app (force-quit, OS reboot, lid-close). She reopens it. Within 5s the hero is back to `running_safely` and her bot replies to the first Telegram message she sends post-recovery.
- If the perimeter didn't come back cleanly, she sees the `recovering` or `error_perimeter` state with the right `Try to fix` action — never silent failure.
- A `state-recovery` notification fires only when there was something Karen would have noticed (extended downtime, repeated container restarts).

**Surfaces:** `app/src-tauri/src/lib.rs` (lifecycle hooks), new `app/src-tauri/src/lifecycle/` (RunGuard, signal handlers, watchdog), `app/src-tauri/src/commands/health.rs` (background watchdog), `compose.yml` (`restart: unless-stopped` on `vault-agent` + `vault-proxy`).

**Acceptance signals (Pass 4 verification → carried into Pass 8):**
- SIGKILL → containers die in ≤5s. Verified via `tests/orchestrator-check.sh` extended.
- App relaunch → hero reaches `running_safely` in ≤10s on a warm perimeter, ≤30s on a cold one.
- `podman kill vault-agent` while app running → watchdog restarts within 30s; hero transitions amber→green automatically.
- OS reboot with `Start when computer boots` ON → app + perimeter both come up; user wakes laptop to a working assistant.

---

## Cross-references (don't duplicate; link)

| Need | Go to |
|---|---|
| The original mission, persona, and copy patterns | `docs/specs/2026-04-19-product-identity-spec.md` |
| Per-page detailed component design (Home, Security, Discover, Preferences, Help) | `docs/specs/ui-rebuild-2026-04-21/user-mode/{08–12}.md` |
| The 10 scoring principles + Pass 1.5 banned-term hits to add | `docs/specs/2026-04-20-ux-principles-rubric.md` (extend per Pass 3) |
| The architecture (perimeter, trust tiers, defense-in-depth, ownership matrix) | `docs/trifecta.md` |
| The data sources for activity, spending, security telemetry | `09-security-monitor.md` "Data sources needed" + `08-home-dashboard.md` backend addenda |
| Current frictions to fix (Pass 1 + 1.5 punch-lists) | `docs/specs/2026-04-28-dogfood-walkthrough-findings.md`; `docs/specs/2026-04-29-live-signal-first-chat.md` |
| Banned-term enforcement | `app/e2e/user-facing.spec.ts:13-33` (extend per Pass 1.5 P0s) |

---

## Open questions for Pass 3 (rubric extension) and the Pass 6 rebudget

These are not decided here. Pass 3 ratifies P11–P13 against actual screens. The Pass 6 rebudget question still needs the user's call.

### For Pass 3

- Are P11–P13 the right shape, or should they fold into existing principles? (E.g., is "perimeter alive iff app alive" really a P3 corollary about errors-guide-next-action?)
- Re-score the existing 13 scored screens against P11–P13. Which ones drop? Which stay?
- The rubric anti-pattern list (`docs/specs/2026-04-20-ux-principles-rubric.md:311`) called out "Something went wrong" — extend with the Pass 1.5 named offenders: bare `container`, `web_search/web_fetch tool`, terse `/start` reply, `Coming in Phase E.2.X`, `spec: docs/specs/...`.

### For the Pass 6 rebudget (still pending user decision)

| Option | Implication for this spec |
|---|---|
| **A** — Ship 5 user pages real (~9 days) | All five Moment 4–7 acceptance signals achievable. Squeezes Pass 7. |
| **B** — Ship Home + Discover + Preferences real; Security + Help friendlier placeholders (~5 days) (recommended in Pass 1.5) | Moments 4 + 6 + 7 achievable; Moments 5 + Help drop to "page exists, says 'still building this — talk to your assistant on Telegram in the meantime.'" |
| **C** — Ship Home only real; 4 friendlier placeholders (~3 days) | Only Moment 4 achievable. Moments 5–7 stay as friendlier-placeholder. Original budget holds. |

For all three, the **friendlier placeholder** target replaces today's:

> Coming in Phase E.2.3
> spec: docs/specs/ui-rebuild-2026-04-21/user-mode/09-security-monitor.md

with copy like:

> We're still building this section. In the meantime, talk to your assistant on Telegram — it'll know what's going on. [Open Telegram]

No phase codes. No spec paths. One concrete fallback action.

---

## Out of scope (locked per master plan)

- External-AI coordinator via MCP (Claude Desktop / Claude Code / Gemini CLI integration) — deferred to v0.3+
- New capability sidecars: vault-calendar, vault-voice, vault-email
- Backend orchestrator architecture changes (engine is solid; Pass 4 adds lifecycle hooks but does not restructure)
- Manifest schema changes
- Mobile app (Telegram is the mobile interface)
- `components/moltbook-pioneer` user-mode UI (target API still acquired by Meta)

---

## End of Pass 2

This document is complete as of 2026-04-29. The next session's instance:

1. Picks up Pass 3 (rubric extension) — P11–P13 ratification against existing screens, ≤1 day per master plan.
2. OR jumps to Pass 4 (lifecycle ownership) — the foundation for Passes 5–7. ~3 days per master plan.
3. The Pass 6 rebudget decision (A/B/C) is still pending; the user has Pass 1 + 1.5 + this Pass 2 evidence to choose with.

Pass 1 + 1.5 + 2 cumulatively give the next instance everything they need to start cutting code.
