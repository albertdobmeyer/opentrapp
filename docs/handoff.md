# Handoff — Active Mission

**Last updated:** 2026-05-04 (whitepaper + 3 ADRs landed; post-launch roadmap published; 6 of 8 enrichment areas queued for the next session)
**Current phase:** Post-v0.3.0 enrichment. The 3-week "Delightful Sloth" UX-coherence polish phase concluded with the v0.3.0 ship recommendation on 2026-05-02; the codebase-cleanup pass concluded on 2026-05-03. This phase is about elevating the project from "shipped open-source tool" to "publishable security-research project."
**Branch:** `main` — pushed to `origin/main`
**Tag:** `v0.3.0` at `75dbccb` with 9 platform binaries attached

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

**Pick up at:** [`docs/roadmap-post-launch.md`](roadmap-post-launch.md). Six of the eight enrichment areas are queued. Recommended next item: §1 (formal threat model). See "What's queued" below for the full ordering and rationale.

---

## What landed in this session (2026-05-04)

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

Six of the eight roadmap items remain. Recommended ordering, with rationale:

| Roadmap | Item | Rationale for ordering |
|---|---|---|
| §1 | Formal threat model | Highest credibility lever for a security-focused project. Foundation for citations from §5 (prior art). ~1 focused day. |
| §4 | Reproducibility section + SLSA / SBOM in CI | Engineering rigor; mostly mechanical once the spec is written. Half day for `reproduce.md`, ~1 day for the CI work. Best done after §1 because the threat model's "evidence" cells will reference reproducible commands. |
| §5 | Prior-art comparison | Pre-empts the most common reviewer question. Half day. |
| §8 | CONTRIBUTING.md + CODE_OF_CONDUCT.md | Standard open-source hygiene; missing-but-expected. Quick win — 1–2 hours. |
| §6 | Mermaid architecture diagrams | Visual polish for README and trifecta.md. Half day. |
| §7 | Demo recording for the landing page | Last because it benefits from a stable v0.3.x release and a clean recording machine. |

§1, §4, §5, §8 can be done in parallel by different contributors. §6 and §7 are visual-polish work and should batch with whatever the next visible release is.

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
