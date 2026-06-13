# Handoff — Active Mission

**Last updated:** 2026-06-12 (**Now executing the road-to-recommendable checklist. This session: authored `tests/boundary-selftest.sh` (the 1A/1B boundary gate, fail-closed) and shipped the 3C residual-risk front-door page — both pushed (`77a9204`, `00505f6`).** Phase B (headless daemon/viewer split, B1–B4b) done + CI-green; v0.7.2-rc1 published (pre-release) + announced (Discussion #73); daemon ships in every installer. Current shipped release: **v0.7.2-rc1** (pre-release; v0.7.0 is last stable).)
**Current phase:** Working [`road-to-recommendable.md`](road-to-recommendable.md) top-to-bottom. The gate is **Tier 1**, and its load-bearing items need **capable hardware** (the dev box swap-storms running the full perimeter). The strategy: author every executable artifact (scripts/playbooks) from the dev box so each hardware item is **one command**, then run them on the Windows box / a cloud VM.
**Branch:** `main` — pushed; `v0.7.2-rc1` tag → published pre-release. Monorepo (ADR-0013); `app/src-tauri` is now a Cargo workspace (`opentrapp-core` + `opentrapp-daemon`).

> ## ⟶ NEXT SESSION — READ THIS FIRST: the road from "built" to "recommendable public security tool"
>
> Phase B (the headless daemon/viewer split, [ADR-0019](adr/0019-headless-daemon-gui-viewer-split.md)) is
> implemented end-to-end and CI-green; v0.7.2-rc1 ships it. **The architecture is done.** What separates it
> from a tool we can publicly recommend as an *official* security layer for open agent systems (OpenClaw
> et al.) is **verification at the consumption end on real hardware** — which this dev box physically can't
> do (it swap-storms running the full perimeter). The critical path runs through **capable hardware** (the
> Windows box / a cloud VM). This is a §11 problem, not an architecture problem.
>
> ### Landed 2026-06-12 (this session) — first checklist execution
> - **1A/1B — `tests/boundary-selftest.sh` authored** (`make boundary-selftest`, commit `77a9204`). Six
>   boundary checks grounded in the live wiring — B1 network isolation, B2 L7 allowlist (403 / not-403),
>   B3 vendor-credential injection, B4 L3 `vault_egress_drop_private` set, B5 proxy-CA fingerprint pinning,
>   B6 read-only skill delivery. **Fail-closed**: exit 1 on any failure, exit **2 on "cannot assess"** (down /
>   tool missing) — never a false green (§11). `bash -n` + all exit-code paths verified on the dev box; the
>   boundary assertions themselves are **🔶 unrun pending hardware**. Doubles as the daemon's resume self-test
>   (1B / #45). Also **fixed the checklist's credential grep**: `TELEGRAM_BOT_TOKEN` legitimately lives in
>   the agent (compose:69) — only the Anthropic/OpenAI key is proxy-injected.
> - **3C — `docs/what-this-protects.md` shipped** (commit `00505f6`). Plain-language T1–T6 distillation, the
>   "does NOT" half given equal weight, linked front-and-center from README **Values** + top of **Limitations**.
>   Checklist 3C ✅ — the one Tier-3 item that needed no hardware.
> - **2A/2B — soak + red-team artifacts authored** (commit `03c2245`). `tests/proxy-memory-soak.sh`
>   (`make proxy-soak`, RSS over load×time + leak verdict) and `tests/red-team-breakout.sh` (`make red-team`,
>   R1–R7 breakout battery, CONTAINED/BREACH, fail-closed). Lint + exit-code paths verified; 🔶 unrun pending hardware.
> - **#45 — daemon runs the boundary self-test on every (re)start, fail-closed (IMPLEMENTED + CI-green).**
>   Landed in two CI-verified slices: `opentrapp_core::selftest` embeds the script (`include_str!`) + maps
>   exit→Verdict (slice 1, `7cf0730`); `supervisor::verify_boundary_fail_closed` runs it after cold start /
>   resume / restart — Fail→stop+`boundary-failed` marker, CannotAssess→alert, Pass→clear (slice 2, `c8d4afc`).
>   **Opt-in `OPENTRAPP_SELFTEST_ON_RESUME` (default OFF, §11)** → shipping behavior byte-unchanged until
>   hardware-verified. `opentrapp-daemon --boundary-selftest` = on-demand operator check. ADR-0018 addendum
>   documents the resumed==cold contract. The script is *embedded*, so there is **no packaged-resource staging
>   to get wrong** — the daemon is self-contained. **Remaining (hardware):** flip the opt-in on, run green
>   cold + every resume path, then promote opt-in→default.
> - **1E — code-signing CI scaffolded** (commit `66750fc`, then **fixed in `719cc19`**). **Both** macOS and
>   Windows are now **commented ready-to-activate templates** (NOT live). Windows: SignPath template (inline
>   activation checklist) — not live because slugs come from the OSS account + every `uses:` must be SHA-pinned
>   (Scorecard). macOS: the six `APPLE_*` env lines, added only once the secrets are real. **Remaining = human
>   only:** Apple Developer Program + `APPLE_*` secrets; SignPath OSS approval + SHA-pin + `SIGNPATH_*` secrets.
>   See `code-signing-policy.md`.
>   - **⚠️ CI regression + fix (the §11 lesson of the session):** `66750fc` wired the macOS `APPLE_*` env
>     LIVE, assuming an empty `APPLE_CERTIFICATE` = "skip signing". It does NOT — `tauri` treats a
>     *present-but-empty* cert as "sign now", runs `security import` on a blank cert, and **fails the macOS
>     `.app` bundle**. Build (macOS Intel + ARM) went red `66750fc`→`2dc09aa` (Linux/Windows stayed green;
>     the Rust/contract gates were never affected). **`719cc19` reverts the live env to a commented template;
>     CI verified green on `719cc19` (all four platform builds success).** Takeaway: a workflow edit is only
>     "inert" once a *real build* proves it — YAML-valid + GitHub-accepted is the producing end, not the
>     consuming end.
>
> **The dev box is now tapped out** — every checklist item authorable without the perimeter is done + pushed.
> Everything remaining needs the Windows box / a cloud VM (run the `make` targets, idle/defer tests) or an
> external human (Apple/SignPath certs, third-party review). Resume on capable hardware per the runbook below.
>
> ### Landed prior session (2026-06-09 → 06-12) — Phase B
> - **Phase B daemon split — FULL (B1–B4b), CI-green on all platforms.** `opentrapp-core` (tauri-free) holds
>   the orchestration core + marker contract; `opentrapp-daemon` owns the perimeter (runguard → up →
>   idle-supervise + waker → teardown) + a durable control channel; it **ships as a sidecar in every
>   installer** (verified inside the `.deb`: `usr/bin/opentrapp-daemon`, 5.8 MB, WebKit-free). The GUI can
>   **defer** to it (viewer mode) behind opt-in `OPENTRAPP_DAEMON_DEFER=1` (default OFF → behaves exactly
>   like before). CI asserts the daemon graph has no WebKit.
> - **Phase A leanness gate-verified** live (close dashboard → ~211 MB freed, no leak) — footprint §10.4.
> - **Phase C** generic per-component dashboard (dev mode) — the manifest-projection vision.
> - **v0.7.2-rc1** cut + published (pre-release) + announced (Discussion #73 — **not yet pinned**; pinning
>   is GitHub-UI-only, no API). PR board cleared (14 Dependabot closed-to-regenerate, #56 re-applied fresh).
>
> ### The road to public recommendation (prioritized — this is the real "what's left")
> **Tier 1 — load-bearing (a security tool's claims must be *verified*, not asserted — §11):**
> 1. **Boundary self-test on real hardware, cold-start AND resume** (WS0-0b, tasks #39/#40). **Script now
>    authored** (`make boundary-selftest`) — running it green on a cold perimeter + every resume path is the
>    one remaining step, plus wiring the daemon to run it on (re)start, fail-closed (#45). **This is THE gate.**
> 2. **Idle auto-pause + wake verified in production** (WS0-0a, task #35) — the headline feature firing and
>    waking *exactly once* under a real agent (the box could never run this end-to-end).
> 3. **Code signing** — **CI now scaffolded** (decision 2026-06-12: scaffold *both* Windows + macOS).
>    both macOS + Windows are commented ready-to-activate templates (macOS was briefly live but broke the
>    build — fixed in `719cc19`). Remaining is human
>    procurement only — see the signing decision in "RUN THIS NEXT — resubmit SignPath" below.
> 4. **Daemon-split defer verified + promoted** — run `docs/b4b-hardware-test-plan.md` (7 tests); if it
>    passes, flip `OPENTRAPP_DAEMON_DEFER` opt-in → default to actually deliver the lean background process.
>
> **Tier 2 — hardening:** proxy RSS bounded over (load × time) so a days-long run can't leak (WS1, #41/#42);
> an **adversarial / red-team pass** (can a compromised agent actually break out of the perimeter?); ideally
> a **third-party security review** (the gold standard for "official security tool").
>
> **Tier 3 — trust polish:** cut a **stable** release (not an RC) once Tier 1 verifies; tighten the
> reproducible-build + SBOM/cosign story; **✅ residual-risk front-door page done** ([what-this-protects.md](what-this-protects.md)).
>
> ### Next session — tackle every item we can (DUAL PATH — pick by where you're running)
>
> **▸ If on the DEV BOX (this machine — can't run the perimeter, CI compiles Rust):** the executable
> artifacts are now all authored — the dev-box authoring backlog is nearly exhausted. What's left here:
> 1. ✅ **#45 — daemon runs `boundary-selftest.sh` on every (re)start, fail-closed** — DONE, CI-green
>    (slices `7cf0730` + `c8d4afc`), behind opt-in `OPENTRAPP_SELFTEST_ON_RESUME`. Script *embedded* in the
>    daemon (no staging). Remaining is hardware-only (enable + verify).
> 2. ✅ **2A `tests/proxy-memory-soak.sh`** + ✅ **2B `tests/red-team-breakout.sh`** — authored, lint-clean.
> 3. ✅ **#55 / 1E — signing CI scaffolded** (`66750fc` + fix `719cc19`): both macOS + Windows are commented
>    ready-to-activate templates (the live macOS env broke the build; reverted to a template).
> 4. **Dev box is now tapped out** — everything else needs the perimeter or an external human.
>
> **▸ If on CAPABLE HARDWARE (Windows box / cloud VM — can run the full perimeter):** execute, top-down.
> Every test below is now a single `make` target:
> 1. `make perimeter-up` → `make boundary-selftest` (cold; first run pins the CA baseline) → all-PASS. **1A.**
> 2. `export OPENTRAPP_SELFTEST_ON_RESUME=1` and run the daemon so it self-tests on (re)start; re-run
>    `make boundary-selftest` after each resume: user-pause→resume, idle-dormant→wake, daemon kill→restart.
>    Fail-closed on any mismatch. **1B** (#45 — then promote the opt-in to default).
> 3. Leave a real agent idle past threshold → Dormant → Telegram message → wakes + replies **exactly once**;
>    measure cold-start latency. **1C** (#35), assert boundary+exactly-once (#40).
> 4. Run `docs/b4b-hardware-test-plan.md` (7 tests, record RSS) → if green, flip `OPENTRAPP_DAEMON_DEFER`
>    opt-in→default + record resting RSS in footprint §10.4. **1D.**
> 5. `make proxy-soak --duration 360` → attribute growth, apply fix (**2A/2B**, #41/#42); `make red-team`
>    cold + with a hostile skill loaded → all CONTAINED (**§2B**, #54).
>
> ### Read first
> [ADR-0019](adr/0019-headless-daemon-gui-viewer-split.md) · [b4b-hardware-test-plan.md](b4b-hardware-test-plan.md)
> · [footprint §10.4](footprint-and-device-usability.md) · `app/src-tauri/crates/{core,daemon}/` ·
> [threat-model.md](threat-model.md) (the basis for the Tier-1 boundary tests).
>
> ### Secondary / standing
> - **opencode pitch** (`docs/pitch-opencode.md`, gitignored — do NOT commit) is send-ready + refreshed
>   2026-06-12; only the human send remains (see the older callout below). Scoped to the skills-scanner
>   pointer, NOT the perimeter.
> - **Dependabot** will re-open fresh PRs against current `main`; review as a batch — merge the patch bumps,
>   eyeball the majors individually (lucide-react 0→1, eslint 9→10, actions/upload+download-artifact 4→7/8).
> - **Pin Discussion #73** in the GitHub UI (`···` → Pin discussion — there's no API for it).

> ## ⟶ Fixed this session (2026-06-08, session 3): packaged first-run credential dead-end
>
> The Karen v0.6 E2E reproduced a **shipped high-severity bug**: on a packaged AppImage,
> entering the API key + bot token and clicking **Continue** on the wizard's Connect step
> returned a "setting could not be saved" toast — no way forward, first-run dead-ended.
>
> **Root cause:** the wizard wrote keys via `writeConfig("agent",".env")` — the generic
> *component-config* editor, which resolves into the agent **component directory**. On a
> packaged first-run that directory is the **read-only AppImage bundle** (the writable staged
> copy is only created later, inside the credentials-gated bootstrap → chicken-and-egg). The
> write failed; the error was also mislabeled "settings". Dev source trees are writable, so it
> never reproduced in dev — packaged-only.
>
> **Fix (commit `80e4dfa`):** two dedicated Tauri commands `save_credentials` / `read_runtime_env`
> write+read the **runtime** `.env` (`~/.opentrapp/.env`) directly — where `bootstrap::step_write_env`
> and the perimeter actually read it — upsert + preserve other vars + `0600`. Converted **all four**
> runtime-`.env` credential sites off the component-dir path (`ConnectStep`, `ActivationModal`,
> `Preferences` key-rotation, `install-step` prefetch) to kill the whole bug class. Validated:
> tsc 0, eslint clean, vitest 87, orchestrator-check 114/0/0 (§5 confirms both new Rust commands
> have frontend wrappers), integration-test 24/0, and **CI all-green** including `Rust (check + test)`
> (compiles + 2 new unit tests `upsert_*`/`write_credentials_at`) and all 4 platform builds.
> **Remaining:** the packaged first-run *re-grade* needs a new tagged `v0.6.x` build (`build-images`
> is tag-only) — the code fix is done + CI-green.

> ## ⟶ NEXT SESSION — READ THIS FIRST: opencode pitch is technically ready; what's left is human/recording
>
> The active frontier is the **opencode skills-pointer pitch** (`docs/pitch-opencode.md`, gitignored —
> do NOT commit it). Mission (MISSION.md): get opencode to add a "recommended for security-conscious
> users" pointer to **openagent-skills** (the skill scanner + CDR). This session de-risked everything
> technical; what remains is human/recording work only.
>
> ### ✅ Done this session (2026-06-08) — the pitch's technical blockers
> - **opencode scouted.** They ALREADY ship runtime isolation + proxy-side credential injection
>   (Docker's `sbx run opencode` agent sandbox) + a capability permission system (`ctx.ask()`,
>   doom-loop detection). So the whole-perimeter / "containerization layer" pitch is a NON-STARTER
>   (we'd be displacing Docker's official sandbox). The unmet gap is **skill-content vetting before
>   load** — and opencode HAS skills (`skills/` dir, `SKILL.md`, the `opencode-agent-skills` plugin).
>   That gap is the entire wedge. Pitch is scoped to it.
> - **Citations verified** (safe to quote): Koi/Yomtov 341/2,857 = 11.9% (koi.ai, Hacker News, SC
>   Media); Snyk 3,984 skills 13.4% critical + 36% prompt-injection (snyk.io ToxicSkills); 42,447-skill
>   study 26.1% ≥1 vuln (arXiv 2602.06547).
> - **opencode-skills compatibility PROVEN** (task #36) — the "works with their CLI" proof:
>   pulled real opencode skills (`open-hax/opencode-skills`, Anthropic Agent-Skills format, NO
>   `clawdbot` metadata) → both scan **Clean**; a ClawHavoc-style malicious opencode skill (prompt
>   injection in `SKILL.md` + bundled `setup.sh` w/ cred-exfil + AMOS `curl|sh`) → **BLOCKED (1 crit +
>   3 high)**; and the full **CDR 8-stage round-trip** rebuilt a real opencode skill clean-room +
>   post-verified Clean (via `qwen2.5-coder:1.5b`). HONEST caveat recorded: the 1.5b reconstruction
>   introduced minor semantic drift (invented a `stop-editing` command) — fidelity cost of the
>   fail-closed rebuild; a 3b/7b model reduces it at a memory cost. Forge scanner CLI:
>   `bash workloads/skills/tools/skill-scan.sh <skill-dir>`; CDR `tools/skill-cdr.sh <SKILL.md>`.
> - **OpenSSF passing badge** (#12755) live on README — third-party credibility signal.
> - **Demo gifs DONE:** "malicious skill caught" gif (`docs/assets/demo-skill-caught.gif`, real scan of
>   a malicious opencode `SKILL.md`, `b3e6f68`) + wizard/tour re-recorded vs v0.6 (`236100c`, via
>   `scripts/demo-gif.sh`). All embedded in README/spotlight/pitch.
> - **Recipient researched** (saved in the gitignored pitch notes): canonical repo `anomalyco/opencode`
>   (171k★; `sst/opencode` redirects there; `opencode-ai/opencode` is ARCHIVED). First-touch **Adam
>   (@adamdotdev / `adamdotdevin`)**, decision-maker **Dax Raad (@thdxr)**. Channel = a HUMAN one (X DM /
>   email), NOT the security path — opencode's `SECURITY.md` auto-bans AI-generated security reports, so
>   the pitch must open "this is a recommendation, not a security report" and read unmistakably human.
> - **Skill scanner self-audited (honesty pass) + made leaner** (`026422c`, `5619c09`): a workflow
>   audit found real overclaims; fixed them honestly (the opencode audience reads code, and their culture
>   punishes AI-slop overclaiming). (a) **Pinned the CDR model to 1.5b** — killed a `cdr-intent.sh`
>   footgun that defaulted to 7b/4.7GB when `cdr.conf` wasn't sourced. (b) **BYO-model**: both model
>   scripts (`cdr-intent.sh`, `create-draft.sh`) now speak Ollama-native AND OpenAI-compatible
>   (`CDR_API_FORMAT` in `cdr.conf`) — a user can reuse a model they already run; **no mandatory heavy
>   download**. Validated both protocols live (rebuild + create produce Clean SKILL.md). (c) **Honest
>   docs**: fixed ADR-0003's false "deterministic per input" claim; "five INDEPENDENT defences" → honest
>   layered framing (3 distinct mechanisms; stages 1/2/5 share the pattern set); stated CDR cost plainly
>   (scan-only = offline/on-demand/~0 RAM); made "any LLM backend" true+precise. Scanner self-test 10/10
>   (patterns untouched). **The pitch draft now reads honest-and-precise, which is STRONGER for opencode.**
> - **CDR pipeline hardened** (`fae7f3a`→`7de296c`→`1cf8e7e`): (a) tried a 3b CDR default for fidelity,
>   but a live A/B showed 3b FAILS post-verify lint 2/2 where 1.5b passes — **REVERTED**, kept 1.5b
>   (ADR-0015's 1.5b-parser/3b-judge split was right). (b) Fixed the real defect: stage-7 post-verify
>   (lint/scan/verify) was TERMINAL; now it runs INSIDE the retry-repair loop, so a marginal-but-clean
>   reconstruction self-heals instead of false-quarantining (retires much of the ZONE-4a class).
>   Security preserved (malice stripped at the stage-3 prefilter; scan/verify still gate delivery;
>   confirmed a malicious skill is still rejected). 3b now passes. (c) Added **deterministic, model-free
>   regression tests** (`cdr-pipeline.test.sh` 11/11) via an env-gated `CDR_INTENT_STUB` test seam.
>
> ### ⟶ Remaining before send — just the human send + one optional credibility check
> - 🟢 **All pre-send prep is DONE** (citations, badge, scouting, compatibility proof, gifs, recipient,
>   honest+lean materials). **The only step left is a human: final read-through of `docs/pitch-opencode.md`
>   + send to Adam** (X DM / email; lead "not a security report").
> - 🟡 Karen v0.6 first-run E2E — a general credibility check (the "never dead-ends" floor), NOT a pitch
>   blocker; needs `xdotool`/`wmctrl`/`imagemagick` prereqs (state.json `karen-e2e-v06`).
> - The full pre-send checklist + scouting + recipient notes live at the bottom of `docs/pitch-opencode.md`.
>
> ### Memory optimization — COMPLETE (Phase 0–3), one operator verify pending
> All four phases shipped: Phase 0 (measurement harness), Phase 1 (on-demand shields, resting 5→3),
> Phase 2 (`4ced564` — agent image **754→590 MB** via a safe `*.d.ts`/`*.map`/`*.flow` + `@types`
> node_modules strip; validated by a LIVE BOT SMOKE — the pruned agent returned a real LLM reply
> "PONG"; LESSON: OpenClaw treats `*.ts` extensions AND `*.md` workspace templates as RUNTIME assets,
> both caught the hard way; see `workloads/agent/docs/specs/2026-06-06-image-conservative-prune.md`),
> Phase 3 (`54596f0`·`db95371`·`fc35a52`·`dcb28c3`·`0708471`·`0d5aef8` — idle auto-pause + Telegram
> peek waker, default ON; ADR-0018). **One thing pending (task #35): operator live-verify Phase 3 on a
> machine with RAM headroom** (idle → Dormant + RAM≈0 → message resumes exactly once + cold-start) —
> this 7.2 GB box swap-storms the perimeter.
>
> ### Working constraint (unchanged): the 7.2 GB box can't compile Rust — verify via CI round-trips
> push, then `gh run watch <CI-run-id> --exit-status` on the `Rust (check + test)` job (~5 min; a push
> triggers several workflows — pick `workflowName == CI`, not Scorecard/CodeQL). Parse-check cheaply
> first with `rustfmt --edition 2021 --check <file>`. Frontend gates (eslint `--max-warnings 0`,
> `tsc --noEmit`) CAN run locally. NOTE: the box CAN run a single `podman build` + a 2-container bot
> smoke when Brave/Slack are closed (~3 GB free) — that's how Phase 2 was validated; the FULL
> 5-container perimeter still swap-storms.

> ## ⟶ 2026-06-08 — CDR robustness: post-verify moved into the retry-repair loop (`7de296c`)
>
> Fixed a real structural defect (the reconstructor↔lint coupling). The CDR retry loop covered stages
> 4–6 only; **stage 7 (post-verify: lint/scan/verify) was TERMINAL** — a clean reconstruction that
> marginally failed (e.g. a `TODO` token tripping lint — deterministically confirmed the ONLY lint-FAIL
> path for reconstructed output) was quarantined with no repair attempt. That's why 3b failed 2/2 where
> 1.5b passed. Fix: lint→scan→verify now run INSIDE the loop; a failure becomes a repair hint + retry,
> quarantine only after the budget. **Security preserved** (verified): malice is stripped at stage 3
> prefilter before the loop, and scan+verify still gate delivery, so nothing can be "retried into
> passing" — a malicious skill is still REJECTED at prefilter. Validated: 1.5b PASS (regression), **3b
> now PASS** (was 0/2), self-test 10/10, cdr-pipeline.test.sh 9/9. This also retires much of the
> ZONE-4a false-quarantine class. Spec: `workloads/skills/docs/specs/2026-06-08-cdr-postverify-in-retry-loop.md`.
> **Regression tests added** (`1cf8e7e`): `cdr-pipeline.test.sh` now 11/11 with two model-free tests —
> retry-then-recover, and persistent-failure→quarantine — via a minimal env-gated `CDR_INTENT_STUB`
> seam in `cdr-intent.sh` (test-only, never set in the container, doesn't bypass scan/verify).
>
> ## ⟶ 2026-06-08 — skill scanner: honest self-audit → leaner (BYO-model) + corrected docs
>
> Prompted by "is our scanner truly as novel/effective as I think, and how heavy is the parser model?"
> Ran a 4-agent adversarial workflow audit, then acted on it. Commits `026422c` + `5619c09`.
>
> - **Honest verdict (carry forward):** the scanner is a competent **offline regex blocklist** (87
>   patterns, 16 injection) — real and deterministic, like `npm audit` for skills. CDR (quarantine →
>   LLM intent-extract → rebuild) is a genuine property (original never delivered) and **first-to-apply
>   CDR to skills**, but NOT conceptually novel (email CDR ~2010). "Five INDEPENDENT defences" was an
>   overclaim (stages 1/2/5 share the pattern set → ~3 distinct mechanisms). It does NOT catch
>   polymorphic/text-natural injection or trivial obfuscation (admitted in threat-model).
> - **Leanness (the key answer):** the **scanner needs NO model** (pure offline grep) and `vault-skills`
>   is **on-demand** → scan-only = ~0 resting RAM, no download. Only the **opt-in CDR rebuild** needs an
>   LLM. Parser default is `qwen2.5-coder:1.5b` (~1 GB) — fixed a footgun where `cdr-intent.sh` silently
>   defaulted to 7b (4.7 GB).
> - **BYO-model shipped:** both `cdr-intent.sh` and `create-draft.sh` now speak Ollama-native AND
>   OpenAI-compatible (`CDR_API_FORMAT`/`CDR_ENDPOINT`/`CDR_API_KEY` in `config/cdr.conf`). A user points
>   CDR/creation at a model they ALREADY run (agent model, LM Studio, vLLM, managed API, remote Ollama) —
>   no forced download. Validated both protocols live (against Ollama's own `/v1/chat/completions`).
> - **Docs corrected** (ADR-0003 determinism; "five independent"→layered; CDR cost stated; "any LLM
>   backend" now true+precise) across README, `docs/skills-spotlight.md`, `workloads/skills/...`, and the
>   pitch. Spec: `workloads/skills/docs/specs/2026-06-08-cdr-byo-model-backend.md`. Scanner untouched
>   (self-test 10/10). **3b CDR default tested + REVERTED** (`5855684`→`48f1d7b`): a live A/B on a real
>   opencode skill showed `qwen2.5-coder:3b` FAILS CDR post-verify lint 2/2 (its rebuilds break the
>   template/lint constraints) where `1.5b` passes — bigger ≠ more faithful for this reconstructor. Kept
>   1.5b (also the lean choice). **Remaining follow-up:** quantify the CDR false-positive rate.
>
> ## ⟶ 2026-06-08 — opencode pitch readiness (compatibility proven) + memory Phase 2 shipped
>
> **Goal:** de-risk the opencode skills-pointer pitch enough to send. Outcome: all *technical*
> blockers cleared; only human/recording items remain (see the NEXT SESSION block above for the
> full breakdown).
>
> - **Scouted opencode.** It already has runtime isolation + proxy-side credential injection
>   (Docker `sbx run opencode`) and a capability permission system. → the whole-perimeter pitch is
>   a non-starter; the wedge is **skill-content vetting before load**, which they lack. opencode HAS
>   skills (`SKILL.md`), so openagent-skills applies directly. Pitch (`docs/pitch-opencode.md`,
>   gitignored) reframed around this.
> - **Verified all 3 supply-chain citations** (11.9% Koi/Yomtov, 13.4% Snyk, 26.1% arXiv 2602.06547).
> - **Proved openagent-skills works on REAL opencode skills** (task #36): clean skills scan Clean; a
>   malicious opencode-format skill is BLOCKED (cred-exfil + AMOS C2 + prompt injection, across both
>   `SKILL.md` and a bundled script); full **CDR 8-stage round-trip** rebuilds a real opencode skill
>   clean-room + post-verifies Clean (qwen2.5-coder:1.5b; minor semantic-drift caveat recorded).
> - **Memory Phase 2 shipped** (`4ced564`): vault-agent image **754→590 MB** via a safe node_modules
>   strip (`*.d.ts`/`*.map`/`*.flow` + `@types`; NO `*.ts`/`*.md`/package removed). Validated by a
>   live bot smoke (pruned agent replied "PONG"). Two file types are RUNTIME assets for OpenClaw and
>   must stay: `*.ts` (extensions incl. telegram) and `*.md` (workspace templates like AGENTS.md) —
>   both caught the hard way (the `.md` one only by the live smoke). This box CAN do a single
>   `podman build` + 2-container bot smoke with Brave/Slack closed (~3 GB free); the full 5-container
>   perimeter still swap-storms.
>
> ## ⟶ 2026-06-06 — Memory optimization (run on small laptops): Phase 0+1+3 done, Phase 2 paused
>
> A live profiling attempt showed the 5-container perimeter takes the 7.2 GB dev box to
> ~142 MB free / 3.8 GB swap (trips the `CONSTITUTION.md` swap>500 MB guardrail). Plan
> (`~/.claude/plans/glimmering-meandering-babbage.md`, 4 phases) to cut the resting footprint.
>
> **Honest reframe:** the resident RAM is dominated by **vault-agent (~600 MB Node/OpenClaw) +
> vault-proxy (~150 MB mitmproxy)**; vault-skills/vault-social are idle `sleep infinity` bash
> (~5–20 MB each, **not** "1 GB"). So **idle auto-pause is the only big RAM lever**; on-demand
> shields are hygiene; image-slim is disk not RAM; **measure first**.
>
> | Phase | Status |
> |------|--------|
> | **0** measurement harness | ✅ `d858827` — `make profile-memory` (per-container RSS + host RAM/swap + image sizes) |
> | **1** on-demand skills/social | ✅ `3ba9c4e`, **CI-green** — `on_demand` flag + `boot_services()`; up()/shell_up() skip; bootstrap shell_services fix; execute.rs start-if-needed + 300 s keep-warm; orchestrator-check §30 (114/0). Resting perimeter **5→3 containers**. |
> | **2** agent image prune | ⛔ PAUSED — needs an image rebuild + `verify.sh`; the box can't build; agent image is security-critical (validate-before-commit). |
> | **3** idle auto-pause + waker | ✅ CODE-COMPLETE + CI-green, **default ON** (all via CI round-trips; box can't compile locally). A `54596f0` (idle signal + dormant markers); B `db95371` (`AssistantStatus::Dormant` + tray); C `fc35a52` (watchdog idle hook); ADR `dcb28c3` (ADR-0018 + T6 row); D `0708471` (`idle.rs` peek waker — no `offset` ever + `stop_waker` cancel-before-resume + dormant-cleared-on-launch + unit tests); E `0d5aef8` (gate → `idleAutoPause`/`idleTimeoutMinutes` settings, `closeToTray` wired via `on_window_event`, Dormant hero + Home tile + Preferences toggle). **Remaining: a one-off operator live-verify on a machine with RAM headroom** (idle → Dormant + RAM≈0 → message resumes exactly once + cold-start) — this box swap-storms the perimeter. |
>
> **Update:** Phase 3 was completed via CI round-trips (A–E above; idle auto-pause default ON). Only
> **Phase 2 stays paused** — it needs a real `vault-agent` image rebuild + `verify.sh` this 7.2 GB box
> can't run (swap-storms; `earlyoom` armed) and which is security-critical (validate-before-commit).
> **Resume Phase 2 on a machine with RAM headroom.** Phase 1 follow-ups (in its commit):
> component-workflow on-demand auto-start; real in-container `podman exec` execution (framing B —
> today commands run host-side, so on-demand mainly readies the dev/compose path).

> ## ⟶ 2026-06-05 — OpenSSF Best Practices PASSING badge earned
>
> The project earned the **OpenSSF Best Practices passing badge** (bestpractices.dev
> **project #12755**). The live badge is on the README badge row (`e016839`), links to
> the project page, and reports `passing`.
>
> | What | Detail |
> |------|--------|
> | Badge | OpenSSF Best Practices **Passing** (#12755) — was *Lobster-TrApp* / 18% pre-rebrand |
> | Answer catalog | `docs/openssf-badge-answers.md` — all 67 criteria + metadata, each verified against the repo, plain text, honest (63 Met / 4 N/A) |
>
> ### Load-bearing findings (carry forward)
> - **Edit the entry, never re-apply.** The badge predated the rebrand (filed as
>   *Lobster-TrApp* at lobster-trapp.com). A name/domain change edits #12755 in place;
>   the **repository-URL field is what Scorecard's CII-Best-Practices keys on**. Re-applying
>   would orphan progress.
> - **Verification caught real drift** (now fixed): GitHub Discussions is OFF (so
>   `discussion` is met via the issue tracker, not Discussions); CodeQL was NOT "zero" —
>   the open code-scanning alerts were OpenSSF Scorecard posture checks surfaced as SARIF,
>   not code vulns. Both corrected in `docs/openssf-best-practices-application.md`.
> - **5 code-scanning alerts dismissed** (with explicit user authorization): #72/#73 CodeQL
>   unused-variable false positives (the var is used in an inline `{e}` format string), and
>   #77/#78/#79 devcontainer dependency-pin advisories (dev tooling). The 4 Scorecard-posture
>   alerts remain open by design; #42 (CII-Best-Practices) clears once Scorecard re-runs.
> - **Honesty stance for re-attestation:** do NOT press the form's "no cryptography" button.
>   The software verifies update + image signatures and uses TLS, so crypto criteria are
>   answered individually (Met via libraries; N/A only for pfs / password-storage / random).
>   The answer catalog has zero em-dashes and is a 1:1 mirror of the questionnaire for next time.
>
> ### Also this session
> - **Landing page deployed** to `opentrapp.com`: the stale four-container copy was replaced
>   with the committed five-container copy (committed long ago at `ab2ffb5`, never deployed).
>   Verified live: HTTP 200, "five-container" ×5, "four-container" ×0.
> - **Dependabot:** `tar` 0.4.45 → 0.4.46 (GHSA-3pv8-6f4r-ffg2); CI green; alerts #14/#15 closed (`1079fc3`).
> - **Trackers reconciled** to v0.6.0 reality: `state.json` (lt-sec-001 / lt-brand-001 →
>   completed, DNS-rebinding residual → resolved, Karen E2E rescoped to v0.6) and the gitignored
>   `AGENT-TODO.md` (ZONE 2/4/5/6a/8 marked shipped; ZONE 1/3 still open).
> - **Zone 6b dogfood reply-misattribution fixed** (`2ed32e8`): late or continuation bubbles bled
>   into the next scenario and were recorded against the wrong prompt. Added `BotClient.reset_chat()`
>   (drains in-flight bot messages until the chat is quiet; sends nothing, so no send-budget cost),
>   called from `_attach_files`, plus a `serial_attachments` marker on A1/A5/B4. Verified statically
>   (all files compile; `pytest --collect-only -m serial_attachments` → exactly a1/a5/b4, strict
>   markers pass). One **live Telegram run** still needed to confirm the runtime drain (operator;
>   the dogfood suite is cost-bearing and not in CI).
>
> ### Follow-up
> - **Automatic:** Scorecard `CII-Best-Practices` flips **0 → 5** on the next nightly run.
> - **Operator queue:** SignPath resubmission (now unblocked), demo gifs vs the v0.6 build, and
>   one live Telegram dogfood run to verify the Zone 6b fix above.

> ## ⟶ 2026-06-02 (RELEASED) — READ THIS FIRST: v0.6.0 is published
>
> v0.6.0 is **live**: pushed, tagged, CI-built (4 platforms + SBOMs + cosign +
> SLSA provenance), and **published** (auto-updater will prompt v0.5.0 users).
> All four completion items (B/A/C/D) plus the release bump landed sequentially
> (parallelism was dropped — the 7.2 GB box swap-storms with concurrent agents + Ollama).
>
> | Item | Commit | What |
> |------|--------|------|
> | **B** Sentinel staging | `cbd2b9f` | `sentinel/` as a verified `:ro` bundle resource (host bridge + shields); README Ollama note |
> | **A** Allowlist approval | `665da53` | off-allowlist blocks → explained one-tap human decision; only-human-loosens (ADR-0016); `EgressApprovalsCard` |
> | **C** Live atproto adapter | `96d99a4` | first live network adapter (Bluesky public AppView); un-park social (ADR-0017); validated live |
> | **D1** Judge 2nd-opinion | `8450257` | rung-2 judge on the skills auto-allow — tighten-only (VERIFIED→QUARANTINED), opt-in `--judge` |
> | release | `e624c2c` / `7ff6cae` | version bump + notes; **fix(ci): green the gate** — see the load-bearing finding below |
>
> ### ⚠ Load-bearing finding — the local gate omitted two CI jobs
> CI's `CI` workflow had been **red on `main` since before v0.6** because
> `npm run lint` (eslint `--max-warnings 0`) and `tests/integration-test.sh` were
> never in our local gate (we ran cargo/tsc/vitest/playwright/orchestrator-check only).
> The first `v0.6.0` tag built on a red commit and produced no release. `7ff6cae`
> fixed both (stale pre-ADR-0013 paths in the integration test; 36 accumulated lint
> problems) and **added both jobs to the documented gate in `CLAUDE.md` §7**. Always
> run `npm run lint` + `integration-test.sh` — a local green without them ≠ CI green.
>
> ### Gate (full, CI-equivalent, green at the released commit)
> **lint 0/0**, cargo `109/0`, orchestrator-check **108/0/0** (§21–§29), tsc clean,
> vitest `87/87`, playwright `25/25`, **integration-test 0 failures**; bash suites:
> atproto 7/7, skill-verify-judge 4/4, adapter 16/16, firewall 2/2, persona-guard 4/4,
> disarm-report 4/4, cdr-pipeline 9/9, embed 6/6, judge 3/3.
> Requires Ollama with `qwen2.5-coder:1.5b` + `:3b` + `all-minilm`.
>
> ### Remaining = operator queue (NOT code; do not re-implement)
> - **D2** pre-release: re-record demo gifs against the v0.6 build; OpenSSF badge
>   resubmission; sweep `forge→skills` in the **gitignored** `docs/pitch-opencode.md`
>   (on-disk only — never committed).
> - **D3 / Zone 6b** dogfood-harness reply misattribution (`tests/dogfood/test_full_arc.py`):
>   add a `reset_chat()` helper + a `serial_attachments` marker. Pre-existing test-infra
>   bug, deferred from v0.6.
> - ~~Push + cut v0.6.0~~ **DONE** — published 2026-06-02 (`/releases/latest` → v0.6.0).
>
> ### The load-bearing findings this session (carry forward)
> 1. **Verified-resource staging beats image-copy** for shared libs — consistent
>    with how the whole perimeter stages policy files (refined SD-B1).
> 2. **Allowlist persistence:** seed is re-staged + overwritten each launch, and the
>    proxy bind-mount is a single file — so additions persist OUTSIDE the staged path
>    and append IN-PLACE (never temp+rename, which swaps the inode), then SIGHUP.
> 3. **Whole-skill judging dilutes** a buried instruction (3b reads it as "documentation")
>    — judge per-paragraph instead (`skill-chunks.py`). The malicious chunk in isolation
>    blocks deterministically.

> ## ⟶ 2026-06-01 (continuation) — superseded by the completion entry above
>
> **The next session is implementation, against a harmonised plan:**
> **[`docs/specs/v0.6/08-completion-plan.md`](specs/v0.6/08-completion-plan.md)** — read it first.
>
> ### What landed this continuation (on `main`, gated green, pushed)
> - **Rung-1 embeddings** (`ee5e775`) — D2 resolved → `all-minilm`; `sentinel/embed.sh`
>   (`vector`/`score`/`drift`) + `corpus/`. Banked finding: `drift` (vs the agent's
>   own voice) is the reliable gating signal; `score` (corpus similarity) is a
>   **recall-safe booster, never a gate** (misses novel paraphrases → must not
>   suppress rung 2).
> - **Per-profile image bundling** (`1b84c5e`), **M4 adapter abstraction** (`dc5fb76`),
>   **ADR-0015** (`d024c89`) — the three parallel Sonnet streams.
> - **GUI Sentinel bridge + activity indicator** (`4dffcfb`) — `commands/sentinel.rs`
>   (`sentinel_judge`, malformed→escalate-never-allow) + the watching/thinking
>   badge on the Security page.
> - **Persona-drift outgoing guard** (`eabbb36`) — `persona-guard.sh`; hijacked
>   outgoing posts HELD; fail-safe never-auto-send.
> - **Disarm-diff display** (`9920c51`) — read-only trust artifact via the
>   **manifest channel** (`cleaned-skills` cmd in-container → `CleanedSkillsCard`).
>
> ### The two load-bearing principles this session established (carry forward)
> 1. **Security-first ordering:** read-only transparency before any write/loosening
>    surface. (Why the allowlist is deferred to its own threat-modeled slice.)
> 2. **Right channel for the component type:** workloads → manifest command;
>    infra (proxy/egress, no manifest) → the orchestrator's container-management
>    layer. (`08` §3.)
>
> ### What remains (all in `08-completion-plan.md`, sequenced + harmonised)
> - **A** Allowlist approval (threat-modeled write surface) · **B** production
>   Sentinel staging (host + container) · **C** M4 live network adapter · **D**
>   closeout (judge-as-2nd-opinion, pre-release, Zone 6b, ADR-0016).
> - **Sequencing:** Opus does **B → A** sequential (shared runtime+GUI surfaces);
>   Sonnet runs **C / D** in parallel (disjoint files; must avoid the collision
>   set: `build.rs`, `bootstrap/mod.rs`, `podman.rs`, `compose.yml`, `lib.rs`,
>   `App.tsx`, `SecurityMonitor.tsx`).
> - **Decisions RESOLVED (2026-06-01):** SD-A1 Always+Deny (defer allow-once),
>   SD-A2 remember-deny, SD-B1 bind-mount dev / image-copy release, SD-B2
>   no-bundle-Ollama, **SD-C1 scout AT Protocol first**. (`08` §9.) No open
>   blockers — the next session implements directly.
>
> ### Verify (current gate at `9920c51`)
> orchestrator-check **89/0**, cargo **96/0**, tsc clean, vitest **82/82**,
> playwright **25/25**; bash suites: judge 3/3, egress 5/5, embed 6/6, firewall
> 2/2, adapter 16/16, persona-guard 4/4, disarm-report 4/4. Requires Ollama with
> `qwen2.5-coder:1.5b` + `:3b` + `all-minilm` pulled.

> ## ⟶ 2026-06-01 — v0.6 implementation handoff (M0–M4 — history)
>
> **What v0.6 is:** the "uses AI to make AI safe" reassessment. A tiny local AI
> (**Sentinel**, `sentinel/`) judges the gray zone the static defences miss.
> Full spec: **`docs/specs/v0.6/`** (00-index → 07-roadmap). Concept locked,
> milestones M0–M4 implemented + verified against a live local model.
>
> ### What landed this session (all on `main`, gated green)
> - **M0** (`b854dcc`) — renamed `forge → skills` everywhere (`workloads/skills`,
>   `vault-skills`, `openagent-skills`). Historical ADRs/archive untouched.
> - **M1** (`12f7e2a` + `f9f564c`) — the Sentinel judge lib (`sentinel/judge.sh`,
>   injection-hardened, lib-first) + the **ZONE-4a fix** (CDR was ~50% flaky on
>   clean skills → retry-with-repair makes it reliable, quarantine-never-silent)
>   + the **disarm diff** (plain-language "what was removed", saved as
>   `DISARM-DIFF.txt`).
> - **M2** (`15c4362`) — modular distribution: `distribution.yml` (single
>   source), profile-driven `build.rs` + bootstrap, `scripts/install-shield.sh`
>   (install one shield standalone, no GUI).
> - **M3** (`f0b1c63`) — adaptive containment: `sentinel/egress-advisor.sh`
>   proposes least-privilege from the egress log; **never-auto-loosen invariant**
>   (ADR-0002) structurally enforced + tested.
> - **M4** (`d78a77e`) — semantic firewall: `workloads/social/tools/semantic-firewall.sh`
>   catches **paraphrased injections the 25 regexes miss** (rung-0 → rung-2).
> - **D3 fix** (`04e4dde`) — the one quality ceiling. See tiering finding below.
>
> ### The load-bearing finding — tiered models
> **Give the bigger model only to the role whose mistakes you can't otherwise
> catch.** The tiny model is the **parser** (CDR describe: skill → intent JSON);
> its failures are schema-detectable + retry-recoverable → stays on the leaner
> **`qwen2.5-coder:1.5b`** (6/6 once the prompt is explicit — reliability came
> from the *prompt*, not size). The judge's failures are *not* self-checking →
> it gets **`qwen2.5-coder:3b`** (allows benign docs-example 5/5, blocks exfil,
> resists judge-injection; the 1.5b over-blocked). Banked in
> `docs/specs/v0.6/01-sentinel-spine.md §4` + `sentinel/README.md`.
> Both local, no API key. Env-overridable (`SENTINEL_MODEL`/`CDR_MODEL`).
>
> ### How to verify (one-liners)
> - `bash tests/orchestrator-check.sh` → **72/0** (re-verifies §10–§20: perimeter,
>   bot vocab, proxy-log, rename-complete, Sentinel lib, distribution, advisor,
>   semantic firewall).
> - USP live: `bash workloads/social/tools/semantic-firewall.sh --file workloads/social/tests/fixtures/paraphrased-injection-posts.json`
>   (judge catches what regex can't) · `cd workloads/skills && bash tools/skill-cdr.sh tests/cdr-fixtures/clean-skill.md` (reliably delivers + disarm diff).
> - Standalone install: `bash scripts/install-shield.sh openagent-skills` → a `skills` CLI, no GUI.
> - Full gate: cargo `91/0`, tsc clean, vitest `74/74`, playwright `--project=default` `25/25`.
> - **Requires Ollama** running with `qwen2.5-coder:1.5b` + `:3b` pulled (parser/judge).
>
> ### What's deferred (flagged in commits + specs — NOT faked)
> - **Rung-1 embeddings** (D2) — not built; no embedding model pulled; rung 0→2
>   works without it. Persona-drift on *outgoing* posts needs this.
> - **GUI pieces** — the Sentinel activity indicator, the one-tap allowlist
>   approval UX, the install-profile picker. Backends exist; the React/Tauri
>   surfaces don't. These presuppose the GUI invoking Sentinel (currently a
>   bash lib the CLIs call).
> - **M4 live adapter** — `semantic-firewall.sh --adapter file` works; a live
>   agent-social-network adapter (Mastodon/AT-proto/Nostr) + its validation is
>   the remaining step. The adapter seam is in place.
> - **Per-profile image bundling** (smaller AppImage) — release/packaging.
> - **Wiring the judge as an auto-allow scanner second-opinion** — now viable
>   with the 3b's precision (was blocked by 1.5b over-blocking); not yet wired.
>
> ### Suggested next-session order
> 1. The GUI Sentinel surfaces (activity indicator + disarm-diff display +
>    one-tap allowlist) — the biggest user-visible gap; reuse the
>    `useBootstrapProgress` event pattern.
> 2. ADR-0015 recording the Sentinel decision (the spec suggests it).
> 3. Rung-1 embeddings (pull a small embed model; wire similarity/drift).
> 4. M4 live adapter scouting (MISSION.md Thread C step 1).
> 5. Pre-release: re-record demo gifs against a v0.6 build; update the gitignored
>    `docs/pitch-opencode.md` to the new `skills` naming; OpenSSF badge.
>
> **Gitignored working docs (on the maintainer's machine, not in the repo):**
> `MISSION.md` (multi-session north star), `AGENT-TODO.md` (zones — 1, 2, 3, 5,
> 6a done; 4a done via M1; 4b done; 6b open), `docs/pitch-opencode.md` (opencode
> outreach draft, awaits the right human + the skills rename).

> ## ⟶ 2026-05-21 — E2E run + rescope (read this first)
>
> A full Karen E2E ran against the **cosign-verified v0.5.0 AppImage on a true clean box**.
> **Verdict: SHIP-WITH-CAVEATS — the security thesis HOLDS; first-run/recovery UX is the gap.**
> - Tier B 7/7 substantive PASS (credential exfil, workspace, exec, **indirect injection**,
>   malicious skill, pairing, self-promote all refused). Forge scanner self-test 10/10 direct.
>   The only Tier-B fail is a banned word ("sandboxed"), not a breach.
> - Full scored record: **`docs/specs/2026-05-20-dogfood-full-arc-findings.md`**.
> - **All next work is rescoped into construction zones in the (gitignored) `AGENT-TODO.md`** —
>   one focused mission per agent. ZONE 1 (first-run/recovery UX) is the top priority.
> - **The retry-idempotency P0 is fixed + committed** (`e52541f`, local). New bugs to file:
>   proxy-log can't persist (ZONE 3), forge CDR-on-clean fails + unreachable via chat (ZONE 4),
>   bot vocabulary (ZONE 5), autostart pins binary path (confirmed live), stale verify.sh.
> - **Impact on the SignPath/OpenSSF mission below:** the E2E *confirms the security posture* the
>   resubmission needs — that axis is now evidenced. The A1–A4 security tasks below remain the
>   gating checklist; the new UX zones are additive, not blockers for SignPath.
**Latest release:** **`v0.5.0`** — published, `latest`, all platforms, cosign-signed. Five-container perimeter (ADR-0009/0010) + self-sufficient bootstrap (ADR-0011): no on-host build, native podman orchestrator (no compose), pre-built cosign-signed images delivered as release assets and digest-verified at first launch. ~90 MB AppImage.

> **v0.5.0 fully validated (2026-05-20):** clean-box E2E from a downloaded AppImage with no source clone — `fetch_perimeter_images` pulled the signed tarballs from the **published** release, digest-verified each, loaded them, brought up all five containers (vault-egress healthy under rootless podman), agent activated, hero "running safely". Tamper test refused a swapped image. See [ADR-0011](adr/0011-zero-trust-self-sufficient-bootstrap.md).
>
> **Known issues / v0.5.1 candidates:**
> 1. **Autostart pins the binary path (P1).** Autostart defaults *on* (`app/src/App.tsx:39-66` reconcile + the persisted preference) and registers the *current* binary path. For an AppImage (no stable path) the entry goes stale when the AppImage moves/updates → a failed launch on next login. Fix options: default autostart *off*; or, for AppImage, install to a stable location / repair-or-skip a stale entry on launch. This was the root cause of the "Sandbox setup failed" card seen when an old/ephemeral AppImage autostarted.
> 2. **macOS/Windows runtime install** still deferred — `podman` absent by default (Linux/AppImage only so far).
> 3. GHCR `vault-*` packages are private — fine for runtime (images come from release assets), but make them public for the cosign/transparency audit axis.

---

## RUN THIS NEXT — close the security gap, then resubmit SignPath

The maintainer applied to **SignPath Foundation** for free Windows code-signing under the old **Lobster-TrApp** branding + the old website. SignPath is on hold. The maintainer wants to **resubmit fresh** under the **OpenTrApp** brand + `opentrapp.com` — **after** the open security issues are documented and the regressions are tested. Order matters: a clean security posture is what makes the resubmission credible.

> ### ⟶ Signing decision (2026-06-12) — scaffold both Windows + macOS now
>
> **Decision:** rather than wait on the SignPath resubmission, **pre-build the CI integration for both
> platforms** (commit `66750fc`), so the moment certs/approval land, activation is a few-line change — not
> new engineering. This de-risks the resubmission and removes signing from the critical path.
> - **macOS — ready-to-activate template** (commented `APPLE_*` env in `ci.yml`). It was briefly wired live
>   (`66750fc`) but that BROKE the macOS build: `tauri` treats a present-but-empty `APPLE_CERTIFICATE` as
>   "sign now" and fails on the blank cert — so empty secrets are not inert. Reverted to a commented template
>   in `719cc19` (CI green). *Activate by:* enrolling in the Apple Developer Program, adding the six `APPLE_*`
>   secrets, then uncommenting the env lines (present==real, no longer empty).
> - **Windows — ready-to-activate SignPath template** (commented in `ci.yml`, inline checklist). Deliberately
>   NOT live: the org/project/policy slugs come from the (fresh, pending) SignPath OSS account, and every
>   `uses:` must be SHA-pinned (OpenSSF Scorecard). *Activate by:* SHA-pinning the SignPath action, filling
>   the slugs, adding `SIGNPATH_*` secrets, uncommenting. This supersedes the CI-integration steps in the
>   old plan `~/.claude/plans/ethereal-wiggling-rocket.md` — they are now pre-written in the workflow.
> - **Order still holds:** the security work (A1–A4 below) → green gates → resubmit SignPath under OpenTrApp
>   + rerun OpenSSF badge. The scaffold doesn't change that order; it just means the *CI half is already done*.
> - Full required-secrets tables: [`docs/code-signing-policy.md`](code-signing-policy.md).
>
> #### ⟶ SignPath application SUBMITTED 2026-06-13 — pending review
> The fresh SignPath Foundation application (OpenTrApp brand) was **submitted by the maintainer on
> 2026-06-13 and is now awaiting SignPath's review.** Next action is SignPath's — watch for their email to
> `albertkdobmeyer@gmail.com`. The submitted values are in [`signpath-application.md`](signpath-application.md).
> **On approval:** activate the Windows SignPath template in `ci.yml` (SHA-pin the action + fill
> org/project/policy slugs + add `SIGNPATH_*` secrets + uncomment) — the CI integration is already written.
> - **Site deployed + verified LIVE** before submitting (the Download/Privacy URLs only count once live).
>   `scp`'d `index.html` + `privacy.html` to the VPS; runbook §4 all-green (both SHA-synced, nginx active,
>   home + privacy HTTP 200) and independently re-confirmed over Cloudflare: new SignPath line present, old
>   false line gone, footer Privacy link present, `/privacy.html` serving the real page.
> - **Values submitted:** Project `opentrapp`; repo `github.com/albertdobmeyer/opentrapp`; homepage
>   `https://www.opentrapp.com`; **Download URL** `https://www.opentrapp.com/#download`; **Privacy URL**
>   `https://www.opentrapp.com/privacy.html`; Maintainer Type **Individual**; Build System **GitHub Actions**;
>   reputation led with security signals (OpenSSF Best Practices #12755, Scorecard, CodeQL, SBOM+cosign+SLSA,
>   public threat model/whitepaper) since the repo is young (1★). Full tagline/description/reputation text is
>   in the 2026-06-13 chat transcript.
> - **Honest-wording flag:** the download page says *"free Windows code signing provided by the SignPath
>   Foundation's open-source program — rollout in progress"* (`77d4da0`). It is NOT signed yet. **If the
>   reviewer asks for unconditional present-tense, drop "rollout in progress" once the first signed release
>   ships** — do not claim signed before it is.
> - **When approved:** activate the Windows SignPath template in `ci.yml` (SHA-pin the action + fill
>   org/project/policy slugs + add `SIGNPATH_*` secrets + uncomment) — the CI integration is already written.
> - The security follow-ups (A1–A4 below) remain open and may be read by the reviewer; they were NOT gating
>   the submission (maintainer chose to submit now with the CI scaffold + live site ready).
> - Artifacts: download-page note + `docs/privacy.html` (`77d4da0`); deploy runbook tracks `privacy.html`
>   (`a7d0f1b`); `docs/code-signing-policy.md` (macOS + Windows secrets tables).

## ⟶ OpenSSF Scorecard remediation (2026-06-13) — overall was 7.4

Triaged the Scorecard (<https://scorecard.dev/viewer/?uri=github.com/albertdobmeyer/opentrapp>);
the honest finding: most low checks are **structurally capped for a solo Tauri app**, and
"26 vulnerabilities" is mostly **unmaintained upstream Tauri GTK3 crates** (not exploitable, not
ours). Full reasoning + the accepted-advisory list + how to read the Scorecard is in
[`docs/known-advisories.md`](known-advisories.md). Landed on branch `scorecard-remediation` (PR):

- **Real vuln fixed:** `react-router-dom ^6.26.0 → ^6.30.4` (kills `GHSA-2j2x-hqr9-3h42`
  open-redirect); `npm audit --omit=dev` now **0**. tsc/lint/vitest(87) green.
- **`cargo deny` was RED on main** (Phase-B fallout): the version-less path dep
  `opentrapp-core` tripped `[bans] wildcards = "deny"`. Fixed with `allow-wildcard-paths = true`
  + `publish = false` on the three internal crates (they're never published). `cargo deny check`
  + `cargo audit` now exit 0. Also synced a **stale `Cargo.lock`** (was `opentrapp 0.7.1`, missing
  the two Phase-B crates) and dropped the resolved `RUSTSEC-2024-0429` ignore.
- **Docs:** `known-advisories.md` (new, linked from SECURITY + threat-model); CONTRIBUTING note
  that the maintainer routes own changes through PRs (recovers Scorecard *CI-Tests* from -1);
  CII Silver/Gold gap analysis in `openssf-best-practices-application.md`.

### ⚠️ MAINTAINER ACTION — branch protection (WS-B)
Apply the safe settings (NO required approvals — they'd block your own solo merges). Run:
```bash
gh api -X PUT repos/albertdobmeyer/opentrapp/branches/main/protection \
  -H "Accept: application/vnd.github+json" \
  -f 'required_status_checks[strict]=true' \
  -f 'required_status_checks[contexts][]=Rust (check + test)' \
  -f 'required_status_checks[contexts][]=Frontend (tsc + vitest)' \
  -f 'required_status_checks[contexts][]=Orchestration (42 checks)' \
  -f 'required_status_checks[contexts][]=Integration tests (cross-module contracts)' \
  -f 'required_status_checks[contexts][]=Playwright smoke tests' \
  -F 'enforce_admins=true' \
  -F 'required_pull_request_reviews[dismiss_stale_reviews]=true' \
  -F 'required_pull_request_reviews[require_code_owner_reviews]=true' \
  -F 'required_pull_request_reviews[required_approving_review_count]=0' \
  -F 'required_linear_history=true' \
  -F 'required_conversation_resolution=true' \
  -F 'restrictions=null' \
  -F 'allow_force_pushes=false' -F 'allow_deletions=false'
```
Set `required_approving_review_count=1` **only if you add a second maintainer** (which also lifts
the `Code-Review` + `Contributors` checks — the single highest-leverage real-world step).

### Honest residual caps (do NOT chase to 10/10)
- **Vulnerabilities:** Scorecard's external OSV scan won't read `deny.toml`; the GTK3 set keeps it
  low until Tauri drops GTK3. The one real vuln is fixed.
- **Code-Review / Contributors:** need a second human/org — solo cap.
- **Packaging (-1):** desktop app, no registry target — accepted.
- A literal 10/10 is neither achievable nor sensible for a solo Tauri project.

### The security work blocking SignPath

There is **one tracked task** in `~/.claude/state.json` (`lt-sec-001`) plus **one tracked known issue** (`lt-sec-001-residual`). The full plan is at `~/.claude/plans/soft-herding-whale.md` (Item A). The four sub-tasks:

- **A1.** Add a regression test that confirms direct IP-literal requests through `vault-proxy` return 403. The current behaviour was confirmed but is not pinned by a test.
- **A2.** Document the **DNS-rebinding residual risk** explicitly in `docs/threat-model.md` as a T-numbered residual risk, with the `block_private=false` trade-off rationale linked from there.
- **A3.** **Investigate whether `block_private=true` can be re-enabled.** It was disabled in `compose.yml` (the mitmproxy flags) for Telegram WebSocket compat. If the upstream Telegram proxy path no longer requires it, re-enabling closes the DNS-rebinding gap structurally. Root-cause context is in `components/opencli-container/docs/openclaw-internals.md`.
- **A4.** Add a "security claims surfaced by LLM tooling" template stanza to the dogfood-findings template at `tests/dogfood/findings-template.md` so the next dogfood pass triages inline AI-tool suggestions systematically.

When all four are done **and** the test gates are green, **then** rerun the OpenSSF Best Practices Badge form (pre-filled at `docs/openssf-best-practices-application.md`) and the SignPath Foundation application. Both submissions reference the threat model + reproduce.sh / reproduce.md, which need to reflect the new security work to make a good impression.

### Concrete files the new session should read first

- `~/.claude/state.json` — task list + known issues
- `~/.claude/plans/soft-herding-whale.md` — the security + rebrand plan (rebrand half complete; security half pending)
- `docs/threat-model.md` — needs the new T-row added (A2)
- `components/opencli-container/proxy/vault-proxy.py` lines 92–106 — the IP-literal denial logic to test (A1)
- `components/opencli-container/proxy/allowlist.txt` — current allowlist
- `compose.yml` lines 79–80 — the `block_private=false` / `block_global=false` flags (A3 target)
- `components/opencli-container/docs/openclaw-internals.md` — Telegram proxy root cause (A3 background)
- `tests/dogfood/findings-template.md` — where the new stanza goes (A4)

---

## What landed in the rebrand (2026-05-17 → 2026-05-18)

Multi-day rebrand from Lobster-TrApp → OpenTrApp landed end-to-end. **Done is done** — no leftover rebrand work.

### GitHub side
- Parent repo renamed: `albertdobmeyer/lobster-trapp` → `albertdobmeyer/opentrapp` (GitHub auto-redirects from the old URL)
- 3 submodule repos renamed:
  - `openclaw-vault` → `opencli-container`
  - `clawhub-forge` → `openagent-skills`
  - `moltbook-pioneer` → `openagent-social`
- 4 release titles fixed (`Lobster-TrApp v0.x.y` → `OpenTrApp v0.x.y`)
- 4 release bodies rewritten to use new repo URL + OpenTrApp branding; v0.4.0 has a "🪧 Note on naming" banner explaining its pre-rebrand asset filenames
- Repo `homepage` fixed (was a stale URL pointing at the maintainer's pre-2026 GitHub username `gitgoodordietrying`; now `https://opentrapp.com`)
- Repo description rewritten: "A safer way to run autonomous CLI agents on your own computer. Open-source, MIT, community-driven."
- Repo topics: dropped `openclaw`, added `opentrapp`, `cli-agents`, `ai-safety`, `container-security`, `skill-scanner`, `open-source`
- **v0.4.1** tagged + released with `OpenTrApp_0.4.1_*` asset filenames across every platform, cosign-signed, with per-platform CycloneDX SBOMs. The `releases/latest` URL — which the landing-page Download button uses — auto-resolves to v0.4.1.

### Code, config, docs
- 147+ files swept in PR #57 (parent rename + first-run migration script)
- 3 submodules rebranded inside their own repos via PRs #4 / #3 / #1, then wired in PR #59 (`refactor(submodules): wire opencli-container / openagent-skills / openagent-social`)
- README + whitepaper + trifecta + ADRs + active specs reframed so **OpenClaw is the reference deployment, not the protagonist**. The architecture is described agent-agnostically; OpenClaw is named at upstream-link/CVE/feature-citation level, not in section titles or generic claims.
- Five-commitments **Values** section added to README + landing page:
  1. Safety-first, safety-always
  2. Honest about residual risk
  3. Agent-agnostic, community-driven
  4. Transparency over marketing
  5. Shared for the safety of the commons
- All "Clawbot" references replaced with "agent" / "the agent" outside historical archives and the literal upstream brand.

### Visuals / landing page
- New OpenTrApp banner logo at `logos/OpenTrApp-Logos/OpenTrApp-BannerLogo.png` (regenerated 2026-05-18 with the full wordmark — the previous file was missing the middle letters of "Open"), propagated to `app/public/logo-banner.png` and `docs/img/logo-banner.png`
- Tauri bundle icons fully regenerated via `npx tauri icon logos/OpenTrApp-Logos/OpenTrApp-SquareLogo.png`
- Custom tray icons (`tray-{green,amber,red}.png`) at 32×32 — colored disc + the OpenTrApp square logo
- Favicon → multi-resolution ICO (16/32/48/64/128/256) at `app/public/favicon.ico` + `docs/img/favicon.ico`
- New procedurally-generated `docs/bg-hero.png` (856×896, dark navy + brand-green/blue radial glows + faint hex lattice — drop-in replacement for the prior lobster-themed background)
- Hero logo got a CSS upgrade: 4-layer drop-shadow, radial brand halo behind it, diagonal `mask-image`-clipped shimmer animation that sweeps every 5.5s, hover lift, `prefers-reduced-motion` honored
- Section subtitles got semantic `<br class="claim-br">` breaks so they don't wrap at arbitrary widths on desktop (`.claim-br { display: none }` under 640px keeps mobile clean)

### Infra
- Cloudflare Origin Cert issued for `opentrapp.com` (15-year, ECDSA, installed at `/etc/ssl/cloudflare/opentrapp.com.{pem,key}` on Hetzner)
- nginx config at `/etc/nginx/sites-available/opentrapp.com` serves the landing
- nginx config at `/etc/nginx/sites-available/lobster-trapp.com` rewritten as a 301-only redirect to `https://opentrapp.com$request_uri` (using the existing LE cert at `/etc/letsencrypt/live/lobster-trapp.com/`)
- Hetzner web root `/var/www/opentrapp.com` symlinks to `/var/www/lobster-trapp.com` so existing deploy scripts keep working; both nginx vhosts reference the symlinked path
- Cloudflare in **Full (strict)** TLS mode for opentrapp.com
- CI workflow (`.github/workflows/ci.yml`) fixed: the `Compose release-notes body` step now forces `shell: bash` so Windows + macOS Intel jobs don't fail on PowerShell parsing the heredoc. This was a long-standing latent bug; pre-v0.4.0 releases had been missing their Windows MSI silently.

### Intentional residue (do not "fix")
- `app/src-tauri/src/bootstrap/migrate_from_lobster_trapp.rs` keeps "lobster-trapp" in 16 references. The migration script must reference the **legacy install paths** (`~/.lobster-trapp/`, `~/lobster-trapp/`, `dev.lobster-trapp.app`, `lobster-trapp_*` podman objects) to detect prior installs and move them to OpenTrApp paths. Removing them breaks every upgrade.
- `app/package-lock.json` line 2 + 8 — autogen, will rewrite on next `npm install`.
- `docs/social-preview/lobster-trapp.svg` — separate asset rename task; not blocking anything (used for GitHub social previews; the og:image used by the landing page is now `img/favicon.png` / `img/logo-banner.png`).
- `OpenClaw`, `ClawHub`, `ClawHavoc`, `Moltbook` — third-party proper nouns. Preserved as accurate citations. The npm package `openclaw@2026.2.26` is what's literally installed inside `vault-agent`; renaming would lie about the install.

---

## Operator queue (the maintainer drives these)

These are unchanged from prior handoffs except for status updates. They sit alongside the security work but **none of them block it.**

1. **OpenSSF Best Practices Badge** — form pre-filled at `docs/openssf-best-practices-application.md`. Submit **after** the security work is done. The form references threat-model.md + reproduce.sh; both should reflect the new T-row + the (possible) `block_private=true` re-enable.
2. **SignPath Foundation re-application** — the original was for Lobster-TrApp branding. Resubmit fresh under OpenTrApp after security work lands. Reuses the existing plan at `~/.claude/plans/ethereal-wiggling-rocket.md` for the CI integration steps once SignPath approves.
3. **Demo recording** — 60-second discovery → install → use loop. Unblocked now that v0.4.1 is shipped. Shooting script at `docs/demo/README.md`.
4. **Manual upgrade test** — install v0.4.1 on a host that already has a Lobster-TrApp install (or simulate one via `~/.lobster-trapp/` + `~/lobster-trapp/.env`). Verify `migrate_from_lobster_trapp.rs` moves state cleanly and the bot resumes on first launch.
5. **Tier C1' screenshot** — launch-button screenshot in `(ShellReady, Absent)`.
6. **Tier D1 + D2** — graceful window-close and tray-Quit termination paths.
7. **Live re-run of Tier A4** — bot's hand-off behaviour. Run `make dogfood-fresh-sessions` first.
8. **Adversarial skill staging for Tier B5** — needs ClawHub publishing credentials.
9. **Dead Cloudflare API token** at `/root/.secrets/certbot/cloudflare.ini` on Hetzner — flagged in prior handoffs, still stale. The active certbot token at `/etc/letsencrypt/cloudflare.ini` is scoped narrowly (lobster-trapp.com only, not opentrapp.com — that's why we used a Cloudflare Origin Cert for opentrapp.com instead of LE). Worth regenerating to "all zones" next time you're in the dashboard.

---

## Gotchas worth knowing

1. **Always run `make dogfood-fresh-sessions` before re-testing prompt changes.** OpenClaw's session transcripts at `/home/vault/.openclaw/agents/main/sessions/*.jsonl` cache prior responses; the model self-mimics them. Documented in `tests/dogfood/CHECKLIST.md` §0a.
2. **Cloudflare auto-injects a bot-management `<script>`** before `</body>` on every response from both `lobster-trapp.com` and `opentrapp.com`. Any byte-level diff between the live HTML and the local `docs/index.html` will show false-positive divergence. Use `ssh hetzner sha256sum` (per `docs/deploying-the-landing-page.md` §1) for sync checks.
3. **Submodule changes need separate PRs** in their respective repos. Pattern: branch in submodule → commit + push to submodule's GitHub → merge submodule PR → bump submodule reference in parent → parent PR. Used three times in PRs #4/#3/#1 + PR #59.
4. **`HUMAN-TODO.md` §4 is sensitive** (adversarial registry-staging recipe). Don't stage, commit, or push that file. Operator-only.
5. **Hetzner deploys are out-of-band from app releases.** Marketing site ships when `docs/index.html` changes via `scp` — see `docs/deploying-the-landing-page.md`. `RELEASING.md` covers app tag-and-build separately.
6. **The maintainer's GitHub handle is `albertdobmeyer`** (current). The legacy `gitgoodordietrying` is deprecated — if you see it in any URL or doc, it's stale.
7. **nginx `sites-enabled/` was non-standard** before this session — concrete files instead of symlinks. Both `lobster-trapp.com` and `opentrapp.com` are now proper symlinks to `sites-available/`. Don't replace them with concrete files again.
8. **A prior session attempted a bulk sed rebrand** that broke the migration script and replaced "OpenClaw" with "opensource" across the tree. We reverted with `git restore .` and did a more careful pass. If a similar mass-rename is ever tempting again, be surgical — don't blanket-replace vendor names.
9. **CI workflow runs on tag push (`tags: ['v*']`)** — tagging `v0.4.x` from main triggers the full release build matrix.

---

## Verified facts the implementing agent should treat as established

- **Cargo + npm + tauri.conf versions** are unified at `0.4.1`. The prior mismatch (`0.4.0` in tauri.conf, `0.3.2` everywhere else) is why pre-rebrand release assets shipped with `0.3.2` in their filenames. Never let this drift again — bump all three together when cutting a release.
- **`vault-agent` runs `npm install -g openclaw@2026.2.26`** as its agent runtime. Verified in `components/opencli-container/Containerfile` line 19. The runtime name is the real third-party package name; OpenTrApp does not fork or modify it.
- **`pause_perimeter`** at `app/src-tauri/src/commands/lifecycle.rs:87-119` is `compose stop` against the root `compose.yml`: stops all 5 containers (post-ADR-0009; was 4 prior), preserves all volumes, persists via `~/.opentrapp/paused`. (Migrated from `~/.lobster-trapp/paused` for upgraders by the migration script.)
- **`hard-kill` and `nuclear-kill`** wipe `vault-data` and the agent image. Confirmed in `components/opencli-container/scripts/kill.sh:30-49,71-72`.
- **`vault-proxy` reads `ANTHROPIC_API_KEY` per request** at `components/opencli-container/proxy/vault-proxy.py:176-181`; never gates startup; warns if absent.
- **`vault-proxy.py:92-106`** IP-literal denial: `ipaddress.ip_address(host)` succeeds for `127.0.0.1`, `172.17.0.1`, `10.x`, `192.168.x` → returns `False` → 403. This is the defense we need to pin with a regression test (A1).
- **`SIGHUP` reloads the allowlist only**, not env vars (`vault-proxy.py:49`). To pick up new keys: `compose up -d --force-recreate vault-proxy`.
- **`api.anthropic.com` is on the proxy allowlist** (`components/opencli-container/proxy/allowlist.txt:4`).

---

## Working state at session end (2026-05-18)

```
$ git log --oneline -10
97df1b1 fix(ci): force bash on the release-notes-body step
d5ee5cf brand(landing): add semantic line breaks to long section subtitles
8a88f88 chore(release): bump to 0.4.1 + neutral OpenTrApp bg-hero
e48fc23 brand(banner): regenerate banner with full "OpenTrApp" wordmark rendered
9eee043 brand(icons): refresh all icons from OpenTrApp square logo + add hero gloss/shine
f9d9a87 docs(values): demote OpenClaw to specific example + add five-commitments values section
9de26bb docs(reframe): generalize OpenClaw mentions to agent-agnostic framing
e5b56c0 Merge pull request #59 from albertdobmeyer/rebrand-submodule-integration
b5149c8 refactor(submodules): wire opencli-container / openagent-skills / openagent-social
1d1a1cb Merge pull request #57 from albertdobmeyer/rebrand-opentrapp

$ git submodule status
 75fc40a  components/openagent-social   (heads/main)
 190e66a  components/opencli-container  (heads/main)
 a2b0af8  components/openagent-skills    (heads/main)
```

Working tree clean. All test gates green at v0.4.1:
- cargo lib 72/72
- vitest 74/74
- tsc clean
- orchestrator-check 42/42 (0 warnings)
- Playwright + CodeQL + fuzz × 3 + supply-chain audit all green

---

## Memory pressure caveat (still applies)

Maintainer's dev machine is a 2017 Lenovo IdeaPad with 7.2 GB RAM. Heavy parallel operations swap. Per maintainer's `~/.claude/CLAUDE.md`, max two Claude Code sessions simultaneously (one terminal, one Cursor). Stop dev servers and Ollama models between demos; check `free -h` periodically; if swap > 500 MB, stop everything non-essential before continuing.

CI runs all heavy work; nothing in the security tasks above requires the maintainer's machine to be the bottleneck.

---

## Cross-doc reference graph (orientation)

- **Threat model:** `docs/threat-model.md` (needs A2 edit)
- **Whitepaper:** `docs/whitepaper.md`
- **Architecture:** `docs/trifecta.md`, `docs/diagrams.md`, `docs/adr/`
- **Reproducibility:** `docs/reproduce.md` + `docs/reproduce.sh`
- **Releasing:** `RELEASING.md`, `docs/deploying-the-landing-page.md`
- **Dogfood test rig:** `tests/dogfood/README.md`, `tests/dogfood/CHECKLIST.md`, `tests/dogfood/findings-template.md` (needs A4 edit)
- **Skill-installation policy:** `docs/specs/2026-05-06-skill-installation-policy.md` — Option B accepted, user-bridge model
- **Plan files:** `~/.claude/plans/soft-herding-whale.md` (security + rebrand), `~/.claude/plans/ethereal-wiggling-rocket.md` (SignPath integration)
