# UX Principles Rubric for Non-Technical Users

**Date:** 2026-04-20 (extended 2026-04-29 with P11–P13 + placeholder-page scores per Pass 3 of the Delightful Sloth phase)
**Scope:** The OpenTrApp desktop GUI + the Telegram first-chat surface (added in Pass 1.5)
**Purpose:** A scoring tool, not a manifesto. Use this to audit screens and prioritize rebuilds.

---

## How to Use This Document

1. **When auditing a screen:** Walk through it as a non-technical user. For each principle, score 0–10 using the rubric below.
2. **When building a new screen:** Read the principles first. Run through the test questions before writing a single string.
3. **When prioritizing work:** Sort screens by aggregate score. Rebuild the lowest-scoring ones first.

**Don't treat every principle as equally weighted.** Principle 1 (no plumbing) is load-bearing — a 3/10 there is worse than a 3/10 on Principle 10.

---

## The Scoring Scale

| Score | Meaning |
|-------|---------|
| **10** | Exemplary. This screen could teach the principle to another team. |
| **7–9** | Good. Applies the principle with minor, easily-fixed gaps. |
| **4–6** | Partial. Applies the principle in places but has noticeable violations. |
| **1–3** | Fails. Violates the principle in a way that would confuse a real user. |
| **N/A** | Principle doesn't apply to this screen (e.g. no forms → Principle 8 is N/A). |

---

## The 10 Principles

### Principle 1: Never expose the plumbing

**The rule:** Internal architecture — containers, proxies, submodules, manifests, compose files, component IDs, seccomp, port numbers — never appears in user-facing text.

**Why:** Karen doesn't know what a container is and shouldn't need to. The product is an AI assistant, not a container orchestrator.

**Test question:** *If I stripped every sentence from this screen into a text file and showed it to my retired aunt, would she encounter a word she has to Google?*

**Exemplary (10/10):** Screen uses only words a Facebook user would recognize. No exceptions.
**Failing (1–3/10):** Screen shows container names, exit codes, file paths, or IDs.

---

### Principle 2: Outcomes over mechanisms

**The rule:** Describe what happened for the user, not what the system did internally.

**Why:** "24/24 checks passed" is a mechanism. "Your assistant is safe to use" is an outcome. The user cares about the latter.

**Test question:** *Does each status message answer "what does this mean for me?" rather than "what did the code do?"*

**Good examples:**
- "Your assistant is running safely" (outcome) — not "vault-agent container is up" (mechanism)
- "Ready to go" (outcome) — not "exit code 0" (mechanism)

**Failing:** Progress bars showing "Running cargo build...", status saying "exit 1".

---

### Principle 3: Every error tells the user what to do next

**The rule:** No error message is complete until it tells the user the next action to take.

**Why:** "Error: Tauri IPC not available" is a dead end. "This usually works on a second try — click Try Again" is a path forward.

**Test question:** *For every error message on this screen, can the user tell me, in one sentence, what they should do next?*

**Good:** "Didn't finish cleanly — this usually works on a second try."
**Bad:** "Error: ENOENT", "Something went wrong".

---

### Principle 4: Normalize transient failures

**The rule:** First-attempt failures that are commonly transient (timing, network blips) should be **reassuring**, not alarming.

**Why:** Red banners + "ERROR" screaming make Karen think she broke the app. Calm language + "this is normal" + Try Again keeps her going.

**Test question:** *If the first attempt fails with a race condition, does the UI make the user anxious or confident?*

**Good:** Amber icon + "Didn't finish cleanly — this usually works on a second try" + blue "Try Again" button.
**Bad:** Red X + "FAILED (exit 1)" + stack trace.

---

### Principle 5: Progressive disclosure

**The rule:** Show the minimum the user needs by default. Hide power-user details behind labeled toggles.

**Why:** 90% of users never need to see build logs, commands, or configs. The 10% who do should be able to find them — but not have them dumped on them by default.

**Test question:** *Can a first-time user complete the task without ever seeing developer details?*

**Good:** "Show details" toggle, "Developer Tools" collapsible section.
**Bad:** Terminal output shown by default, all configs visible on page load.

---

### Principle 6: Role-based labels, not component names

**The rule:** Users see what things DO, not what they're CALLED internally.

**Why:** "OpenCli Container" is a codename that means nothing. "My Assistant" describes the thing's role in the user's life.

**Test question:** *Is every label a word the user would use to describe the thing, not a word we invented?*

**Good:** "My Assistant", "Skills", "Network", "App Data Location".
**Bad:** "OpenCli Container", "OpenSkill Forge", "OpenAgent Social", "Monorepo Path".

---

### Principle 7: Status text is a sentence, not a state token

**The rule:** Status text reads like a sentence fragment a human would say, not like a machine state.

**Why:** "success", "done", "failed" are state tokens. "Ready to go", "Something isn't connecting" are sentences.

**Test question:** *Could each status message end with a period and sound like something a friendly person would say?*

**Good:** "Ready to go", "Setting up...", "Waiting for Telegram."
**Bad:** "SUCCESS", "STATE: RUNNING", "status=ok".

---

### Principle 8: Forms guide, don't interrogate

**The rule:** Every input field has (a) a plain-language label, (b) a short hint about *why* it's needed, and (c) either an example or a link to create the thing.

**Why:** "Anthropic API Key: ___" is interrogation. "Anthropic API Key: ___ Powers your AI assistant. Get one at console.anthropic.com" is guidance.

**Test question:** *For every input, could the user answer "why is this being asked?" and "where do I get this?" without leaving the screen?*

**Good:** Every field has a one-line purpose + a "Get one at…" link.
**Bad:** Labels alone, no placeholders, no hints, no links.

---

### Principle 9: Loading states have context

**The rule:** Every spinner has a sentence explaining what the app is doing, not just "Loading...".

**Why:** A spinner with no text is anxiety. A spinner with "Checking your computer..." tells the user the app is working for them.

**Test question:** *For every loading state, can the user tell a friend what the app is currently doing?*

**Good:** "Checking your computer...", "Setting up your assistant...".
**Bad:** Bare spinner, "Loading...", "Please wait...".

---

### Principle 10: Safe by default

**The rule:** Destructive actions (delete, reset, stop) require extra effort. The default path is always non-destructive.

**Why:** A user who clicks the wrong button by accident should be able to recover. Destructive actions should be amber or red and behind a confirmation if they can't be undone.

**Test question:** *If the user clicks every visible button in random order, can they destroy their setup?*

**Good:** "Stop Assistant" in caution-yellow, "Reset" behind a toggle.
**Bad:** "Delete All" as the primary blue button.

---

### Principle 11: The perimeter is alive iff the app is alive

**The rule:** App start ⇒ perimeter up. App close ⇒ perimeter down. App crash ⇒ perimeter cleanly torn down (no orphan containers). OS reboot ⇒ app autostart respected; perimeter follows the app, not the other way around.

**Why:** The product premise is that the perimeter is invisible-but-load-bearing infrastructure. That demands the perimeter's lifetime equal the user's session with the app. Today (per Pass 1's Phase 1 audit) the app is a control panel, not a lifecycle owner — SIGKILL leaks containers, closing the app leaves them running, relaunching shows "Your assistant is ready" even when the perimeter is offline. Each of those states is a lie to the user.

**Test question:** *If I randomly kill the Tauri process right now, will containers die within 5 seconds? If I relaunch, will the app know whether the perimeter is up before it tells the user "running safely"?*

**Good (10/10):** Process exit → containers stop. App relaunch → silent health probe → hero state matches reality. Watchdog auto-restarts a single dead container within 30s.
**Failing (1–3/10):** Tray placeholder says "initializing" forever. Hero status is hardcoded "running safely." `podman ps` shows zombie containers from a session that ended hours ago.

**Where to score N/A:** Static screens with no perimeter coupling (Welcome, Setup Complete, 404, ErrorBoundary).

---

### Principle 12: The app speaks only when it's helping

**The rule:** Notifications fire on **events that change the user's options** — never on routine internal state changes. Three valid trigger categories:
1. **Threat-blocked** — vault-proxy blocked something the agent tried; user should know
2. **State-recovery** — perimeter went degraded and came back; user noticed the gap
3. **Action-required** — API key invalid, credit empty, OS permission missing

**Why:** Pass 1 found no notification system today; the tray placeholder is mute. The vision quote was: "the app might give little updates and notifications once in a while to inform the enduser if something happens (injection attack prevented)." The opposite trap is over-notifying — the goal is "silent middleman," not "chatty middleman." Each false-positive notification trains the user to ignore the next one, including the real one.

**Test question:** *In a 24-hour idle test, does the app fire any notifications? It shouldn't.* Then: *In a planted-attack test, does it fire exactly one humane notification with a single clear next action? It should.*

**Good (10/10):** Three trigger categories cleanly separated. Each notification has plain-English copy + one actionable next step. OS-permission gate handled gracefully.
**Failing (1–3/10):** Notifications fire on container restarts that resolved in <60s. Notifications mention component names ("vault-agent transitioned to RUNNING"). Notifications without a clear user action.

**Where to score N/A:** Wizard screens (no notifications during setup); 404, ErrorBoundary, Setup Complete.

---

### Principle 13: Dev-tools-lite surfaces feel native, not embedded

**The rule:** When the user reaches for a power-user task (peek at activity, install a skill, download from the network, change a setting, get help), the surface they land on shares the visual language, copy voice, and layout grammar of the Home dashboard. It is NOT an iframe, a console window, an "Advanced" tab styled differently, or a wrapped CLI dump.

**Why:** Pass 1 found the post-wizard sidebar destinations are placeholders rendering "Coming in Phase E.2.X" with raw `docs/specs/...` paths. The architecture-driven temptation will be to "expose the manifest workflow" by showing the workflow output verbatim. The user-driven answer is that the workflow is invisible plumbing; the user sees only their question being answered.

**Test question:** *Take any user-mode page. Replace its title with the Home page's title. Same fonts, same colors, same copy voice? It passes. If a developer-affordance (raw stream, JSON dump, hex hash) is visible without explicit "Show technical details" disclosure, it fails.*

**Good (10/10):** A first-class user-facing page that wraps a backend workflow. The workflow's raw output is hidden behind a "Show technical details" disclosure.
**Failing (1–3/10):** "Coming in Phase E.2.X" + a `docs/specs/...` path visible to end-users. Raw forge stream as the primary view. Manifest YAML editor as the way to install a skill.

**Where to score N/A:** Pages that are core flow (Welcome, the wizard's four screens, Setup Complete, 404, ErrorBoundary), not dev-tools-lite.

---

## Score Matrix (19 Surfaces × 13 Principles)

Originally scored 2026-04-20 (rows 1–13). Re-scored 2026-04-29 with P11–P13 added; rows 14–19 added (5 user-mode placeholder pages + Telegram first-chat surface from Pass 1.5). **Wizard rows 2–5 re-scored 2026-04-29 (Pass 5)** after the MissingRuntimeCard rebrand, codename translations, error-pattern routing, and context-aware fallback. P2/P4 holdouts on row #4 (Configuration) reflect deferred P2 nice-to-haves ("Anthropic API key" → "AI account key" rename) and the unchanged transparency level. **Rows 14–18 re-scored 2026-04-30 (Pass 6)** after the Home / Discover / Preferences rebuilds + Security / Help friendlier-placeholder pattern. Tier-0 cliff resolved across all five surfaces.

| # | Surface | P1 | P2 | P3 | P4 | P5 | P6 | P7 | P8 | P9 | P10 | P11 | P12 | P13 | Avg |
|---|---------|----|----|----|----|----|----|----|----|----|----|----|----|----|-----|
| 1 | Setup: Welcome | 10 | 10 | N/A | N/A | N/A | 10 | 10 | N/A | N/A | 10 | N/A | N/A | N/A | **10.0** |
| 2 | Setup: System Check | 9 | 8 | 8 | 8 | 7 | 9 | 8 | N/A | 9 | 10 | N/A | N/A | N/A | **8.4** |
| 3 | Setup: Assistant Modules | 9 | 9 | 8 | 7 | 9 | 9 | 9 | N/A | N/A | 10 | N/A | N/A | N/A | **8.8** |
| 4 | Setup: Configuration | 8 | 8 | 8 | 6 | 9 | 7 | 8 | 9 | N/A | 9 | N/A | N/A | N/A | **8.0** |
| 5 | Setup: Setting Up Your Assistant | 9 | 8 | 9 | 9 | 9 | 10 | 8 | N/A | 9 | 10 | 6 | N/A | N/A | **8.7** |
| 6 | Setup: Complete | 10 | 10 | N/A | N/A | N/A | 10 | 10 | N/A | N/A | 10 | N/A | N/A | N/A | **10.0** |
| 7 | Dashboard | 10 | 9 | N/A | N/A | 8 | 10 | 9 | N/A | 8 | 10 | **3** | **0** | N/A | **7.4** |
| 8 | Component Detail (Assistant) | 8 | 7 | 6 | N/A | 9 | 9 | 8 | N/A | 7 | 8 | 5 | N/A | 7 | **7.4** |
| 9 | Component Detail (Skills) | 7 | 6 | N/A | N/A | 8 | 9 | 8 | N/A | 7 | 10 | N/A | N/A | 7 | **7.8** |
| 10 | Component Detail (Network) | 9 | 9 | N/A | N/A | 10 | 10 | 10 | N/A | N/A | 10 | N/A | N/A | 7 | **9.3** |
| 11 | Settings | 8 | 7 | N/A | N/A | 9 | 9 | 8 | 6 | N/A | 9 | N/A | 8 | N/A | **8.0** |
| 12 | 404 Page | 10 | 10 | 10 | N/A | N/A | N/A | 10 | N/A | N/A | 10 | N/A | N/A | N/A | **10.0** |
| 13 | ErrorBoundary | 7 | 5 | 7 | 6 | 4 | N/A | 8 | N/A | N/A | 9 | N/A | N/A | N/A | **6.6** |
| 14 | Home | 9 | 9 | N/A | 8 | 9 | 10 | 9 | N/A | 8 | 10 | **10** | **9** | 8 | **9.0** |
| 15 | Security Monitor (friendlier placeholder) | 8 | 8 | N/A | N/A | N/A | 9 | 8 | N/A | N/A | 10 | N/A | N/A | N/A | **8.5** |
| 16 | Discover | 9 | 9 | N/A | N/A | 9 | 10 | 9 | N/A | 8 | 10 | N/A | N/A | 8 | **9.0** |
| 17 | Preferences | 9 | 9 | 8 | 8 | 9 | 9 | 9 | 8 | N/A | 9 | N/A | **9** | N/A | **8.7** |
| 18 | Help (friendlier placeholder) | 8 | 8 | N/A | N/A | N/A | 9 | 8 | N/A | N/A | 10 | N/A | N/A | N/A | **8.5** |
| 19 | Telegram first-chat (live, Pass 1.5) | 7 | N/A | 9 | 10 | 8 | 9 | 8 | 9 | 6 | 10 | N/A | N/A | N/A | **8.4** |

**N/A means the principle doesn't apply** (no forms on a 404 page, no loading on the complete step, no perimeter coupling on the wizard's static pages, etc.).

**Bold scores changed in the 2026-04-29 re-score** — primarily lifecycle (P11) and notification (P12) drops on Dashboard, plus the 5 placeholder pages added.

**Bold scores changed in the 2026-05-02 Pass 7 Day 4 re-score** — Home P11 jumped 9 → 10 (the `paused_by_user` hero state shipped, closing the last gap from the 7-state hero machine — every state in `docs/specs/2026-04-29-delightful-sloth-target-ux.md` is now reachable). Home P12 jumped 8 → 9 (backend status aggregator runs every 60s, fires `assistant-status-changed` events on transitions, drives both the proactive alerts banner and the hero card; OS-permission gate added). Preferences P12 jumped 8 → 9 (notification toggle now requests OS permission on first enable, falls back gracefully to in-app toasts on denial; autostart toggle now actually wires through `tauri-plugin-autostart` instead of just persisting the preference).

---

## Priority Ranking (Lowest Score = Rebuild First)

### Tier 0 — Critical (score < 5.0)

**#14 Home (placeholder) — 3.8**
The user lands here right after the wizard's celebratory "Your assistant is ready! 🎉" — and sees `Coming in Phase E.2.2` + a `docs/specs/...` path in monospace. The single biggest contradiction in the product. P11 + P12 + P13 all 0/10: the page doesn't surface perimeter state, fires no notifications, and explicitly admits the dev plumbing is showing.

**#15–18 Security Monitor / Discover / Preferences / Help (placeholders) — 4.9 each**
Same shape as Home: P1=2 (phase code + spec path visible), P2=3 (promises features that don't exist), P13=0 (anti-native by definition). Slightly higher than Home because P11 + P12 are N/A for these (lifecycle and notifications belong to Home).

**Concrete fixes:** see Pass 6 in `~/.claude/plans/yes-we-are-building-delightful-sloth.md`. Three options on the table (A: ship 5 real, B: ship Home + Discover + Preferences real, C: ship Home only). All three options replace the "Coming in Phase E.2.X + spec path" pattern with a friendlier placeholder for whatever isn't shipped real.

**Same Tier 0 issue applies to the 10 dev-mode placeholder pages** (DevAllowlist / DevComponentDetail / DevComponents / DevLogs / DevManifests / DevOverview / DevPreferences / DevSecurity / DevShellLevels) — all render `DevPlaceholder.tsx` with phase-code + spec-path leaks. Lower priority because dev mode is gated behind Cmd/Ctrl+Shift+D + a confirm dialog, but the same Pass 7 cleanup must address them.

---

### Tier 1 — Rebuild soon (score 5.0 – 7.5)

**#13 ErrorBoundary — 6.6** (unchanged)
Issues:
- Shows raw `error.message` to the user (developer jargon leak)
- "Something went wrong" doesn't say what broke or what to do
- Hard to recover — only offers "Try again" and "Dashboard" without explaining when each applies

Concrete fixes:
- Map common error types (network, Tauri IPC, permission) to friendly sentences
- Hide `error.message` behind a "Show technical details" toggle
- Add contextual advice: "If this keeps happening, try restarting the app."

**#7 Dashboard — 7.4** (was 9.1 before P11 + P12 were added)
The original 10-principle scoring missed that the dashboard doesn't actually surface perimeter state. The tray placeholder says "initializing" forever; the hero section is hardcoded. Pass 4 (lifecycle ownership) is the fix — until then, the dashboard scores worse than every other "running" screen because it makes a load-bearing claim it can't substantiate.

**#8 Component Detail (Assistant) — 7.4** (was 7.8 before P11 was added)
Knows the component's state but not the perimeter's lifecycle. Same Pass 4 dependency.

---

### Tier 2 — Polish pass (score 7.5 – 8.5)

**#9 Skills detail page — 7.6**
Issues:
- Shows raw "skill count" and pattern counts without explaining significance
- "Developer Tools" label is good but the content inside is entirely developer-facing
- No clear action for the user beyond "scan" — what does scanning DO for them?

Fix: Add a user-facing summary ("25 skills installed, all clean. Last checked 2 minutes ago") and an "Install new skill" flow for when ClawHub is integrated.

---

**#4 Configuration step — 7.7**
Issues:
- Heading is "Configuration" — clinical. Should be "Connect Your Accounts" or "Add Your Keys".
- Shows missing config files as raw paths (`.env`) — violates P1
- Uses raw `component_name` field which could surface "OpenCli Container" if mapping is missing
- When keys are loaded from existing file, no confirmation toast

Fix: Rename heading, route `component_name` through the role-based label function, hide file paths behind a "Show file details" toggle.

---

**#8 Component Detail (Assistant) — 7.8**
Issues:
- "Actions" header is clinical — could be "What you can do"
- When not_setup, no breadcrumb back to the wizard
- Security badge shows on-hover detail that's technical

Fix: Rename section headers in a conversational voice, add a "Set up now" button when in not_setup state.

---

**#2 System Check — 8.0**
Issues:
- When Podman is missing, shows `sudo apt install podman podman-compose` — terminal jargon leak (P1)
- "Continue anyway" is ambiguous — continue to what? With what risks?
- Install guidance for macOS/Windows is a download link but no friendly wording

Fix: Replace terminal commands with "We'll help you install the missing piece — click here" and open platform-appropriate guidance. Reword "Continue anyway" to "Skip this for now (not recommended)".

---

**#11 Settings — 8.0**
Issues:
- "App Data Location" field is good but no user would know when they'd need to change it. Needs context: "Advanced — leave empty unless your admin told you otherwise."
- Form doesn't have a save confirmation for simple changes
- "Re-run Setup Wizard" is fine but no warning that this will take you through configuration again

Fix: Add "Advanced" tag to App Data Location, hide it behind a toggle, improve feedback on save.

---

**#5 Setting Up Your Assistant — 8.1**
Issues:
- The row status text only shows one line — when setup is actually running, the user has no sense of progress (no "Step 3 of 12" or time estimate)
- The "Something went wrong" messaging was improved (Fix 2) but still no specific guidance for KNOWN failure modes (e.g. "proxy not ready yet — common on slow machines")

Fix: Add step-based progress ("Building container... Installing dependencies... Running security checks..."), detect common failure patterns from stream output and show targeted guidance.

---

**#3 Assistant Modules — 8.3**
Issues:
- "Installed" / "Partially installed" / "Not installed" is good but doesn't explain the difference
- "Install Missing Modules" button is fine but doesn't say how long it'll take

Fix: Add hover tooltips or inline explanations for partial install state, show an estimated time on install button.

---

### Tier 3 — Already strong (score 8.5+)

#19 Telegram first-chat (8.4 — borderline; flagged for `components/opencli-container` per Pass 1.5 to push to 9+), #10 Network placeholder (9.3), #1 Welcome (10), #6 Complete (10), #12 404 (10).

These don't need rebuilds. If we touch them, it should be for feature additions (e.g. real-time status on the dashboard, fresh-account pairing-flow re-test for Telegram), not UX fixes.

---

## Recommended Next Pass

The 2026-04-29 re-score makes Pass 4 (lifecycle ownership, ~3 days) the unambiguous next code work, because the Dashboard's drop from 9.1 → 7.4 is entirely on P11 — and Pass 4 is the *only* pass that closes P11. Without Pass 4, the placeholder rebuild (Pass 6) ships a Home page that reads from a perimeter the app doesn't own — same lie, prettier UI.

Sequenced recommendation per the master plan (`~/.claude/plans/yes-we-are-building-delightful-sloth.md`):

1. **Pass 4 — Lifecycle ownership (~3 days)** — closes P11 globally. After: Dashboard recovers to ≥9, ErrorBoundary's "Try Again" can actually do something, P12 work has reliable signal sources to fire on.
2. **Pass 5 — Wizard polish (~3 days)** — close the 3 remaining wizard P0s from Pass 1 (MissingRuntime "sudo apt install" leak; codenames in technical log; "Podman or Docker" naming). Push every wizard screen ≥9.
3. **Pass 6 — Dev-tools-lite surface (~3–9 days, A/B/C still pending)** — the 5 placeholder pages get the Tier-0 fix. The choice between Options A/B/C is a user decision informed by Pass 1 + Pass 1.5 evidence.
4. **Pass 7 — Notifications + recovery + cleanup (~2 days)** — wires P12. Adds Pass 1.5 anti-patterns to the banned-term list.
5. **Pass 8 — Pre-ship full re-walk (~1–2 days)** — re-score against this extended rubric. Targets: every screen ≥ 8.5, zero P1 violations, all lifecycle stress tests pass.

---

## Anti-Patterns Observed But Already Fixed

Keeping this as a reference for what NOT to do:

- ❌ "OpenClaw Orchestrator" subtitle in sidebar → Removed
- ❌ "COMPONENTS" section header → Removed
- ❌ "Monorepo Path" in Settings → "App Data Location"
- ❌ "Checking prerequisites..." → "Checking your computer..."
- ❌ "Container runtime" → "Secure sandbox"
- ❌ "Component submodules" → "Assistant modules"
- ❌ "Cloned but missing component.yml" → "Partially installed"
- ❌ "Clone All Submodules" → "Install Missing Modules"
- ❌ "Something went wrong — click Retry" → "Didn't finish cleanly — this usually works on a second try"
- ❌ "Retry" (amber) → "Try Again" (blue, primary action color)

These were caught in three ways: walkthrough audits with the Karen persona, the 19-term banned list in `app/e2e/user-facing.spec.ts`, and reviewing screenshots after each change. Future work should use all three techniques.

---

## Anti-Patterns Observed in Pass 1 + Pass 1.5 — NOT YET FIXED

These are the new violations the 2026-04-28 dogfood walkthrough and 2026-04-29 live first-chat run surfaced. Pass 7's cleanup must address each (and the banned-term list in `app/e2e/user-facing.spec.ts:13-33` should grow to cover the catchable ones).

**Phase-code + spec-path leaks (Tier 0 — Pass 6 owns):**

- ❌ "Coming in Phase E.2.X" → friendlier placeholder OR ship the page real
- ❌ `spec: docs/specs/ui-rebuild-2026-04-21/user-mode/0X-...md` (monospace, end-user-visible) → never appears in user mode

**Banned-term list gaps (Pass 7 Day 4 sweep — extended `app/e2e/user-facing.spec.ts:13-50`):**

- ✅ `containers` (plural), `sandboxed`, `web_search`, `web_fetch`, `admin key`, `Admin key`, `Admin Key`, `billing scope`, `cost endpoint` — **CLOSED 2026-05-02 (Pass 7 Day 4):** added to `BANNED_TERMS` array. The `admin key` family guards against re-introduction of Day-1a's unwound spending feature; the `web_*` and `sandboxed` terms guard against the Pass 1.5 bot-reply leaks recurring inside the parent app's user surfaces. Sweep verified clean across `app/src/pages/user/` and `app/src/components/user/`.
- ⏸ `container` (singular), `sandbox` (singular), `Podman`, `Docker`, `Forge`, `Pioneer`, `Vault` — **deferred to Pass 8 polish:** singular `container`/`sandbox` are too useful as English nouns ("safe container", "sandbox metaphor") to ban globally; Podman/Docker remain in the install-flow body text for macOS/Windows where the user must download the actual product (the Linux path already hides them behind a "Show terminal command" disclosure per Pass 5). Forge/Pioneer/Vault appear only on the marketing landing page (out of this app's scope); flagged for Pass 8 deserve-to-exist review.

**Wizard P0s named in Pass 1 (Pass 5 owns):**

- ✅ `sudo apt install podman podman-compose` displayed as primary install guidance — **CLOSED 2026-04-29 (Pass 5):** primary copy is now "sandbox runner"; the raw apt command is hidden behind a "Show terminal command" disclosure with a "use the guide above" framing.
- ✅ Internal codenames in InstallStep technical log (`opencli-container: setup`, `openskill-forge: setup`) — **CLOSED 2026-04-29 (Pass 5):** translated to "Your assistant: install" / "Skill scanner: install" / "Sandbox runner: ready" / "Downloading your assistant…" / "Running assistant security audit (24 checks)…".
- ✅ Three thrown errors that fall through to `UNKNOWN_FALLBACK` — **CLOSED 2026-04-29 (Pass 5):** added specific patterns for `Some assistant modules failed to download`, `Workflow ended with status:`, and `exited with code` in `app/src/lib/errors.ts:54-90`. Also made `UNKNOWN_FALLBACK` context-aware via `classifyError(err, context?)` — Karen now sees "Building didn't finish" instead of "Something went wrong".

**Telegram first-chat P0s named in Pass 1.5 (`components/opencli-container` system prompt — out of parent scope):**

- ❌ `/start` reply is "Pong. What can I help with?" — too terse, doesn't introduce the assistant or set expectations
- ❌ Tool-inventory inconsistency: `what can you do?` advertises web search; `summarize today's news` admits there's no `web_search/web_fetch tool`

---

## How to Use This Rubric for New Work

Before shipping any new screen:

1. **Read the 10 principles** — don't skim, read.
2. **Score the screen** against each principle. Write the score in a PR comment.
3. **Any score below 7 blocks shipping.** Fix or justify.
4. **Add new banned terms** to `app/e2e/user-facing.spec.ts` if you find any during the audit.
5. **If a new principle emerges** from the audit, add it here as Principle 11, 12, etc.
