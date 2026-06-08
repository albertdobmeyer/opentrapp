# Spec — vault-agent image: conservative prune

**Status:** Validated — image 754 → 590 MB; live bot smoke (real LLM reply) passed
**Date:** 2026-06-06
**Author:** memory-optimization initiative, Phase 2 (`~/.claude/plans/glimmering-meandering-babbage.md`)
**Touches the security boundary?** No — see *Security implications*.

## Purpose

Shrink the `vault-agent` image (disk, download, cold-start) without changing what
the container can do or reach. Two motivations:

1. **Footprint hygiene.** The resting RAM win shipped in Phase 3 (idle auto-pause).
   This is the disk/download side of "runs on a small laptop": a smaller image is a
   faster, lighter install.
2. **Outreach positioning.** Part of pitching the perimeter as a containerization
   layer other CLI-agent projects (e.g. opencode) could adopt is that it is *lean
   and efficient*, not a bloated wrapper. A trim, defensible image is part of that
   credibility.

Expected payoff (honest): **disk/download/startup, on the order of tens of MB. Not
RAM** — the resident set is the Node/OpenClaw runtime, which a prune of on-disk,
unloaded modules does not change.

## Security implications

This change must **not** alter the security boundary. Specifically it does NOT:

- add Linux capabilities, change seccomp, or relax read-only / noexec / no-new-privs;
- change the proxy allowlist, tool policy, safeBins, shell levels, or `sandbox.mode`;
- add any binary back (the destructive-binary and package-manager removals stay);
- touch `entrypoint.sh`, the proxy bootstrap, or the Telegram proxy patch.

It removes only **unused files from `/usr/local/lib/node_modules`** (dependencies
OpenClaw does not load at runtime under proxy-only, no-local-LLM operation) and any
build caches. The one real risk is **functional, not security**: removing a module
that OpenClaw actually loads would stop the gateway from starting. That risk is
retired by the verification plan below — nothing commits until the bot responds.

The image digest changes, so the cosign signature re-pins on the next tagged build
and `images/image-digests.json` updates then (CI `build-images`, tag-only). Note in
the PR/commit.

## Approach — measurement first, prune surgically

The agent CLAUDE.md forbids guessing on this image. So:

1. **Measure.** Build the builder stage (or a throwaway `node:22-alpine` running the
   same `npm install -g openclaw@2026.2.26 --ignore-scripts`) and record
   `du -sh /usr/local/lib/node_modules/* | sort -h | tail -30`. This gives the real
   prune targets instead of assumptions.
2. **Identify unused trees.** Strong a-priori candidate: `node-llama-cpp` and any
   local-LLM / native-inference optional deps — the vault connects to Anthropic/
   OpenAI *through the proxy* and never runs a local model (this is exactly why the
   build already passes `--ignore-scripts`). Confirm against the measurement and
   against OpenClaw's actual startup imports before removing.
3. **Prune.** In the builder stage, after the install, remove the confirmed-unused
   module trees explicitly (`rm -rf` of named dirs) — preferred over a blanket
   `npm prune --omit=optional`, which is less auditable on a security image. Strip
   the builder npm cache. (The production stage already does
   `rm -rf /var/cache/apk/* /tmp/*` and copies only `node_modules`.)
4. **Keep it conservative.** One reviewable set of removals, each justified by the
   measurement + a "not imported at startup" check. No base-image swap (decided).

## Verification plan (nothing commits until all pass)

On a machine that can build + run the perimeter (this 7.2 GB dev box swap-storms, so
this runs elsewhere or after freeing RAM):

1. `podman images` size **before vs after** — record the delta (the headline number).
2. `make verify` — all **24 security checks** pass (unchanged from baseline).
3. **Bot smoke:** `make start`, confirm the OpenClaw gateway starts cleanly (logs),
   pair + send a Telegram message, confirm a normal reply. This is the check that
   matters — a wrongly-pruned dep shows up here.
4. `make profile-memory` (from the repo root) — confirm resident RAM is ~unchanged
   (expected) and the image is smaller on disk.

If any step fails, revert the prune and narrow the removal set. Only after all four
pass does the Containerfile change get committed; the digest re-pin rides the next
tagged CI build.

## Measurement findings (2026-06-07)

Built the builder stage in a memory-capped throwaway container and probed with
`node openclaw.mjs gateway --allow-unconfigured` (which surfaces eager
`ERR_MODULE_NOT_FOUND` before the config check):

- **node_modules baseline: 606 MB** (openclaw subtree 586 MB). Largest trees are
  unused integrations: node-llama-cpp 33M, koffi 28M, @larksuiteoapi 25M, @line
  19M, @mistralai 17M, @google 14M, @slack 12M, @aws-sdk 12M, @discordjs 8.5M,
  @whiskeysockets (WhatsApp) 9M, @cloudflare 9M, etc.
- **Removing integration packages is IMPOSSIBLE without breaking the bot.**
  OpenClaw's compiled `dist` bundle *statically imports every* provider and
  channel: removing `@aws-sdk` →
  `ERR_MODULE_NOT_FOUND ... imported from dist/auth-profiles-*.js`; removing
  `@slack` → `... imported from dist/send-*.js`. Confirmed by bisection
  (providers AND chat platforms are both eager). The "drop unused integrations"
  win would require patching OpenClaw's bundle — fragile, out of scope, rejected.
- **First strip attempt deleted `*.ts` — and that BROKE the bot.** OpenClaw ships
  its **extensions as TypeScript source** and loads them at runtime: there are 136
  `extensions/*/index.ts` entry points, **including `extensions/telegram`**.
  Deleting `*.ts` removed all 136 (gateway then logged
  `extension entry escapes package directory: ./index.ts` for each; baseline shows
  0 such warnings). The gateway still reached "listening", which is exactly why
  "gateway starts" is an insufficient check — it started with every extension,
  Telegram included, broken. Caught only by diffing baseline vs pruned.
- **Second attempt deleted `*.md` — and the LIVE BOT SMOKE caught that too.** With
  `*.ts` kept, the gateway started clean AND all 136 extensions loaded, so it looked
  safe. But sending a real Telegram message returned no reply, and the agent logged
  `handler failed: Error: Missing workspace template: AGENTS.md
  (.../openclaw/docs/reference/templates/AGENTS.md)`. **OpenClaw reads markdown
  workspace templates at runtime** — deleting `*.md` breaks the LLM reply path. Only
  the end-to-end bot smoke (not gateway-start, not extension-count) exposed it.
- **Final safe strip — declarations/sourcemaps/flow only:** `*.d.ts` (the bulk of
  the win in SDK-heavy trees), `*.map`, `*.flow`, plus the compile-time
  `@types`/`bun-types` packages. **No `*.ts`, no `*.md`, no package, no directory
  sweep.** Result: openclaw subtree **586 → 386 MB**, **image 754 → 590 MB
  (164 MB / ~22% off)**, all 136 extensions + `AGENTS.md` + all 1818 `*.md` intact.

**Decision:** ship the `*.d.ts`/`*.map`/`*.flow` + `@types`/`bun-types` strip.
Conservative (no executable code, no package, no security config, no `*.ts` source,
no `*.md`), ~164 MB off the image.

**Validation (all passed):** built baseline vs pruned; extension `index.ts` count
136 = 136; `AGENTS.md` present; gateway starts clean (0 `ERR_MODULE_NOT_FOUND`, 0
"escapes"); and the **gold-standard live bot smoke** — brought up `vault-agent`
(pruned) + `vault-proxy`, paired the test user, sent a Telegram message, and got a
real LLM reply ("PONG") with the Anthropic round-trip visible in the proxy log.

**Lesson (carry forward):** for this image, neither "gateway starts" nor "extensions
load" is sufficient — OpenClaw treats `*.ts` (extension source) and `*.md` (workspace
templates) as RUNTIME assets. Validation MUST end with a live message that produces a
real LLM reply. Only delete file types Node and the TS transpiler never read: `*.d.ts`,
`*.map`, `*.flow`, and `@types`-style declaration packages.

## Out of scope

- Removing unused integration packages — blocked by OpenClaw's monolithic eager
  imports (would need a bundle patch).
- npm/corepack removal from the final image (low value, cross-stage fiddly).
- Aggressive distroless / runtime-only base swap (separate, larger change).
- `vault-egress` / other images (this spec is `vault-agent` only).
