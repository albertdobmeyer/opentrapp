# Dogfood Walkthrough Findings — Pass 1

**Date opened:** 2026-04-28
**Author:** Claude (Opus 4.7) impersonating a non-technical end-user
**Status:** ROLLING — populated moment-by-moment as the walkthrough proceeds
**Scope of this session:** SIGNAL COLLECTION ONLY. No fixes are applied during Pass 1. The next session's instance reads this file and applies fixes against the punch-list.
**Plan reference:** `~/.claude/plans/yes-we-are-building-delightful-sloth.md` Pass 1
**Rubric reference:** `docs/specs/2026-04-20-ux-principles-rubric.md`

---

## Methodology

I impersonate a non-technical end-user — call them **Karen** — who has read on Reddit about a powerful new AI agent (OpenClaw) and the security risks around it. She wants to use it for productivity (drafting emails, planning trips, summarizing articles, organizing files) but is scared of the threats. She lands on opentrapp.com hoping for a safer way in.

For each user moment, I walk through as Karen would, then:

1. Score the moment against the existing 10-principle rubric.
2. Document every friction with `file:line` references.
3. Capture the friction's severity (P0 = blocks the user / P1 = significant clunk / P2 = polish opportunity).
4. Suggest concrete fixes (without prescribing implementation).

Severity legend:
- **P0** — Karen can't progress, gets confused, or bounces.
- **P1** — Karen progresses but the moment feels clunky or violates a load-bearing principle.
- **P2** — works fine but could feel more delightful.

---

## The 8 user moments

| # | Moment | Status | Aggregate score |
|---|---|---|---|
| 1 | Discovery (opentrapp.com → pitch → download) | DOCUMENTED | 6.2/10 |
| 2 | First-run install + wizard | DOCUMENTED | 9.5/10 |
| 3 | First chat (Telegram pairing → first message → response) | DOCUMENTED | 5.5/10 |
| 4 | Returning use (close laptop → reopen → app already running) | DOCUMENTED | ~4.8/10 |
| 5 | Monitoring peek (curiosity) | DOCUMENTED | ~4.8/10 |
| 6 | Add a tool from openclaw network | DOCUMENTED | ~4.8/10 |
| 7 | Download from openclaw network | DOCUMENTED | ~4.8/10 |
| 8 | Crash & recovery | DOCUMENTED | ~1/10 |

**Methodology caveat:** All findings are from CODE READING in this session, not from live-running the app. Dynamic friction (UI lag, animation timing, real Telegram round-trip behavior, real install-time failure modes) is not in this doc. Recommend a brief live-run validation sub-pass during Pass 5 or Pass 8.

---

# Moment 1 — Discovery

**Surface:** `docs/index.html` (the landing page at opentrapp.com — Hetzner deploys this static file directly).

**Karen's mental state:** Has read about OpenClaw and is intrigued. Has also read scary threads about AI agents going rogue, leaking keys, etc. Lands on opentrapp.com via a link from a comment thread. She has 60 seconds to decide whether to download.

## Walkthrough notes (Karen's eye)

### Above the fold (hero, lines 597–632)

- **Headline** (`docs/index.html:603`) — "Your own AI assistant, safe on your computer." → ✅ instantly clear, hits the value prop. Karen reads it and gets it.
- **Sub-headline** (`docs/index.html:605-607`) — "Get a personal AI assistant you control from Telegram — one that runs on your computer, not in the cloud. OpenTrApp keeps it safely separated from your personal files, passwords, and accounts. **No terminal required.**" → ✅ "No terminal required" is the right phrase to land early. Reassuring.
- **Hero badge** (`docs/index.html:600`) — "v0.2.0 — Hardened release" → ⚠️ **P2:** version numbers are developer culture; "Hardened release" is jargon. Karen doesn't know what hardening means in software. She glosses past it but it gives a faint "this is a tech tool" vibe.
- **Trust list** (`docs/index.html:626-631`) — four bullet items:
  - "Agent can't touch your files" → ⚠️ **P1, Principle 6:** "agent" is the inconsistency. The headline says "assistant"; this says "agent." Karen registers the mismatch subliminally. Pick one — recommend "assistant" everywhere user-facing.
  - "API keys stay outside the sandbox" → ⚠️ **P1, Principle 1+8:** "sandbox" appears here for the first time without ever being defined. Karen knows what an API key is (she has one for ChatGPT) but "sandbox" is unexplained jargon at this point in the page.
  - "Every network request logged" → ⚠️ **P1, Principle 1:** "network request" is dev-speak. Translate to "Every site your assistant visits is logged" or "We log everywhere your assistant goes online."
  - "24-point security verification" → ⚠️ **P2, Principle 2:** mechanism, not outcome. Translate to "Runs through a 24-point safety check" or just "Verified safe before every start."

### Hero visual SVG (lines 635–700)

This is the dense one. The intent — show the security architecture as a one-directional flow with three barriers — is correct and on-vision. Karen will glance at it and her eye will catch the labels.

- ⚠️ **P0, Principle 1:** SVG text labels include "SECURE SANDBOX" (line 644), "PROXY GATEWAY" (line 662), "SCANNER + MONITOR" (line 681), "domain allowlist" (line 671), "request logging" (line 672), "prompt injection detection" (line 688). This is a wall of plumbing-language at the moment Karen is most-likely scanning the page.
- The diagram could keep its conceptual content but rename labels: "Where your assistant lives" instead of "SECURE SANDBOX"; "Key vault" or "Where your keys are kept safe" instead of "PROXY GATEWAY"; "Skill checker" instead of "SCANNER + MONITOR"; "Approved-sites filter" instead of "domain allowlist"; "Logs every step" instead of "request logging."
- ✅ The phrase "API keys stay outside the container" (line 692) is good in spirit but contradicts itself: "container" is the dev-speak word. Substitute "API keys stay outside the sandbox" or better "Your assistant never sees your API keys directly."

### Features section (lines 706–738)

- **Title** (line 708) — "Three layers of protection, working together" → ✅ clear.
- **Subtitle** (line 709) — "Your AI assistant is powerful — these layers make sure it stays safe." → ✅ frames the problem the user feels.
- **Card 1 — Contain** (lines 711–719): "locked-down sandbox" again. "API keys are kept outside the sandbox — the assistant never sees them." The card-detail line "6 independent security layers protect you even if one layer has a bug" → ⚠️ **P2, Principle 2:** mechanism-flavored ("layers," "bug"). Better: "Multiple safeguards work together so a single mistake can't expose your stuff."
- **Card 2 — Scan** (lines 721–728): "scanned for 87 known malware patterns" → ✅ specific and credible. The card-detail "Built because 11.9% of published OpenClaw skills were found to be malware" → ✅ strong evidence; this is the kind of stat that makes Karen trust the product.
- **Card 3 — Monitor** (lines 730–737): "checked against an approved list" → ✅ clear. "approved websites" → ✅ clear.

### Ecosystem section (lines 744–840) — **THIS IS WHERE THE PAGE LEAKS THE MOST**

- **Title** (line 746) — "What's inside" → fine.
- **Card role labels** (lines 751, 764, 777) — "Runtime / Toolchain / Network" → ⚠️ **P1, Principle 6:** these are dev architecture buckets. Karen doesn't know what "runtime" or "toolchain" means as nouns. They're decorative for a dev audience but confusing here. Drop them or rename: "Your assistant," "Skill safety," "Network safety."
- **Card 1 H3 "Your Assistant"** (line 756) → ✅ user-friendly.
- **Card 2 H3 "Skill Scanner"** (line 769) → ✅ clear.
- **Card 3 H3 "Network Monitor"** (line 782) — copy mentions "the network is containerized and ready" (line 784) → ⚠️ **P0, Principle 1:** "containerized" is plumbing leaking through. Rewrite: "Coming soon — the safety system for the network is already in place."
- **Ecosystem flow diagram** (lines 791–839):
  - The diagram nodes are labelled **Forge, Pioneer, Vault** (lines 804, 812, 820) — the **internal codenames** of the three sub-repos. → 🚨 **P0, Principle 6:** This is the highest-severity leak on the page. The product identity spec (`docs/specs/2026-04-19-product-identity-spec.md:150-167`) explicitly maps these to "Skill Store / Agent Network / My Assistant" for users. The landing page should use the user labels in the diagram. Right now, anyone hovering on the page sees the dev codenames. Karen reads "Forge, Pioneer, Vault" and either gets confused or files this product as "for developers."
  - The "You → Forge → Pioneer → Vault → OpenTrApp wrapper" flow is also subtly off the actual security architecture. The real flow is: You command the assistant in Vault; Vault talks via Proxy; skills come through Forge into Vault; feeds come through Pioneer into Vault. The straight-line "stages" arrangement implies a linear pipeline that isn't quite the architecture. Worth revisiting in Pass 2 (aspirational spec) for accuracy AND user-friendliness.

### How It Works (lines 845–869)

- **Title** (line 847) — "From download to dashboard in three steps" → ✅ concrete and inviting.
- **Subtitle** (line 848) — "No terminal required. The setup wizard checks everything for you." → ✅ second mention of "no terminal required"; this is good repetition for the people who skim past the hero.
- **Step 1** (lines 851–855) → ✅ clear, friendly.
- **Step 2** (lines 857–861) → ✅ accurate description of the existing wizard's guided modals.
- **Step 3** (lines 863–867) — "Start, stop, and monitor your **agent** with one click." → ⚠️ **P1, Principle 6:** "agent" again instead of "assistant." Same mismatch as the trust list. Also: "Every network request is logged for your review" → "network request" is dev-speak; could be "Every site your assistant visits is logged for your review."

### Download section (lines 874–911)

- **Title** (line 876) — "Get OpenTrApp for your platform" → ✅ clear.
- **Subtitle** (line 877) — "All installers are built from source in CI and signed. **Requires Podman or Docker installed on your system.**" → 🚨 **P0, Principles 1 + 3 + 8:**
  - "Built from source in CI and signed" → dev jargon Karen doesn't parse.
  - "Requires Podman or Docker installed on your system" → **THIS IS THE BIGGEST FRICTION POINT ON THE LANDING PAGE.** Karen has no idea what Podman or Docker is. She googles "Podman" and lands on a complex documentation site that asks her to use the command line. She bounces.
  - **This single line negates "No terminal required" said twice earlier.** The whole "no terminal" promise of the page collapses at the download moment.
  - **Recommended fix:** Either bundle Podman/Docker with the installer (the setup wizard already detects it; we should also offer to install it for her — Pass 5 territory), OR rewrite this line to be reassuring instead of alarming: "If your computer doesn't have a 'sandbox runner' yet, the setup wizard will install one for you in one click."
- **Linux/macOS/Windows cards** (lines 879–911) — three platforms, each with download links → ✅ structurally fine, BUT all the links currently point to `https://github.com/albertdobmeyer/opentrapp/releases/latest` which doesn't have a release object yet (per Phase 1 Plan-mode finding). Until Pass 8 produces the actual binary release, every download link is a dead end. → 🚨 **P0:** can't ship without resolving this.

## Score against rubric (Moment 1 — Landing page)

| Principle | Score | Notes |
|---|---|---|
| P1 — Never expose plumbing | **3/10** | "sandbox," "container/containerized," "agent," "network request," "Podman or Docker," "CI," "Forge/Pioneer/Vault" all appear in user-facing copy. Multiple violations on a page that's most users' first impression. |
| P2 — Outcomes over mechanisms | **6/10** | Mostly outcome-focused, but "24-point verification," "6 independent security layers," "built from source in CI" are all mechanism-flavored. |
| P3 — Errors tell the user what to do next | N/A | No error states on the landing page. |
| P4 — Normalize transient failures | N/A | No failure states. |
| P5 — Progressive disclosure | **8/10** | Page is well-paced. Hero → features → ecosystem → how-it-works → download is a sane disclosure order. |
| P6 — Role-based labels, not component names | **4/10** | Forge/Pioneer/Vault appearing as diagram labels is a direct violation. "Runtime/Toolchain/Network" badges similarly. |
| P7 — Status text is a sentence | N/A | No status surface here. |
| P8 — Forms guide, don't interrogate | N/A | No forms on this page. |
| P9 — Loading states have context | N/A | No loading states (static page). |
| P10 — Safe by default | **10/10** | No destructive actions to misclick. |

**Aggregate (excluding N/A): ~6.2/10.**

For comparison: the wizard screens score 7.7–10 in the existing rubric. **The landing page is currently the lowest-scoring user-facing surface in the product** — the very first thing Karen sees scores worse than any screen she'll encounter inside the app.

## Concrete fixes for Pass 5/7 (or a separate landing-page fix)

Listed in priority order. All are copy + SVG label changes — no architectural work.

1. **(P0)** Replace Forge/Pioneer/Vault diagram labels (`docs/index.html:804, 812, 820`) with the user-facing names from the identity spec: "Skills Store" (or "Skill Safety"), "Network Safety," "Your Assistant."
2. **(P0)** Rewrite the download-section Podman/Docker requirement line (`docs/index.html:877`) to either promise installation as part of the wizard, or hide it behind an "Advanced details" disclosure.
3. **(P0)** Resolve the dead-link problem at the download buttons — the GitHub Release object needs to exist or the download flow needs to point somewhere real. (Cross-references Pass 8.)
4. **(P1)** Hero SVG labels (`docs/index.html:644, 662, 681, 671, 672, 688, 692`) — rewrite from technical to outcome language. Suggested mapping above.
5. **(P1)** Eliminate the "agent" vs "assistant" inconsistency: pick "assistant" globally. Affected lines: 627, 866.
6. **(P1)** "Runtime / Toolchain / Network" eco-card badges (`docs/index.html:751, 764, 777`) — drop them or rewrite to user-facing nouns.
7. **(P1)** "containerized" mention (`docs/index.html:784`) — rewrite.
8. **(P2)** "Hardened release" badge (`docs/index.html:600`) — consider plain-English alternative or drop the version badge for non-dev audiences.
9. **(P2)** "Network request" → "site your assistant visits" everywhere.
10. **(P2)** "Built from source in CI and signed" → "Built from source and signed" or just "Open source — see the code on GitHub."

## Cross-pass implications

- **Pass 2 (aspirational spec)** — needs to define what the landing page SHOULD say, not just what's wrong with it. The identity spec's translation table is most of what we need; we just have to apply it to the landing page.
- **Pass 4 (lifecycle)** — irrelevant for the landing page itself, but the wizard's later promise of "everything just works" depends on Pass 4 being real.
- **Pass 5 (wizard polish)** — the landing page promises "the setup wizard walks you through" — Pass 5 must deliver on that. If the wizard still has its current 8.0–8.3 friction, the page is writing checks the wizard will fail to cash.
- **Pass 8 (regression)** — the dead-link download problem MUST resolve. Either binaries shipped to the GH Release before public download is enabled, OR the download CTA points to a "Coming soon — sign up for early access" form during a soft-launch period.

---

# Moment 2 — First-run install + wizard

**Surface:** `app/src/pages/Setup.tsx` orchestrating four screens at `app/src/components/wizard/`:
- `WelcomeStep.tsx`
- `ConnectStep.tsx`
- `InstallStep.tsx` (the long one)
- `ReadyStep.tsx`

Plus support: `WizardProgress.tsx`, `HowToModal.tsx`, `app/src/lib/errors.ts` (`classifyError` — the error-text translator), `app/src/lib/wizardUtils.ts`.

**Karen's mental state:** She downloaded, installed (skipping over the Podman friction from Moment 1, somehow), and is now opening the app for the first time. Setup.tsx auto-routes her to `/setup` because `!settings.wizardCompleted`. Time budget in her head: "this should take under 5 minutes."

## 2.1 — Welcome screen (`WelcomeStep.tsx`)

**Karen's read:**
- Title: "Welcome to OpenTrApp" — ✅ greeting.
- Body: "Your personal AI assistant, safe on your computer. Let's get you set up — it takes about 3 minutes." — ✅ value prop + concrete time estimate.
- "Get Started" button auto-focused — ✅ great keyboard-accessibility detail (`WelcomeStep.tsx:18-20`).
- Friendly hand-rolled logo + shield illustration — ✅ on-brand.
- "Already set up? Skip to dashboard" only shows when `wizardCompleted` is already true — ✅ progressive disclosure for re-entries.
- Code comment at line 65-66: "*Placeholder visual standard for E.2.1; a real unDraw asset replaces this in E.4*" — internal reference, not user-visible. Note: this means the team has plans to swap to professional art; for v0.2.x ship the hand-rolled is fine.

**Score:** P1 = 10/10, P2 = 10/10, P5 = 10/10, P6 = 10/10, P7 = 10/10, P10 = 10/10. **Aggregate: 10.0** — matches existing rubric.

## 2.2 — Connect screen (`ConnectStep.tsx`)

**Karen's read:**
- Title: "Connect your accounts" — ✅ outcome-framed (was "Configuration" before — already fixed; the rubric is slightly out of date there).
- Sub-title: "Your assistant needs two things to work. Enter them once and you're done. Nothing leaves your computer." — ✅ all three sentences land.
- **Anthropic card:**
  - Label "Anthropic API key" — ⚠️ **P2:** "API key" is borderline jargon, but most non-tech users with a ChatGPT subscription have encountered it. Acceptable.
  - Sub-label `ConnectStep.tsx:212-214`: "The AI's brain. Also how you'll pay for its thoughts (about $5–20/month for typical use)." — ✅✅✅ **This is gold.** Cost transparency, friendly metaphor, sets expectations about money. The rubric should canonize this writing voice as the standard.
  - Live valid-format checkmark via `isAnthropicKeyLike()` — ✅ Principle 8 (forms guide). Karen pastes, sees green, knows it's right.
  - Show/hide eye toggle — ✅ standard pattern.
  - "Show me how to get one (2 min)" — ✅ guided modal CTA, with time estimate.
- **Telegram card:**
  - Label "Telegram bot" — ✅
  - Sub-label "How you'll talk to your assistant." — ✅ outcome.
  - "Walk me through it (3 min)" — ✅ guided modal.
- **Smart-paste swap-detection** (`ConnectStep.tsx:110-131`): if Karen pastes a Telegram token into the Anthropic field by mistake, the wizard auto-swaps and announces it via `aria-live`. → ✅✅ **Excellent UX detail.** The aria-live announcement (line 124-129) is also a great accessibility touch.
- **Existing-key masked re-entry** (`ConnectStep.tsx:216-229`): if Karen returns to the wizard, her stored keys appear masked with a "Change" button. → ✅ Principle 5.
- **Skip button** + **Continue button** — ✅ allows progression without keys (some users may want to validate the install first).

**Frictions:**
- ⚠️ **P2, Principle 8:** "Anthropic API key" → "Anthropic key" or "AI account key" might be friendlier. Low priority since the modal CTA explains itself.
- ⚠️ **P2:** When Karen returns with already-saved keys and clicks **Continue** without changing anything, the wizard moves on silently with no toast acknowledging "your saved keys are still in use." (Existing rubric flagged this at line 224.) Minor.
- ⚠️ **P1, Principle 1+3:** Internal call `readConfig("openclaw-vault", ".env")` (`ConnectStep.tsx:87`) — if it errors, the toast at `ConnectStep.tsx:163-169` shows raw `err.message`. Could leak strings like "openclaw-vault not found" or path errors. The error path SHOULD route through `classifyError()` like InstallStep does. Currently it doesn't.

**HowToModal walkthroughs (`ConnectStep.tsx:20-66`):**
- Anthropic 5-step: "Open the Anthropic console" → "API Keys page" → "Create a new key" → "Copy the key immediately (sk-ant- starts)" → "Paste back" — ✅ all steps clear and actionable. Mention of `sk-ant-` is jargon-adjacent but Karen will see it on her clipboard so it's helpful confirmation.
- Telegram 5-step: "@BotFather" → "/newbot" → "Pick a name and username" → "Copy the token" → "Paste back" — ✅ clear walkthrough; `1234567890:ABCdef...` example helps Karen recognize the token shape.
- Code comment at `HowToModal.tsx:94`: `{/* TODO E.4: inline screenshot goes here */}` — modal currently text-only; screenshots planned for later. **For v0.2.x ship this is fine.** A non-tech user who's never seen BotFather might still get lost despite the words; a Pass 7 polish opportunity if budget allows.

**Score (Connect screen):**

| Principle | Score | Notes |
|---|---|---|
| P1 | **9/10** | Internal "openclaw-vault" string could leak via uncaught error toast. |
| P2 | **10/10** | Cost copy is a model for the rest of the app. |
| P3 | **7/10** | Read-config error path doesn't route through `classifyError`. |
| P5 | **10/10** | Show/hide, skip, masked re-entry, modal disclosure all good. |
| P6 | **10/10** | "Connect your accounts," "Anthropic key," "Telegram bot" — all role-based. |
| P7 | **10/10** | "The AI's brain. Also how you'll pay for its thoughts." Sentence-shaped throughout. |
| P8 | **10/10** | Every input has label + hint + how-to link. |
| P10 | **10/10** | Skip + Back are non-destructive. |

**Aggregate (excluding N/A): ~9.4** — improvement over rubric's 7.7 score.

## 2.3 — Install screen (`InstallStep.tsx` — the workhorse)

This is the longest, most-state-heavy step. The screen Karen sees is generally calm, but a lot is happening underneath.

**Happy path UI:**
- Title: "Setting up your assistant" — ✅ outcome.
- Subtitle: live-updating to current sub-step's label, e.g., "Check your computer…" — ✅ Principle 9 (loading states have context).
- "About X minutes remaining" estimate (`InstallStep.tsx:373-378`) — ✅ time budgeting visible.
- 4 sub-step checklist:
  1. "Check your computer" — ✅ friendly
  2. "Download the AI parts" — ✅ "AI parts" is gentle
  3. "Build your assistant" — ✅ outcome-framed
  4. "Test safety checks" — ✅ outcome
- Per-step status glyph (Circle pending → Loader2 spinning → Check succeeded → Filled circle failed) — ✅ good visual language.
- Per-step elapsed timer for the running step — ✅ feedback.
- Pulsing rings animation around the icon — ✅ visual life.
- "Show technical details" toggle hides the raw stream — ✅ Principle 5.
- Auto-advance to ReadyStep after 1s on success — ✅ smooth.

**Frictions on the happy path:**
- ⚠️ **P1, Principle 1:** When Karen clicks "Show technical details," she sees lines like:
  - `→ openclaw-vault: setup` (`InstallStep.tsx:221`)
  - `→ openclaw-vault: start` (`InstallStep.tsx:223`)
  - `→ clawhub-forge: setup` (`InstallStep.tsx:234`)
  - `Container runtime: podman` (`InstallStep.tsx:175`)
  - `Fetching assistant modules…` (`InstallStep.tsx:192`) — "modules" is borderline.
  - All actual stream output from `make setup` and `make start` (which prints podman commands, container names, image hashes, etc.).

  The whole point of progressive disclosure (Principle 5) is that POWER USERS who choose to peek don't get confused. But these specific lines mix internal codenames (openclaw-vault, clawhub-forge) into copy that the wizard authored — they could just as easily say "→ Your Assistant: install" / "→ Your Assistant: start" / "→ Skill Scanner: install." That's a pure copy fix.
  
- ⚠️ **P2:** "Fetching assistant modules…" (`InstallStep.tsx:192`) — could be "Downloading the assistant…" Plain.
- ⚠️ **P2:** Build sub-step's underlying `make setup` and `make start` stream output is a torrent of podman commands. Even with API-key sanitization (`sanitizeLine`, lines 59-66), it's a wall of dev-speak. **Cannot be cleaned at the React layer** — would need cooperation from the underlying make targets. For now, "behind progressive disclosure" is acceptable; just flag for Pass 7.

**Failure paths — where the wizard shines:**

The error handling is genuinely good and follows a 3-tier severity model defined in `app/src/lib/errors.ts`:

- **Level 1 (silent retry):** `withRetry()` wraps each pipeline stage; first failure is silently retried (`InstallStep.tsx:190-198, 219-229, 232-239, 245-266`). Karen sees `retryAttempt` increment in the running label but no scary banner.
- **Level 2 (FriendlyRetry):** if the second attempt also fails AND the error is classified as `retryable: true`, FriendlyRetry component shows. Calm, blue, "Try again" CTA.
- **Level 3 (ContactSupport):** non-retryable or persistent failures route to ContactSupport. Higher-friction but still humane.

The `classifyError()` function (`app/src/lib/errors.ts:200-228`) is one of the strongest UX assets in the codebase. 14 patterns map raw errors like `ECONNREFUSED`, `401 unauthorized`, `ENOSPC`, `EACCES`, `Path traversal`, `Manifest parse error`, etc. to a `{title, userMessage, suggestedAction}` triple. Examples:
- `ECONNREFUSED` → "Can't reach the network" + "Check your wifi connection and try again."
- `401 unauthorized` → "Your AI key isn't working" + "Open Preferences and update your key."
- `ENOSPC` → "Your computer is out of space" + "Free up some space and try again."

This is rubric Principle 3 done right.

**Frictions in the failure paths:**

- ⚠️ **P1, Principle 3:** `UNKNOWN_FALLBACK` (`errors.ts:191-198`):
  - title: "Something went wrong"
  - userMessage: "Something didn't work as expected."
  - suggestedAction: "Let's try again — if it keeps happening, get help."
  
  The rubric anti-patterns list (`docs/specs/2026-04-20-ux-principles-rubric.md:311`) called out "Something went wrong" by name as a thing-not-to-say. The fallback IS this. Pass 7 should think about whether this fallback can be more contextual — e.g., based on which sub-step (`check`/`download`/`build`/`safety`) was running, the fallback could read: "Your computer check didn't work as expected" or "We couldn't finish building your assistant." Still safer than the generic.

- 🚨 **P0, Principle 1:** `MissingRuntimeCard` (`InstallStep.tsx:490-558`) — this is the screen Karen sees if Podman/Docker is missing.
  - "You'll need **Podman or Docker** installed first." (`InstallStep.tsx:504-508`) — the names appear unmasked.
  - Linux block (`InstallStep.tsx:511-518`) shows the literal command `sudo apt install podman podman-compose` and frames it as "Run this in your terminal." → **The rubric specifically called this out at line 246 ("terminal jargon leak")** and it's still here. Pass 5 must fix this.
  - macOS/Windows blocks (`InstallStep.tsx:519-540`) just link to podman-desktop.io with "Download Podman Desktop and run the installer." → Better but still names "Podman."
  - Recommended Pass 5 fix: rebrand to "sandbox runner," offer per-platform-appropriate install paths, hide the raw command behind a "show command" disclosure, and ideally invest in detecting the user's package manager + offering one-click install (out of v0.2.x scope).

- ⚠️ **P1:** `Error("...exited with code ${event.payload.exit_code}")` (`InstallStep.tsx:140-145`) — this raw error is constructed before being passed to `classifyError`. classifyError won't recognize it (none of its patterns match this format), so it falls through to UNKNOWN_FALLBACK. The wizard would tell Karen "Something didn't work as expected" without context. Worth wiring this specific error pattern into `errors.ts` with a more useful user message — e.g., "[step name] couldn't finish — usually a transient issue."

- ⚠️ **P1:** `throw new Error("Some assistant modules failed to download")` (`InstallStep.tsx:204`) and `throw new Error("Workflow ended with status: ${r.status}")` (`InstallStep.tsx:260`) — these likewise won't match any `classifyError` pattern. UNKNOWN_FALLBACK applies. Same fix: add specific patterns or pre-classify before throwing.

- ⚠️ **P2:** Telegram URL prefetch (`InstallStep.tsx:630-646`) silently sets `telegramBotUrl` to null on failure. The Ready screen then falls back to `https://telegram.org` (generic) instead of the Karen's actual `t.me/<botname>`. → Karen lands on telegram.org without any indication of which bot to talk to. Should at minimum log a toast "Couldn't auto-open your bot — find it in Telegram with the username you set." (Cross-references Moment 3.)

**Score (Install screen):**

| Principle | Score | Notes |
|---|---|---|
| P1 | **6/10** | Internal codenames in the technical-details panel + MissingRuntime card surfaces "Podman" + `sudo apt install` directly. |
| P2 | **9/10** | Mostly outcome-framed. "Test safety checks" is good. |
| P3 | **8/10** | Three-tier error model is excellent BUT three thrown errors don't map to specific patterns and fall through to "Something went wrong" generic fallback. |
| P4 | **10/10** | `withRetry` on every step normalizes transient failures perfectly. |
| P5 | **10/10** | Technical-details disclosure works. |
| P6 | **8/10** | "Container runtime" + "AI parts" + "modules" + container names in the log — partial. |
| P7 | **10/10** | "About 2 minutes remaining," "Check your computer…" all sentence-shaped. |
| P9 | **10/10** | Loading states have great context — substep label + elapsed time. |
| P10 | **10/10** | No destructive actions. |

**Aggregate (excluding N/A): ~8.8.** Up from rubric's 8.1, but the 6/10 on P1 is the hot spot.

## 2.4 — Ready screen (`ReadyStep.tsx`)

**Karen's read:**
- Title: "Your assistant is ready! 🎉" — ✅ celebratory. The 🎉 is the only emoji in the codebase user-facing copy AFAIK; appropriate for the moment of victory.
- Body: "Say hi on Telegram to get started." — ✅ concrete next action.
- Primary CTA "Open Telegram" with chat-bubble icon (`ReadyStep.tsx:70-78`) → opens via `@tauri-apps/plugin-shell`'s `openUrl(settings.telegramBotUrl ?? "https://telegram.org")`. → ⚠️ if `telegramBotUrl` is null (prefetch failed), fallback is generic `telegram.org`. Karen lands there confused.
- Secondary CTA "Go to dashboard" — ✅ allows skipping past Telegram opening.
- "💡 Tip: You can ask your assistant things like 'What's the weather?' or 'Plan my Tuesday.'" (`ReadyStep.tsx:94-97`) — ✅ practical example prompts.
- Auto-advance to dashboard after 5s with a "Stay here" cancel button (`ReadyStep.tsx:100-114`) — ✅ great UX: gives agency without trapping Karen on a "click to continue" screen.
- Hand-rolled celebration SVG (logo + confetti) — ✅ on-brand.

**Score:**

| Principle | Score | Notes |
|---|---|---|
| P1 | **10/10** | No plumbing visible. |
| P2 | **10/10** | "Your assistant is ready" is pure outcome. |
| P5 | **10/10** | Auto-advance with cancel = perfect progressive interaction. |
| P6 | **10/10** | "Telegram," "dashboard" — all role-based. |
| P7 | **10/10** | Sentence-shaped throughout. |
| P10 | **10/10** | No destructive actions. |

**Aggregate: 10.0** — matches rubric.

## 2.5 — Cross-cutting wizard concerns

- **`WizardProgress.tsx`** — 4-dot progress bar above Connect/Install/Ready. Hides on Welcome. Uses `aria-current="step"` and per-dot `aria-label`. → ✅✅ accessibility is well-considered.
- **State persistence** — `useWizardProgress` hook persists `progress.step` so Karen can close and reopen the wizard mid-flow and resume. (`Setup.tsx:24-30`.) → ✅ recovery from interruption.
- **`Setup.tsx:32-44`** — `advance()` calls `recordStep()` after navigating; `goBack()` symmetrically rewinds. Clean.
- **The wizardCompleted flag is only set on `complete()`** (`Setup.tsx:46-49`). Karen who closes the app mid-Install never reaches `wizardCompleted`. App-relaunch routes her back to /setup, but `useWizardProgress` resumes her at `install` step. ✅ correct.

## 2.6 — The Moment 2 → Moment 4 lifecycle bridge (cross-pass)

A subtle but important friction emerges at the END of Moment 2:

When `InstallStep` finishes, `make start` has already brought up the four containers via `streamOneCommand("openclaw-vault", "start", "build")` (`InstallStep.tsx:223-224`). Containers are now running.

Karen then clicks "Open Telegram" or "Go to dashboard," eventually closes the app. **Containers stay up indefinitely.** The Phase 1 audit confirmed this: app close ≠ perimeter down, app crash ≠ perimeter down.

**This means Moment 2 currently sets up a hidden expectation that Pass 4 must close.** Today, after first-run install:
- Restart computer → containers are gone (because Podman stops on shutdown by default), and the next time Karen opens the app, the perimeter isn't running, but the app shows "Your assistant is ready" anyway because `wizardCompleted=true`.
- App relaunch → no auto-start of perimeter; the assistant is silently broken.

This isn't a friction in Moment 2's UI — it's a friction in Moment 4 caused by Moment 2's silent assumption. Documented here so the Pass 4 lifecycle work links back to it.

## 2.7 — Concrete fixes for Pass 5/7 (priority-ranked)

### P0 (Pass 5, must fix)

1. **MissingRuntimeCard Linux block** (`InstallStep.tsx:511-518`) — replace `sudo apt install podman podman-compose` direct exposure with friendly install guidance. At minimum: hide the command behind a "Show technical command" disclosure and frame it as "If you're comfortable with the terminal, run this; otherwise click 'Open guide'." The rubric called this out at line 246 a week ago and it's still live.
2. **Internal codenames in technical log** (`InstallStep.tsx:175, 192, 221, 223, 234`) — replace `openclaw-vault`, `clawhub-forge`, "Container runtime," "modules" strings with user-facing labels. Pure copy work.
3. **`MissingRuntimeCard` rebrand** — "Podman or Docker" → "sandbox runner" (or similar). Currently surfaces dev tool names directly to Karen.

### P1 (Pass 5/7)

4. **Wire 3 thrown errors into `classifyError`** (`InstallStep.tsx:140-145, 204, 260`) — currently fall through to `UNKNOWN_FALLBACK` which says "Something went wrong" (an explicit anti-pattern from rubric).
5. **`UNKNOWN_FALLBACK` itself** (`errors.ts:191-198`) — make context-aware based on which sub-step was running. Current generic copy is the rubric's named anti-pattern.
6. **Telegram URL prefetch failure** (`InstallStep.tsx:630-646`) — needs a user-visible signal at the Ready screen if the bot URL couldn't be derived. Currently silently falls back to `telegram.org`.
7. **`ConnectStep` read-config error path** (`ConnectStep.tsx:163-169`) — route through `classifyError` like InstallStep does, instead of dumping `err.message`.

### P2 (polish — only if budget)

8. Save-confirmation toast on Continue when Karen had pre-existing keys.
9. Inline screenshots in `HowToModal` (planned at line 94, currently `TODO E.4`).
10. Replace "Anthropic API key" label with "Anthropic key" or similar.
11. Replace "Fetching assistant modules" with "Downloading your assistant."

## 2.8 — Aggregate score — Moment 2

| Screen | Aggregate (this pass) | Rubric (2026-04-20) | Movement |
|---|---|---|---|
| Welcome | 10.0 | 10.0 | flat |
| Connect | 9.4 | 7.7 | **+1.7** (heading rename + show/hide + smart-paste already shipped) |
| Install | 8.8 | 8.1 | **+0.7** (time estimate + per-step labels shipped) |
| Ready | 10.0 | 10.0 | flat |

**Wizard moment overall: ~9.5/10.** With the P0 fixes from list above, lands at ≥9.7. **The wizard is the strongest user-facing surface in the product** and was the area I expected to surface the most friction. Most of the heavy lifting was done in earlier sprints; what's left is the dev-language leakage in the Install step and the MissingRuntime card.

**This contrasts sharply with the landing page (Moment 1, ~6.2).** Karen's clunkiest moment is BEFORE she opens the app, not after.



---

# Moment 3 — First chat (Telegram pairing → first message → response)

**Surface:** External — Karen leaves the opentrapp app and goes to Telegram.

**Karen's path:**
1. Clicks "Open Telegram" on `ReadyStep.tsx:70-78` → opens `t.me/<botname>` in default browser/Telegram app.
2. Taps "Start" in Telegram → sends `/start` to her bot.
3. Bot replies. (First-reply content lives in the `openclaw-vault` submodule — out of parent-repo scope. Bot uses `anthropic/claude-haiku-4-5` per `components/openclaw-vault/config/split-shell.json5:64-69`. No custom system prompt visible in this repo, so Karen sees OpenClaw's default greeting whatever that is.)
4. `channels.telegram.dmPolicy: "pairing"` (`tool-manifest.yml`) — means Karen has to PAIR her Telegram chat with the bot before it will respond to her. **This is undocumented friction Karen will hit on first contact.** The OpenTrApp app does not warn her she'll need to pair.

## Frictions

- 🚨 **P0, Principle 3:** Telegram URL prefetch failure mode (already named in Moment 2.3). If `deriveTelegramBotUrl()` returns null, "Open Telegram" sends Karen to `https://telegram.org` (the *generic* Telegram landing page), not her bot. She has to manually search Telegram for her bot's username — but she may not remember it. OpenTrApp doesn't show her bot's username anywhere in the UI as a fallback.
- 🚨 **P0, Principle 8:** No paired-vs-unpaired guidance. Karen taps Start, bot might require pairing (depending on OpenClaw config), and there's no signal in opentrapp telling her this is normal or how to recover.
- ⚠️ **P1, Principle 3:** If Karen's API key is invalid or out of credit, the bot replies with whatever OpenClaw's error template says — entirely outside our control. The Anthropic billing gotcha noted in `project_decisions.md:115-124` (workspace spend limit ≠ credit purchase) is a real failure mode here. OpenTrApp could pre-warn Karen at install time by doing a 1-call ping to the API — but doesn't.
- ⚠️ **P2:** No "first time?" hint card on the Ready screen explaining what to type in Telegram. (Existing tip says "What's the weather?" or "Plan my Tuesday" but doesn't explain the `/start` mechanic.)

## Score (Moment 3)

| Principle | Score | Notes |
|---|---|---|
| P3 | **5/10** | Multiple failure modes (no bot URL, unpaired bot, invalid key, no credit) and opentrapp gives Karen no actionable guidance for any. |
| P8 | **6/10** | Ready screen mentions Telegram but doesn't prepare Karen for the pairing or `/start` mechanics. |

**Verdict:** Moment 3 has fewer screens to score but its failure modes are all silent — they look like the bot is broken, not like there's a recoverable issue. Worth a Pass 7 polish for at least the prefetch-failure case (show bot username as a fallback when URL derivation fails) and a small "How to talk to your bot" hint on the Ready screen.

> **Live-validation update (2026-04-29):** A Telethon harness re-run with 8 Karen-flavored prompts revised Moment 3's score from **5.5/10 → ~8.0/10**. The bot voice is friendlier and more graceful-on-failure than code-reading predicted. New P0s surfaced (terse `/start` greeting; "container", "web_search/web_fetch tool" leaks not in the GUI banned list). Pairing-flow gate remains unverified live (test account was pre-paired). Full findings: `docs/specs/2026-04-29-live-signal-first-chat.md`.

---

# Moments 4–7 — Returning use, monitoring, add-tool, download

**🚨 ROOT-CAUSE FINDING — applies to all four moments at once:**

After completing the wizard, Karen lands at `/` (`Home`). She uses the sidebar (`UserSidebar.tsx`) to navigate between five icon-routes: Home / Security / Discover / Preferences / Help. **Every single one of those five pages is a placeholder rendering `UserPlaceholder.tsx`.**

Tally (run 2026-04-28 in this walkthrough):

| Page | File | State |
|---|---|---|
| Home | `app/src/pages/user/Home.tsx` | PLACEHOLDER (Phase E.2.2) |
| Security | `app/src/pages/user/SecurityMonitor.tsx` | PLACEHOLDER (Phase E.2.3) |
| Discover | `app/src/pages/user/Discover.tsx` | PLACEHOLDER (Phase E.2.6) |
| Preferences | `app/src/pages/user/Preferences.tsx` | PLACEHOLDER (Phase E.2.4) |
| Help | `app/src/pages/user/Help.tsx` | PLACEHOLDER (Phase E.2.5) |

Same in dev mode: 10 of 10 dev pages are also placeholders rendering `DevPlaceholder.tsx`.

**`UserPlaceholder.tsx` (lines 11-36) renders:**
- A friendly icon
- A title (e.g., "Your assistant, at a glance")
- A summary that *describes the feature that doesn't exist yet* (e.g., "The hero status card, security/activity/spending tiles, and proactive alerts will live here.")
- A "Coming in **Phase E.2.2**" badge → 🚨 **P0, Principle 1:** internal phase code surfaced to user
- A `spec: docs/specs/ui-rebuild-2026-04-21/user-mode/08-home-dashboard.md` line in monospace font → 🚨 **P0, Principle 1:** spec file path surfaced to user

**Karen's actual experience after wizard:**

1. Wizard says "Your assistant is ready! 🎉"
2. She clicks "Go to dashboard"
3. She sees **a page that promises features and explicitly admits they don't exist yet** ("Coming in Phase E.2.2") with a **monospace `docs/specs/...` reference** that screams "developer prototype."
4. She clicks every other sidebar item — same experience, five placeholder screens in a row.
5. She bounces. The wizard's promise ("Your assistant is ready") is contradicted within five seconds by the destination Karen lands on.

**This is the biggest gap in the codebase between vision and shipped state.** It's not in the rubric's existing scoring (the rubric was scored 2026-04-20 and the rubric author appears to have scored *the wizard screens* but not these post-wizard placeholders).

**Promises Karen reads in the placeholder summaries** (which makes the gap painful):
- Home: "The hero status card, security/activity/spending tiles, and proactive alerts will live here."
- Security: "A friendly timeline of what your assistant has been doing — what it read, what it tried to visit, and which suspicious actions were blocked. Built on real activity data from the security perimeter."
- Discover: "A picture-book of things you can ask your assistant — plan a trip, summarise the news, draft an email. Tap a card and it sends the prompt straight to Telegram for you."
- Preferences: "Update your keys, change your monthly spending limit, choose which notifications to receive, control startup behaviour."
- Help: "Plain-language answers to common questions, with screenshots."

Every one of these is a promise. Karen reads them, gets excited, then sees "Coming in Phase E.2.X" and feels deceived.

## Implication for the polish-phase plan

**Pass 6 ("Dev-Tools-Lite Surface") in the original plan was scoped at ~3 days for "extend SecurityMonitor + add 2 new flows."** This finding **dramatically expands the scope** because there's nothing to extend — every page must be built from the placeholder shell.

Good news: detailed design specs exist at `docs/specs/ui-rebuild-2026-04-21/user-mode/`:

- `07-onboarding.md` — wizard (already shipped)
- `08-home-dashboard.md` — Home page design
- `09-security-monitor.md` — Security/activity timeline
- `10-preferences.md` — Six-section preferences (keys, spend, notifications, startup, etc.)
- `11-help-and-support.md` — Help with redacted-bundle export
- `12-use-case-gallery.md` — Discover with deep-link to Telegram

So Pass 6 isn't designing-from-scratch — it's implementing-from-spec. That's faster but still much more than 3 days. **Realistic estimate: 6–9 days for Pass 6** (longer than originally planned by 3–6 days). This may force a rebudget elsewhere, OR a deliberate choice to ship v0.2.x with only the *highest-priority* user pages real and the others still placeholder-but-friendlier.

**Recommended rebudget — open for user decision:**

- **Option A: Ship all 5 pages real** (~9 days). Most ambitious. Highest first-impression delivery. Compresses Passes 7+8.
- **Option B: Ship 3 critical pages real (Home, Discover, Preferences), keep Security + Help as friendlier placeholders** (~5 days). Pragmatic. Karen can complete real workflows; she sees "this part of the app is in active development" instead of "Coming in Phase E.X.Y."
- **Option C: Ship 1 critical page real (Home, the landing), ALL other pages get the "friendlier placeholder" treatment** (~3 days). Minimum viable. Saves Pass 6 budget for lifecycle + notifications work.

A "friendlier placeholder" replaces "Coming in Phase E.2.X" + spec path with something like: "Active development — visit our roadmap" or "Talk to your assistant on Telegram while we finish this section." No phase codes, no monospace spec paths.

## Per-moment specifics

### Moment 4 — Returning use

Karen reopens opentrapp after closing it. App routes her to `/` (`Home`). She sees the placeholder. She has no idea if her assistant is running, no status indicator, no "your assistant is healthy" reassurance.

This compounds with the **Phase 1 lifecycle finding**: containers might or might not be up depending on whether Karen rebooted, killed `podman`, etc. OpenTrApp doesn't surface this state — Home is a placeholder, no hero status card. Karen tests the assistant via Telegram, finds it's silent, doesn't know why.

- 🚨 **P0:** Returning-user state has zero feedback. Compounded by lifecycle gap.

### Moment 5 — Monitoring peek

Karen clicks "Security" in the sidebar (the shield icon). Lands on `SecurityMonitor.tsx` placeholder.

The placeholder summary specifically promises: "*A friendly timeline of what your assistant has been doing — what it read, what it tried to visit, and which suspicious actions were blocked. Built on real activity data from the security perimeter.*"

That data exists. The vault-proxy logs every request (per `project_decisions.md:100-112` post-redaction). The activity log COULD be rendered. It just isn't yet.

- 🚨 **P0:** the placeholder PROMISES the timeline exists. Karen will feel deceived.

### Moment 6 — Add a tool from openclaw network

There's no sidebar item called "Add a tool" or "Skills." Karen would either:
- Click "Discover" hoping it's there → placeholder.
- Click "Help" → placeholder.
- Give up and go back to Telegram, asking the bot to install something.

The infrastructure exists in `clawhub-forge` (a sub-repo) — there's a working scanner pipeline. The UI surface to invoke it from user-mode does not exist.

- 🚨 **P0:** The capability is real, the surface is missing. Pass 6 must build this from `12-use-case-gallery.md` or a sister spec.

### Moment 7 — Download from openclaw network

Same shape as Moment 6. Karen has nowhere to click. Forge can do safe-download via existing workflow; no GUI exposes it.

- 🚨 **P0:** Same as Moment 6.

## Score (Moments 4–7 collective)

Every placeholder page scores identically against the rubric:

| Principle | Score | Notes |
|---|---|---|
| P1 | **2/10** | "Phase E.2.X" + monospace `docs/specs/...` paths visible. |
| P2 | **3/10** | Promises of features Karen can't access — outcomes she can't have. |
| P5 | **5/10** | Progressive disclosure WORKS in the sense that a placeholder doesn't dump details — but it dumps the wrong thing (phase code + spec path) when it could be empty/calm. |
| P6 | **8/10** | Page titles ARE role-based ("Your assistant, at a glance," "Security & activity," etc.) — that's good. The placeholder just doesn't show real content. |
| P7 | **6/10** | Title sentences are good. Placeholder badge "Coming in Phase E.2.X" is a state token. |

**Aggregate: ~4.8/10 each, applied to all 5 user pages.** That's BELOW the landing page (~6.2). The lowest-scoring screens in the product are the ones Karen lands on after completing setup.

## Concrete fixes (priority-ranked)

### P0 — Pass 6 must address each of these

For each of the 5 user-mode pages (Home, Security, Discover, Preferences, Help):

1. **Either ship the real page** (per its spec at `docs/specs/ui-rebuild-2026-04-21/user-mode/0X-...md`), **or replace `UserPlaceholder.tsx` with a "friendlier placeholder."**
2. **Friendlier placeholder must:**
   - Drop the "Coming in Phase E.2.X" copy entirely. Replace with "We're still building this section."
   - Drop the `spec: docs/specs/...` line entirely.
   - Suggest a current-best-action: "Talk to your assistant on Telegram" + Telegram icon button.
3. **Same treatment for `DevPlaceholder.tsx`** (10 pages). Lower priority because dev mode is gated behind Cmd/Ctrl+Shift+D and the ModeSwitcher dialog warns "Changes here can break your setup," so power users who reach dev placeholders are already in advanced territory. But the phase codes still violate P1 — clean them at the same time.

### P1 — Strongly recommend at least Home is real for v0.2.x ship

Home is the landing page after the wizard. If anything is real, this should be real. The product identity spec already mocks the dashboard at `docs/specs/2026-04-19-product-identity-spec.md:117-144`:

```
Your AI Assistant                      ● Running
Talk to your assistant: Open Telegram → message @YourBot
[Skills (25)] [Security ✓ Safe] [Stop]
What your assistant can do: ...
[Skill Store] [Agent Network — Coming Soon]
```

Karen completing the wizard and seeing THIS dashboard would feel completely different from seeing the placeholder.

### P2 — Eventually all 5 should be real

The specs exist. The infrastructure exists (vault-proxy logs, forge scan, use-case gallery data). The work is presentation-layer integration. Big chunk but not novel.

---

# Moment 8 — Crash & recovery

**Surface:** The whole system (app + perimeter).

**Karen's path tested:**
1. Karen has app open, perimeter running.
2. Karen's laptop battery dies. App crashes. Containers stay up (per Phase 1 finding).
3. Karen reboots laptop. Podman doesn't auto-restart on boot by default.
4. Karen reopens opentrapp. App auto-routes to `/` (Home placeholder). No status indicator.
5. Karen messages bot via Telegram. Silent — bot is offline because containers aren't running.
6. Karen has no recovery path within the app.

## Frictions (compounding all of the above)

- 🚨 **P0:** Phase 1 lifecycle audit confirmed: app start ≠ perimeter up; app close ≠ perimeter down; SIGKILL leaks containers; no health watchdog. **Pass 4 closes this gap.** Documented in detail at the top of the plan file under "B — App owns perimeter lifecycle."
- 🚨 **P0:** Even WITH lifecycle ownership, Karen needs a recovery UI: "Your assistant wasn't running cleanly when you closed me last time. Restored." Or: "Something interrupted your assistant — try again." Pass 7 territory.
- 🚨 **P0:** SIGKILL of the Tauri process leaves orphan containers. Without Pass 4's signal handlers, this is permanent state pollution: Karen's `podman ps` shows zombie containers from a session that ended hours ago. Pass 4 RunGuard must close this.
- ⚠️ **P1:** No tray indicator that updates on perimeter state change (`app/src-tauri/src/lib.rs:59-64` — placeholder "initializing" never updated).

## Score (Moment 8)

| Principle | Score | Notes |
|---|---|---|
| P3 | **2/10** | Recovery is invisible to Karen. No errors, just silence. |
| P11 (proposed) | **0/10** | "Perimeter alive iff app alive" — doesn't hold today. |

Crash & recovery is the moment Pass 4 is designed for. It's the most architectural-work pass and the one that converts the app from a control panel into a true perimeter-lifecycle owner.

---

# Pass 1 Summary — Aggregate Findings

## Score landscape across the 8 moments

| Moment | Surface | Aggregate score | Severity |
|---|---|---|---|
| 1 | Discovery (landing page) | **6.2/10** | 3 P0 + 7 P1/P2 |
| 2 | First-run install + wizard | **9.5/10** | 3 P0 + 4 P1 + 4 P2 |
| 3 | First chat (Telegram) | **5.5/10** | 2 P0 + 1 P1 + 1 P2 |
| 4–7 | Returning use, monitoring, add tool, download | **~4.8/10 each** | 5 placeholders × 2-3 P0 each |
| 8 | Crash & recovery | **~1/10** | 3 P0 (lifecycle audit) |

**Aggregate Karen experience: the wizard is a 9.5/10 island surrounded by a sub-7/10 sea.** The strongest and weakest surfaces in the product are both encountered by every Karen who installs the app, in this order: landing (6.2) → wizard (9.5) → ... → all post-wizard pages (4.8) → eventually a crash event (1).

**The first-impression curve is wrong-shaped.** Karen is most impressed during install, then disappointed forever after. The polish phase needs to flatten that curve — bring landing UP, bring post-wizard UP, and bring crash recovery from invisible to graceful.

## The single biggest finding

**5 user-mode pages and 10 dev-mode pages are all placeholders.** This was not surfaced in the rubric's 2026-04-20 scoring (which audited the wizard screens but skipped the placeholder pages — likely scoring them as "out of scope until built"). The 3-week polish plan's Pass 6 is now significantly under-budgeted.

## Recommended rebudget (for user decision next session)

The plan's original budget allocates 2-3 days for Pass 1 (this pass — already used) + 1-2 days Pass 2 (spec) + 1 day Pass 3 (rubric) + ~3 days each for Passes 4-6 + ~2 days Pass 7 + 1-2 days Pass 8 = ~17-20 days.

**Pass 6 reallocation options:**

- **Option A: Ship 5 user pages real** — Pass 6 grows to ~9 days. Pass 7 gets squeezed to ~1 day. Tighter.
- **Option B: Ship 3 user pages real (Home + Discover + Preferences) + 2 friendlier placeholders (Security + Help)** — Pass 6 = ~5 days. Best risk-adjusted ship.
- **Option C: Ship Home only real + 4 friendlier placeholders** — Pass 6 = ~3 days. Original budget holds. Most conservative.

I recommend **Option B**: Home is the landing-after-wizard, Discover is what unblocks Karen's productivity ("here are things you can ask"), and Preferences is what makes Karen feel in-control. Security and Help can wait one more sub-release — but their placeholders need the friendlier copy, not the current dev-leakage placeholders.

## What Pass 1 is NOT

- Pass 1 didn't actually live-run the app. All findings are from code reading. **Live running would surface dynamic friction** (UI lag, Tauri load timing, real Telegram round-trip behavior, real failure modes during install, animation jank). Recommend a brief live-run sub-pass — maybe 2 hours during Pass 5 or Pass 8 — for dynamic-friction validation.
- Pass 1 didn't walk the openclaw bot's persona/responses. The bot lives in a sub-repo and uses Claude Haiku 4.5 with no custom system prompt visible in the parent repo. Karen's bot experience is whatever OpenClaw defaults provide. If the bot says things like "I am a coding agent specialized in..." that's a Karen-friction the parent repo can't fix — would require a custom system prompt in the openclaw-vault submodule.
- Pass 1 didn't audit the openclaw-vault, clawhub-forge, or moltbook-pioneer submodules in depth. Their `component.yml` files surface to dev mode pages (which are themselves placeholders), so any leakage there only matters once dev mode is real.

## Hand-off to next session

- Findings doc complete (this file).
- All 8 moments documented; Moments 4-7 share the placeholder root cause.
- Next session implements fixes in priority order (P0 → P1 → P2) following the original 8-pass plan, with the Pass 6 rebudget decision pending user input.
- Suggest next session **starts** by asking the user the rebudget question (A/B/C above) before opening any code.


---

## Walkthrough log (chronological)

- **2026-04-28 (early)** — Moment 1 (Discovery / landing page) walked from code only. Findings above. Notable: landing page is currently the lowest-scoring user-facing surface in the product (~6.2/10), worse than the wizard.
- **2026-04-28 (mid)** — Moment 2 (Wizard) walked from code. The wizard is the strongest user-facing surface in the product. Connect step at 9.4 (rubric had it 7.7 — caught up via shipping). Install step at 8.8. ~3 P0s remain: MissingRuntime card "sudo apt install" jargon, internal codenames in technical log, "Podman or Docker" naming.
- **2026-04-28 (mid)** — Moment 3 (First chat) walked from code. External surface (Telegram) but OpenTrApp's handoff to Telegram has multiple silent-failure modes (URL prefetch, pairing, key invalid, no credit). No actionable guidance in any.
- **2026-04-28 (late)** — Moments 4-7 walked from code. **CRITICAL FINDING:** all 5 user-mode pages and all 10 dev-mode pages are placeholders rendering "Coming in Phase E.2.X" with monospace `docs/specs/...` paths visible to user. This was not in the 2026-04-20 rubric scoring. Dramatically expands Pass 6 scope.
- **2026-04-28 (late)** — Moment 8 (Crash & recovery) consolidated from Phase 1 lifecycle audit findings. Pass 4 closes the architectural gap; Pass 7 needs to add the recovery UI.

## End of Pass 1

This document is complete as of 2026-04-28. The next session's instance:
1. Reads this doc + the plan file.
2. Asks the user for the Pass 6 rebudget decision (A/B/C in the Aggregate Findings section).
3. Begins Pass 2 (write the aspirational target-state UX spec) using these findings as input.
