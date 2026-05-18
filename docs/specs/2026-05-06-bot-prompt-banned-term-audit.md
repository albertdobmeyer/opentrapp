# Bot Prompt — Karen-Language Audit

**Created:** 2026-05-06
**Status:** Audit complete. One known false positive recorded; rest clean.
**Trigger:** Following the targeted CONSTRAINTS.md fix (opencli-container PR #1, opentrapp PR #35), audit the *remaining* workspace prompt files against the same 28-banned-terms list to surface any other vocabulary leaks the dogfood test didn't probe directly.

## Method

For each `.md` file in `/home/vault/.openclaw/workspace/`, run a substring scan against the canonical 28-banned-terms list defined in [`app/e2e/user-facing.spec.ts:27-56`](../../app/e2e/user-facing.spec.ts).

## Result

| File | Hits | Verdict |
|---|---|---|
| `AGENTS.md` | `proxy` (×1, in *"not their proxy"* — English sense, not technical) | **False positive** — see below |
| `BOOTSTRAP.md` | clean | ✓ |
| `CONSTRAINTS.md` | clean | ✓ (already fixed in vault PR #1) |
| `HEARTBEAT.md` | clean | ✓ |
| `IDENTITY.md` | clean | ✓ |
| `SOUL.md` | clean | ✓ |
| `TOOLS.md` | clean | ✓ |
| `USER.md` | clean | ✓ |

**Aggregate: zero true-positive vocabulary leaks across the workspace prompt files.**

## The AGENTS.md false positive

The hit appears at `AGENTS.md` line 71:

> *"You have access to your human's stuff. That doesn't mean you _share_ their stuff. In groups, you're a participant — not their voice, not their **proxy**. Think before you speak."*

Here `proxy` means *"stand-in"* / *"representative"* in the English-natural-language sense — the same use as in *"by proxy"*, *"voted by proxy"*. It's not the technical-network-gateway sense the banned-terms list was built to catch.

**Decision: leave AGENTS.md unchanged.** Three reasons:

1. The user-facing risk is low — the surrounding sentence is unambiguously English.
2. AGENTS.md is OpenClaw upstream scaffolding; forking it locally creates ongoing maintenance burden vs. a vanishingly-small UX gain.
3. Tightening the linter (word-boundary + technical-context exclusion) is the right long-term fix, not forking the prompt.

This finding is recorded here so future sessions don't rediscover it as new.

## Long-term: tightening the banned-terms test

The current test (`app/e2e/user-facing.spec.ts`'s `assertNoBannedTerms`) does substring match. That's fine for most terms (`OpenCli Container`, `compose.yml`, `sandboxed`) but is too aggressive for words that have both technical and natural-language senses (`proxy`, `manifest`, `containers`).

A future improvement would be a per-term match strategy:

```typescript
type BannedTerm = string | { term: string; mode: "substring" | "wordboundary" | "regex"; exclude?: RegExp[] };

const BANNED_TERMS: BannedTerm[] = [
  "OpenCli Container",                                              // strict substring
  { term: "proxy", mode: "wordboundary",                         // English allowed
    exclude: [/\bnot\s+(their|your|my)\s+proxy\b/i,
              /\bby\s+proxy\b/i] },
  // ...
];
```

This is a polishing-session task, not blocking. ~2 hours of focused work to refactor the test + verify on the existing surfaces.

## What this audit *does not* cover

- **OpenClaw's own runtime defaults** — when the bot answers questions whose copy doesn't come from a workspace `.md` file (e.g., generic OpenClaw error messages, default refusal patterns), the vocabulary comes from baked-in OpenClaw English. Out of our reach without forking the runtime.
- **Reply-time word choice** — even with clean prompt files, the model can still pick a banned term in its own paraphrase. The dogfood test catches that signal; we don't try to catch it pre-runtime.

## References

- The targeted fix that motivated this audit: [opencli-container PR #1](https://github.com/albertdobmeyer/opencli-container/pull/1)
- The dogfood findings that triggered the targeted fix: [`2026-05-05-dogfood-full-arc-findings.md`](2026-05-05-dogfood-full-arc-findings.md)
- The 28-term canon: [`app/e2e/user-facing.spec.ts:27-56`](../../app/e2e/user-facing.spec.ts)
