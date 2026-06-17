# UI Copy Clarity Pass — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task (inline; per-string judgment + jargon-ban awareness + lockstep test updates make a clean subagent fan-out risky). Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rewrite the user-facing copy to read like a person wrote it — no dashes, clean wrapping at every width, concise and clear — across the end-user surface, without introducing banned jargon or breaking tests.

**Architecture:** One global CSS change fixes wrapping (`text-wrap: balance`/`pretty`); the rest is a per-screen-group copy rewrite applying the rules in the design spec. Copy is inline JSX (no strings file), so edits live in the components/pages/hooks that render or produce user-visible text.

**Tech Stack:** React 18 + TypeScript, Tailwind/CSS (`globals.css`), vitest (unit), Playwright (e2e incl. the jargon-ban test).

**Rules + scope are in the spec — do not duplicate, follow it:** [`docs/superpowers/specs/2026-06-17-ui-copy-clarity-design.md`](../specs/2026-06-17-ui-copy-clarity-design.md). Every rewrite obeys §"copy rules" (no em/en-dash; drop filler; short active declarative sentences; no banned term) and §scope (user-visible strings only; never code comments; never `pages/dev/*`).

---

## Working method (applies to every copy task below)

For each file in the task:
1. Read the whole file. Identify **user-visible string literals only** (JSX text, `title`/`body`/`label`/`subline`/`placeholder`/`aria-label`/toast strings, arrays of copy). **Skip every code comment** (even ones containing `—`) and any dev-only string.
2. Rewrite each per the spec rules. Replace `—`/`–` by intent (period ≫ comma/colon). Tighten and clarify (deeper rewrite). Keep meaning. Introduce **no** banned term.
3. After editing the file, grep it for residual user-visible dashes (the verify step shows how).
4. Commit per group.

The before→after pairs already found in recon are listed in each task as the **known** cases; while reading each file, apply the same rules to **every** user-visible string, not only the listed ones.

---

## Task 1: Global wrapping fix (CSS)

**Files:** Modify `app/src/styles/globals.css`

- [ ] **Step 1: Add the text-wrap base rules**

Append to the base layer (after the existing base/`@layer base` rules, or at end of file if no layers):

```css
/* Readable wrapping: balance short/heading text, pretty-wrap paragraphs.
   Removes single-word orphans and mid-phrase breaks responsively at every width. */
h1, h2, h3, h4 {
  text-wrap: balance;
}
p, li {
  text-wrap: pretty;
}
```

If the file uses `@layer base { … }`, place these **inside** that block instead of bare, to match the existing cascade.

- [ ] **Step 2: Build to confirm CSS is valid**

Run: `cd app && npm run build 2>&1 | tail -4`
Expected: build succeeds; `dist/assets/*.css` regenerated.

- [ ] **Step 3: Commit**

```bash
git add app/src/styles/globals.css && git commit -m "style(ui): balance/pretty text-wrap for clean copy wrapping"
```

---

## Task 2: Onboarding wizard copy

**Files (read + rewrite user-visible strings):**
- `app/src/components/wizard/WelcomeStep.tsx`
- `app/src/components/wizard/ConnectStep.tsx`
- `app/src/components/wizard/ReadyStep.tsx`
- `app/src/components/wizard/WizardProgress.tsx`
- `app/src/components/wizard/install-step/MissingRuntimeCard.tsx`
- `app/src/components/wizard/install-step/pipeline-steps.ts`
- `app/src/components/wizard/install-step/utils.ts`

- [ ] **Step 1: Apply the known rewrites + sweep each file**

Known cases (apply, plus every other user-visible string per the rules):

| File | Before | After |
|---|---|---|
| WelcomeStep | "Your personal AI assistant, safe on your computer. Let's get you set up — it takes about 3 minutes." | "Your AI assistant runs safely on this computer. Setup takes about 3 minutes." |
| ConnectStep | "Anthropic shows the full key once — the string starts with sk-ant-. Copy it now; you can't retrieve it later. If you lose it, you can always create another." | "Anthropic shows your key just once, and it starts with sk-ant-. Copy it now and keep it safe. If you lose it, create a new one." |
| ReadyStep | "Say hi on Telegram — search for @{username} if it doesn't open the right chat." | "Say hi on Telegram. If the chat doesn't open, search for @{username}." |

`pipeline-steps.ts` / `utils.ts`: these hold install step **labels/messages** shown in the progress UI — rewrite the user-visible strings; leave any non-displayed identifiers and comments. Do **not** introduce `containers`/`proxy`/`manifest`/"Checking prerequisites" (banned).

- [ ] **Step 2: Verify no user-visible dashes remain in this group**

Run:
```bash
cd app && for f in src/components/wizard/WelcomeStep.tsx src/components/wizard/ConnectStep.tsx src/components/wizard/ReadyStep.tsx src/components/wizard/WizardProgress.tsx src/components/wizard/install-step/MissingRuntimeCard.tsx src/components/wizard/install-step/pipeline-steps.ts src/components/wizard/install-step/utils.ts; do grep -nE '—|–' "$f" | grep -vE '^\s*[0-9]+:\s*(//|\*|/\*)' ; done; echo "review any lines above — they must all be comments"
```
Expected: any remaining `—` lines are comments only (the grep filters obvious comment lines; eyeball the rest).

- [ ] **Step 3: Typecheck + commit**

```bash
cd app && npx tsc --noEmit && cd .. && git add app/src/components/wizard && git commit -m "copy(wizard): clearer, dash-free onboarding copy"
```

---

## Task 3: Home + status cards copy

**Files:**
- `app/src/pages/user/Home.tsx`
- `app/src/components/user/HeroStatusCard.tsx`
- `app/src/components/user/StatTile.tsx`
- `app/src/components/user/SpendingTile.tsx`
- `app/src/components/user/SentinelActivityBadge.tsx`
- `app/src/components/user/ProactiveAlertsBanner.tsx`
- `app/src/components/user/CleanedSkillsCard.tsx`
- `app/src/components/user/EgressApprovalsCard.tsx`
- `app/src/components/user/TipOfTheDay.tsx`

- [ ] **Step 1: Apply the known rewrites + sweep each file**

Known cases (HeroStatusCard):

| Before | After |
|---|---|
| "Hang tight — this usually takes a few seconds." | "This usually takes a few seconds." |
| "Setup running — watch the progress above" | "Setup is running. Watch the progress above." |
| "Working on it — no action needed." (×2) | "Working on it. No action needed." |

Sweep the rest per the rules. Watch `EgressApprovalsCard`/`SentinelActivityBadge` for security copy that must stay accurate but jargon-free (no `proxy`, `egress` is acceptable user language only if already used; prefer "blocked a connection"/"checked a skill").

- [ ] **Step 2: Verify dashes + typecheck**

Run:
```bash
cd app && for f in src/pages/user/Home.tsx src/components/user/HeroStatusCard.tsx src/components/user/StatTile.tsx src/components/user/SpendingTile.tsx src/components/user/SentinelActivityBadge.tsx src/components/user/ProactiveAlertsBanner.tsx src/components/user/CleanedSkillsCard.tsx src/components/user/EgressApprovalsCard.tsx src/components/user/TipOfTheDay.tsx; do grep -nE '—|–' "$f"; done; npx tsc --noEmit
```
Expected: remaining `—` are comments only; tsc clean.

- [ ] **Step 3: Commit**

```bash
git add app/src/pages/user/Home.tsx app/src/components/user && git commit -m "copy(home): clearer, dash-free status + card copy"
```

---

## Task 4: Help, Discover, Preferences, Security pages

**Files:**
- `app/src/pages/user/Help.tsx`
- `app/src/pages/user/Discover.tsx`
- `app/src/pages/user/Preferences.tsx`
- `app/src/pages/user/SecurityMonitor.tsx`
- `app/src/content/use-cases.ts`

- [ ] **Step 1: Apply the known rewrites + sweep each file**

Known case (Help):

| Before | After |
|---|---|
| "Something is broken — how do I recover?" | "Something broke. How do I fix it?" |

`Help.tsx` has the longest descriptions (security explanations) — this is where the **deeper rewrite** pays off most: split the long dash-joined sentences, lead with the user benefit, keep technically accurate but jargon-free ("secure gateway" is the existing user-facing term for the proxy — keep that pattern, never say `proxy`). `use-cases.ts` is the Discover catalogue copy — tighten each card's blurb.

- [ ] **Step 2: Verify dashes + typecheck**

Run:
```bash
cd app && for f in src/pages/user/Help.tsx src/pages/user/Discover.tsx src/pages/user/Preferences.tsx src/pages/user/SecurityMonitor.tsx src/content/use-cases.ts; do grep -nE '—|–' "$f"; done; npx tsc --noEmit
```
Expected: remaining `—` are comments only; tsc clean.

- [ ] **Step 3: Commit**

```bash
git add app/src/pages/user/Help.tsx app/src/pages/user/Discover.tsx app/src/pages/user/Preferences.tsx app/src/pages/user/SecurityMonitor.tsx app/src/content/use-cases.ts && git commit -m "copy(pages): clearer, dash-free help/discover/preferences/security copy"
```

---

## Task 5: Alerts, errors, recovery + shared chrome

**Files (copy-producing modules + shared components):**
- `app/src/hooks/useAlerts.ts`
- `app/src/hooks/useHero.ts`
- `app/src/hooks/useBootstrapProgress.ts`
- `app/src/hooks/useSentinelActivity.ts`
- `app/src/lib/errors.ts`
- `app/src/components/failure/FriendlyRetry.tsx`
- `app/src/components/failure/ContactSupport.tsx`
- `app/src/components/ActivationModal.tsx`
- `app/src/components/ErrorBoundary.tsx`
- `app/src/components/ModeSwitcher.tsx`
- `app/src/App.tsx`

- [ ] **Step 1: Sweep each file per the rules**

These produce the **alert titles/bodies, error messages, and recovery copy** users read. `lib/errors.ts` maps error classes to user-facing guidance — apply the deeper rewrite there (these are read at the worst moment, so clarity matters most). In the hooks/`App.tsx`/`ErrorBoundary`, rewrite only the **user-visible** strings; skip pure logic + comments. `FriendlyRetry`/`ContactSupport` line-9-style `—` are in comments → skip.

- [ ] **Step 2: Verify dashes + typecheck**

Run:
```bash
cd app && for f in src/hooks/useAlerts.ts src/hooks/useHero.ts src/hooks/useBootstrapProgress.ts src/hooks/useSentinelActivity.ts src/lib/errors.ts src/components/failure/FriendlyRetry.tsx src/components/failure/ContactSupport.tsx src/components/ActivationModal.tsx src/components/ErrorBoundary.tsx src/components/ModeSwitcher.tsx src/App.tsx; do grep -nE '—|–' "$f"; done; npx tsc --noEmit
```
Expected: remaining `—` are comments only; tsc clean.

- [ ] **Step 3: Commit**

```bash
git add app/src/hooks/useAlerts.ts app/src/hooks/useHero.ts app/src/hooks/useBootstrapProgress.ts app/src/hooks/useSentinelActivity.ts app/src/lib/errors.ts app/src/components/failure app/src/components/ActivationModal.tsx app/src/components/ErrorBoundary.tsx app/src/components/ModeSwitcher.tsx app/src/App.tsx && git commit -m "copy(alerts): clearer, dash-free alert/error/recovery copy"
```

---

## Task 6: Verification + test reconciliation (the gate)

**Files:** any `*.test.ts(x)` or `e2e/*.spec.ts` that asserts a changed string.

- [ ] **Step 1: Find unit tests asserting changed copy + update them**

Run: `cd app && npm test -- --run 2>&1 | tail -40`
For each failure caused by a copy change, update the test's expected string to the new copy (the copy is the source of truth now). Re-run until green.

- [ ] **Step 2: Lint + typecheck (CI gates)**

Run: `cd app && npm run lint && npx tsc --noEmit`
Expected: eslint `--max-warnings 0` clean; tsc clean.

- [ ] **Step 3: e2e incl. the jargon ban**

Run: `cd app && npx playwright test 2>&1 | tail -30`
Expected: green, including `user-facing.spec.ts` (no banned term introduced). Fix any e2e asserting changed copy; if a banned term slipped in, reword.

- [ ] **Step 4: Dash guard — zero user-visible em/en-dashes in scope**

Run:
```bash
cd app && git diff --name-only main -- 'src/**/*.tsx' 'src/**/*.ts' | grep -v '\.test\.' | while read f; do grep -HnE '"[^"]*[—–][^"]*"|>[^<]*[—–][^<]*<' "$f"; done; echo "above = any remaining dashes inside string literals / JSX text (should be empty)"
```
Expected: empty (no `—`/`–` inside user-visible literals or JSX text).

- [ ] **Step 5: Re-screenshot the wizard to confirm wrapping**

Re-run the spike browser harness (or `npm run preview`) against the built app and screenshot the Welcome screen; confirm no orphan/mid-phrase break. (Optional but recommended — it's the original symptom.)

- [ ] **Step 6: Final commit (if test updates were separate)**

```bash
git add -A && git commit -m "test(ui): reconcile assertions with clarified copy"
```

---

## Self-review (spec coverage)

- §scope (user-facing only, no comments, no `pages/dev/*`) → Tasks 2–5 enumerate exactly those files; verify steps grep for residual dashes. ✓
- §copy-rules (no dash, drop filler, active/concise, no banned term) → applied per file; jargon ban enforced by Task 6 Step 3. ✓
- §wrapping (CSS `balance`/`pretty`) → Task 1. ✓
- §verification (lint/tsc/vitest+test updates/e2e jargon ban/dash grep/re-screenshot) → Task 6. ✓
- Before→after examples from the spec → Tasks 2–4 known-cases tables. ✓
