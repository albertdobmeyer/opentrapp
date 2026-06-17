# UI Copy Clarity Pass — Design

**Date:** 2026-06-17 · **Status:** Approved (design) · **Owner:** albertd · **Branch:** `ui-copy-clarity`

## Problem

The user-facing copy reads as machine-generated: em-dashes throughout, awkward line wrapping
(single-word orphans, mid-phrase breaks), and descriptions that are wordier and vaguer than they
need to be. The first-run wizard is the most visible example ("Let's get you set
up — it takes about 3 minutes" wrapping as "get / you set up").

## Goal

Make the user-facing copy read like a person wrote it: no dashes, clean wrapping at every width, and
concise, semantically clear descriptions that tell the user plainly what to do.

## Scope

**In:** user-*visible* strings only, in the end-user surface:
- `app/src/components/{wizard,user,failure}/**`
- `app/src/components/{ActivationModal,ErrorBoundary,ModeSwitcher}.tsx`
- `app/src/pages/user/**`
- `app/src/content/use-cases.ts`
- `app/src/App.tsx` (user-facing strings only)
- Copy-*producing* modules whose output users read: `app/src/hooks/{useAlerts,useHero,useBootstrapProgress,useSentinelActivity}.ts`, `app/src/lib/errors.ts`

**Out:**
- `app/src/pages/dev/**` (developer-mode surface)
- All code comments (users never see them) — including em-dashes inside comments in otherwise in-scope files
- Developer-only / debug strings

## The copy rules (the convention)

1. **No em-dashes (`—`) or en-dashes (`–`) in user-visible copy.** Replace by intent:
   - Two independent clauses → two sentences (period). *Most common case.*
   - An aside → a comma, or parentheses if truly tangential.
   - A lead-in to detail/a list → a colon.
   - Literal hyphens in identifiers (`sk-ant-`, `vault-agent` in allowed contexts) stay.
2. **Drop filler lead-ins** ("Hang tight —", "Don't worry —", "Just"). Start with the substance.
3. **Voice (deeper rewrite):** short declarative sentences; active voice; second person; present
   tense; lead with the action the user takes; concrete over vague.
4. **Never introduce a banned jargon term** (enforced by `app/e2e/user-facing.spec.ts`): no
   `proxy`, `manifest`, `containers`, `seccomp`, `monorepo`, `component.yml`, `health probes`,
   `submodule`, the OpenClaw/ClawHub/Moltbook product names, etc. `Podman`/`Docker` only behind the
   existing "show terminal command" disclosure.

## Wrapping — CSS, not manual breaks

Add to the base layer of `app/src/styles/globals.css`:
- Headings / titles (`h1, h2, h3`, hero/step titles): `text-wrap: balance;`
- Body / paragraph copy: `text-wrap: pretty;`

This removes orphans and mid-phrase breaks responsively at every viewport width, which a hardcoded
`<br>` cannot (it is correct at one size and wrong at another). Use a manual break only where a line
break is genuinely semantic (rare).

## Before → after (the validated voice)

| Screen | Before | After |
|---|---|---|
| Welcome | "Your personal AI assistant, safe on your computer. Let's get you set up — it takes about 3 minutes." | "Your AI assistant runs safely on this computer. Setup takes about 3 minutes." |
| Connect (key tip) | "Anthropic shows the full key once — the string starts with sk-ant-. Copy it now; you can't retrieve it later. If you lose it, you can always create another." | "Anthropic shows your key just once, and it starts with sk-ant-. Copy it now and keep it safe. If you lose it, create a new one." |
| Status sublines | "Hang tight — this usually takes a few seconds." · "Setup running — watch the progress above" · "Working on it — no action needed." | "This usually takes a few seconds." · "Setup is running. Watch the progress above." · "Working on it. No action needed." |
| Help | "Something is broken — how do I recover?" | "Something broke. How do I fix it?" |

## Verification (Definition of Done)

1. `cd app && npm run lint` (eslint --max-warnings 0) clean; `npx tsc --noEmit` clean.
2. `npm test -- --run` (vitest) green — **update any unit test that asserts a changed string**.
3. `npx playwright test` green, including `user-facing.spec.ts` (the jargon ban) — copy changes must
   not introduce a banned term, and any e2e asserting changed copy is updated.
4. Final grep guard: **zero `—`/`–` in user-visible string literals** in the in-scope files (comments
   excluded).
5. Re-screenshot the wizard (vite preview or the spike harness) to confirm wrapping is clean.

## Out of scope / non-goals

- No layout, component-structure, or styling changes beyond the two `text-wrap` rules.
- No copy changes to developer-mode pages or code comments.
- No new i18n/strings-extraction system (copy stays inline; YAGNI).
