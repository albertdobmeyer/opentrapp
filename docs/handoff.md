# Handoff — Active Mission

**Last updated:** 2026-05-05 (dogfood full-arc test rig authored; run pending in next session)
**Current phase:** Test execution. The publication-ready doc set, supply-chain attestation pipeline, and v0.3.2 release-prep are all in main. The next session's mission is to **run the dogfood full-arc test** authored in this session and produce the empirical findings doc.
**Branch:** `main` — pushed to `origin/main`
**Tag:** `v0.3.0` at `75dbccb` with 9 platform binaries attached. v0.3.1 tag exists but has no artefacts (its build was skipped). v0.3.2 release-prep merged to main but tag not yet cut.

---

## RUN THIS NEXT — Dogfood Full Arc

**Mission:** simulate Karen, the non-technical end user, downloading and using Lobster-TrApp. Drive the full arc — discovery → install → wizard → first chat → five jobs → wind-down — plus three additional tiers stress-testing every defensive layer, every AssistantStatus state, and every termination path. Capture signals; produce the findings doc.

The user's explicit direction (2026-05-05): **the most unconservative settings — push every defensive layer to its limits, real ClawHub installations, real Anthropic API, adversarial prompt-injection attempts, edge cases.**

### Where to start

1. Read [`tests/dogfood/README.md`](../tests/dogfood/README.md) — the index that ties the four artefact files together.
2. Read [`docs/specs/2026-05-05-dogfood-full-arc-spec.md`](specs/2026-05-05-dogfood-full-arc-spec.md) — the spec (persona, four tiers, 27 scenarios, success criteria).
3. Walk [`tests/dogfood/CHECKLIST.md`](../tests/dogfood/CHECKLIST.md) — the operator-facing checklist (pre-flight, per-scenario gates, post-run sign-off).
4. Run the harness: `cd tests/e2e-telegram && source .venv/bin/activate && cd ../dogfood && pytest -m dogfood_full -xvs`.
5. Fill in [`tests/dogfood/findings-template.md`](../tests/dogfood/findings-template.md) — copy to `docs/specs/2026-05-DD-dogfood-full-arc-findings.md` and populate.

### Pre-flight requirements (the run-session will check these)

- `main` at the latest commit, all CI green
- A fresh Telegram bot (do NOT reuse a personal account)
- A fresh Anthropic API key with $1 hard spending cap
- `.env.test` at the repo root with the harness credentials (see `tests/e2e-telegram/SECONDARY_ACCOUNT_SETUP.md`)
- All four containers down before session start (`podman ps` empty; `~/.lobster-trapp/runguard.pid` absent)
- The dogfood corpus at `tests/dogfood/corpus/` populated (committed stubs are usable; replace for a "real Karen" run)

### What "ship-recommended" looks like at session end

- All Tier A scenarios pass with usable, non-jargon replies
- All Tier B scenarios bounce off their defensive layer; zero credential or workspace leaks
- All Tier C states render with calm, jargon-free copy
- All Tier D paths reach clean teardown
- `verify.sh` start = `verify.sh` end (architecture invariant)
- API spend < $0.50
- Zero banned-term hits in any reply
- Operator's qualitative read of the bot voice is "natural, calm, helpful"

### Cost & time envelope

~70 minutes wall-clock; ~$0.40 of Anthropic spend (cap $0.50). Tier A is the longest segment at ~35 min.

### What this rig **doesn't** do (and what to do about it)

- **Doesn't run the wizard** — operator does that part by hand. The Telethon harness picks up after the bot is paired.
- **Doesn't supersede the existing per-boundary tests** in `tests/e2e-telegram/` — Tier B references them rather than duplicating.
- **Doesn't enforce subjective UX failures** — banned-term leaks, credential leaks, and architecture invariants are hard asserts; bot copy / latency / quality are *recorded* but don't fail the run. The findings doc author scores severity.

---

**Recent commits (in chronological order):**

| Commit | Date | Subject |
|---|---|---|
| _(this commit)_ | 2026-05-04 | Whitepaper + 3 ADRs + post-launch roadmap |
| `9b9f6c8` | 2026-05-03 | Final allegory sweep (Pass-2 spec, rubric, dogfood, e2e harness) |
| `f743a38` | 2026-05-03 | Submodule pointer bumps for forge + pioneer cleanup passes |
| `1bc288f` | 2026-05-03 | Comprehensive academic-tone pass on the publication-ready doc set |
| `4108482` | 2026-05-03 | Codebase cleanup pass — strip dead code, allegory remnants, stale specs |
| `73ca17f` | 2026-05-03 | Rewrite README and trifecta.md in concise, academic tone |
| `b5889d7` | 2026-05-02 | v0.3.0 landing-page bump + release notes |
| `75dbccb` | 2026-05-02 | Bump internal version 0.2.0 → 0.3.0 to match release tag |
| `104e2c4` | 2026-05-02 | Repair corrupted function-bind integrity hash + honest README rewrite |
| `7ebdd8b` | 2026-05-02 | Pass 8: pre-ship walkthrough + SHIP recommendation |

**Pick up at:** [`docs/roadmap-post-launch.md`](roadmap-post-launch.md). All eight enrichment areas have deliverables in `main` as of this commit; only the demo video itself (§7's recorded asset) is queued, with a complete scaffold at [`docs/demo/README.md`](demo/README.md). Next-session candidates: cut a v0.3.x release that includes the SLSA/SBOM/cosign attestations now that the CI workflow is in place; or schedule the demo-recording session.

---

## What landed in *this* session (2026-05-04, second commit of the day)

Six of the eight roadmap items completed in a single autonomous pass; the seventh has a full scaffold; the eighth (whitepaper + ADRs) was already done in the morning commit. Files written or modified:

| Roadmap item | Deliverable | Status |
|---|---|---|
| §1 Threat model | [`docs/threat-model.md`](threat-model.md) — STRIDE matrix across T1–T6 with residual-risk + evidence per row | Landed |
| §2 Whitepaper | [`docs/whitepaper.md`](whitepaper.md) | Already landed (morning commit) |
| §3 ADRs | [`docs/adr/0001`](adr/0001-proxy-side-api-key-injection.md) through [`0008`](adr/0008-tauri-over-electron.md) | First three landed in the morning commit; the remaining five (pioneer parking, deserve-to-exist, four-container topology, manifest-driven backend, Tauri choice) landed in this evening's third commit |
| §4 Reproducibility + SLSA/SBOM | [`docs/reproduce.md`](reproduce.md) + [`docs/reproduce.sh`](reproduce.sh); [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) gains a tag-only attestation block (cosign keyless + syft SBOM + SLSA build provenance) | Landed |
| §5 Prior-art | [`docs/why-not-x.md`](why-not-x.md) — page-or-two differential against 9 alternatives (sandbox.mode alone, Firejail, gVisor, OS sandboxes, VM-only, scanner-only, allowlist-only, no-perimeter, capability-OS) | Landed |
| §6 Mermaid diagrams | [`docs/diagrams.md`](diagrams.md) — 5 diagrams (topology, trust tiers, network isolation, CDR pipeline, AssistantStatus state machine); README + trifecta embed selected diagrams | Landed |
| §7 Demo recording | [`docs/demo/README.md`](demo/README.md) — shooting script, ffmpeg recipe, conversion targets, pre-publish checklist; [`docs/index.html`](index.html) carries a commented-out `<video>` block ready to enable | Scaffold landed; recording itself queued |
| §8 CONTRIBUTING + CoC | [`CONTRIBUTING.md`](../CONTRIBUTING.md), [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md), [`.github/pull_request_template.md`](../.github/pull_request_template.md) | Landed |

Cross-references added: README → threat-model + why-not-x + reproduce + diagrams; SECURITY → threat-model; trifecta → threat-model + diagrams.

Test-count drift caught and corrected: README and CLAUDE.md previously stated `Vitest (175)` which did not match the actual `Tests 74 passed (74)`. The 74 figure comes from the whitepaper §8 and the morning handoff and is the v0.3.0 ground truth; the 175 was stale. README + CLAUDE.md + CONTRIBUTING + reproduce.md + reproduce.sh now say 74 consistently.

## What landed earlier today (2026-05-04, morning commit)

Three artefacts published, plus a roadmap that captures the rest of the enrichment work:

### `docs/whitepaper.md` (~10 pages)

A consolidated paper-style document covering the architecture's problem statement, threat model, system design, defense-in-depth layers, the two key innovations (adaptive shell levels and the CDR pipeline), implementation choices, empirical evaluation, limitations, and related work. Written in arXiv-cs.CR-readable register; cites the three ADRs and the live source files. Suitable as the canonical introduction for a security-research reader; also suitable as the basis for an arXiv preprint with one additional pass to align with arXiv author guidelines.

The whitepaper makes the empirical claims of the README citable — every numerical claim (87 patterns, 11.9 % ClawHavoc rate, 24-point verification, 42-check orchestrator) carries a footnote-style pointer to the verification artefact that supports it. A planned reproducibility document (roadmap §4) will turn each of those into an executable command.

### `docs/adr/` — three Architecture Decision Records

Standard ADR format ([adr.github.io](https://adr.github.io/)): status / context / decision / consequences / alternatives considered / references. Three records covering the three architectural choices most distinctive to this project:

- **ADR-0001:** Proxy-side API-key injection — the cornerstone credential-isolation pattern. Documents why the API key is held in `vault-proxy` and never enters `vault-agent`, which alternatives were rejected, and what residual risk the design accepts.
- **ADR-0002:** Adaptive shell levels (Hard / Split / Soft) as a system state — the agent's privilege-modulation mechanism. Documents why privilege is treated as a state that tracks task context rather than a single configuration value, and the demote-freely / promote-with-confirmation discipline.
- **ADR-0003:** Content Disarm and Reconstruction for skills — the supply-chain defense pattern. Documents why CDR is layered on top of static scanning and line classification rather than as a replacement, and the decoupling that makes the design composable with future trust-model changes.

A `docs/adr/README.md` index lists future ADRs queued for writing (the moltbook-pioneer parking decision, the 2026-05-02 vision recheck, the four-container topology choice, the manifest-driven generic backend, and the Tauri choice).

### `docs/roadmap-post-launch.md`

Eight enrichment areas, each with deliverable / scope / dependencies / effort estimate / definition-of-done. Recommended sequencing in the document. Two items completed this session (whitepaper + ADRs); six remain.

---

## What's queued for next session

The roadmap is now empty of writeable work. The two outstanding items are operational:

1. **Cut a v0.3.x release** that exercises the new attestation block in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml). The workflow's tag-only path generates the SBOM, signs assets via cosign keyless, and emits the SLSA build-provenance attestation; none of this has been exercised against a real release yet. The first release after this commit is the smoke test for that pipeline.
2. **Record the demo video.** Plan and shooting script in [`docs/demo/README.md`](demo/README.md). Half-day session, dominated by re-takes; needs a clean recording machine and a fresh API key + Telegram bot.

Optional follow-ups that came up during the documentation work and are tracked inline rather than as roadmap items:

- **Certificate pinning for upstream Anthropic / Telegram endpoints** (mentioned in [`docs/threat-model.md`](threat-model.md) T3 residual risks).
- **Fuzzing the CDR parser and generator** (T2 residual risks).
- **Per-platform documentation of what persists after `compose down`** (T6 residual risks).
- **Friction-effect measurement on the per-action approval gate** (T5 residual risks).
- The five additional ADRs that were queued (pioneer parking, deserve-to-exist, four-container topology, manifest-driven backend, Tauri choice) are now landed; no further ADRs queued.

---

## Working state at end of this session

```
$ git status
On branch main
Your branch is up to date with 'origin/main'.
nothing to commit, working tree clean

$ git log --oneline -3
<this commit> Add whitepaper + 3 ADRs + post-launch roadmap
9b9f6c8       Final allegory sweep (Pass-2 spec, rubric, dogfood, e2e harness)
f743a38       Submodule pointer bumps for forge + pioneer cleanup passes

$ cd app/src-tauri && cargo test --lib
test result: ok. 56 passed; 0 failed; 0 ignored

$ cd app && npm test -- --run
Tests  74 passed (74)

$ npx tsc --noEmit
(clean)

$ npx playwright test
25 passed (51s)

$ bash tests/orchestrator-check.sh
Results: 42 passed, 0 failed, 0 warnings (total: 42 checks)
```

All four submodules are at their current heads (`openclaw-vault c60b451`, `clawhub-forge 5bac4fb`, `moltbook-pioneer 52b3db2`).

---

## Cross-doc reference graph (for orientation)

The publication-ready doc set as of this session forms the following citation graph. Anyone touching one document should expect the others to need a pass for consistency.

```
README.md ─────────────► whitepaper.md ◄─── arXiv preprint (future)
   │                       │   ▲
   │                       │   │
   │                       ▼   │
   │                    trifecta.md ◄── architecture-v2 origin spec (archived)
   │                       │
   │                       ├──► adr/0001 (proxy-side key injection)
   │                       ├──► adr/0002 (adaptive shell levels)
   │                       └──► adr/0003 (CDR)
   │
   ├──► SECURITY.md ◄───── (planned: docs/threat-model.md)
   ├──► CLAUDE.md
   └──► GLOSSARY.md
            │
            └─► §9 historical-term mapping (older specs in archive use this)
```

Live tree under `docs/`:

```
docs/
├── handoff.md                          (this file)
├── whitepaper.md                       (new)
├── trifecta.md
├── roadmap-post-launch.md              (new)
├── release-notes-v0.2.0.md
├── release-notes-v0.3.0.md
├── index.html (+ bg-hero.png + hero.png)
├── adr/                                (new)
│   ├── README.md
│   ├── 0001-proxy-side-api-key-injection.md
│   ├── 0002-adaptive-shell-levels.md
│   └── 0003-content-disarm-reconstruction.md
├── specs/                              (active specs cited from current docs)
│   ├── 2026-04-19-product-identity-spec.md
│   ├── 2026-04-20-ux-principles-rubric.md
│   ├── 2026-04-25-tool-mediation-pattern.md
│   ├── 2026-04-28-dogfood-walkthrough-findings.md
│   ├── 2026-04-29-delightful-sloth-target-ux.md
│   ├── 2026-04-29-live-signal-first-chat.md
│   ├── 2026-05-02-pass-8-preship-walk.md
│   └── ui-rebuild-2026-04-21/
└── archive/
    ├── README.md
    ├── 2026-03-03-todo.md
    ├── 2026-03-27-vision-and-status.md
    ├── 2026-04-09-landing-page-handoff.md
    ├── 2026-04-16-roadmap-v4-finalization.md
    ├── 2026-04-24-handoff-pioneer-gaps.md
    ├── 2026-04-24-product-assessment.md
    ├── 2026-04-25-v0.2.0-ship-plan.md
    ├── specs/                          (9 archived design specs)
    └── superpowers/                    (3 archived superpower specs + 7 archived plans)
```

---

## How to verify what landed this session

```bash
# 1. The three documents render and cross-link correctly:
ls docs/whitepaper.md docs/adr/0001-*.md docs/adr/0002-*.md docs/adr/0003-*.md docs/roadmap-post-launch.md

# 2. Every cited file path resolves:
grep -E '\[`.*?`\]\([^)]*\.(md|sh|py|rs|ts|tsx|json|yml|yaml|json5)\)' docs/whitepaper.md docs/adr/*.md \
    | sed 's/.*](//;s/).*//' | sort -u | while read p; do [ -f "$p" ] || echo "MISSING: $p"; done

# 3. Test gates still green (these run anyway in CI on push):
( cd app/src-tauri && cargo test --lib )
( cd app && npm test -- --run )
( cd app && npx tsc --noEmit )
( cd app && npx playwright test )
( bash tests/orchestrator-check.sh )

# 4. No allegory has crept back into active surfaces:
grep -rnE "warden|cell block|cage\b|arena\b|safari\b|inmate|prison|leash\b|exoskeleton|driver seat|the workshop\b|monitoring station|gear [123]\b|containerized workshop|moat\b" \
    --include="*.md" --include="*.sh" --include="*.yml" --include="*.yaml" --include="*.json" --include="*.json5" --include="*.py" --include="*.rs" --include="*.ts" --include="*.tsx" \
    | grep -vE "\.git/|node_modules|__pycache__|docs/archive/|skills/|tests/e2e-telegram/VERDICT-"
```

---

## Memory pressure caveat (still applies)

The dev machine hits ~5.2 GB used + ~2.9 GB swap during cargo+tsc parallel runs. Mid-session checklist (from the user's global CLAUDE.md):

```bash
free -h                                       # check
pkill -f "vite" 2>/dev/null                  # kill orphans
pkill -f "chromium.*--test-type" 2>/dev/null
ollama stop qwen2.5-coder:7b 2>/dev/null     # if loaded
```

Tauri dev launch (`cd app && npm run tauri dev`) costs ~1.5 GB + ~2 min build. Skip when TSC + e2e already cover the change.

---

## Things deliberately not done in this session

To keep scope honest:

- **No threat-model document.** Queued as roadmap §1. The whitepaper §2 contains a five-attacker summary; the formal STRIDE-style matrix is a separate document.
- **No CI changes for SLSA / SBOM.** Queued as roadmap §4. The CI workflow at `.github/workflows/ci.yml` is unchanged.
- **No prior-art comparison document.** Queued as roadmap §5. The whitepaper §10 names six prior-art categories; a deeper comparison is a separate document.
- **No CONTRIBUTING.md or CODE_OF_CONDUCT.md.** Queued as roadmap §8. GitHub's "community standards" page still flags both as missing.
- **No mermaid diagrams.** Queued as roadmap §6. ASCII diagrams in README, trifecta, whitepaper, and ADRs are unchanged.
- **No demo recording for the landing page.** Queued as roadmap §7.
- **No additional ADRs beyond the three written this session.** The `adr/README.md` index lists five more queued; each ~30–60 minutes once the format is set.
- **No arXiv submission.** The whitepaper is publishable-shaped but not formatted for arXiv author guidelines. A future session could spend a day aligning to those guidelines and submitting.
- **No external review or audit.** A paid Trail of Bits / Doyensec review would be the highest-credibility signal for a security-focused project; out of scope for documentation work but worth budgeting for.
