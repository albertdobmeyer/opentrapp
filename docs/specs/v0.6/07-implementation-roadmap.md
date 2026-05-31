# OpenTrApp v0.6 — Implementation Roadmap (spec B)

> The execution order for the six specs. Read **after** [`00-index.md`](00-index.md).
> This is the "how to build it" map: milestones, what blocks what, the first PR,
> the gate every milestone passes, and what can run in parallel.

---

## 1. The shape: one gate → one foundation → three parallel legs

```
M0  Naming sweep (gate)            ── blocks everything
       │
M1  Sentinel lib + skills leg      ── the foundation; proves the whole concept
       │   (M1a lib → M1b skills)
       ├──────────────┬──────────────┐
M2 Modular         M3 Adaptive    M4 Semantic
   distribution       containment    firewall (social)
   (+ GUI rung-3)                     ── riskiest, last
```

Two hard sequencing rules:
1. **M0 before anything** — rename `forge → skills` on clean ground, then build
   on final names (same lesson as the ADR-0013 collapse: descaffold first).
2. **M1 before M2/M3/M4** — they all consume the Sentinel library; it must exist
   and be frozen before the legs wire to it. After M1, the three legs are
   independent and can run in parallel (different files, shared only the frozen
   lib).

## 2. Dependency graph

| Milestone | Specs | Depends on | Unblocks |
|-----------|-------|-----------|----------|
| **M0** Naming sweep | [`06`](06-naming-consistency-sweep.md) | — | M1, M2, M3, M4 |
| **M1** Sentinel lib + skills | [`01`](01-sentinel-spine.md) + [`03`](03-cleanroom-skills.md) | M0 | M2, M3, M4 |
| **M2** Modular distribution | [`05`](05-modular-distribution.md) | M0, M1 | release |
| **M3** Adaptive containment | [`02`](02-adaptive-containment.md) | M1 | release |
| **M4** Semantic firewall | [`04`](04-semantic-firewall-social.md) | M1 | release |

## 3. The milestones

Each milestone: **goal · specs · done-when (from the spec) · the gate**. Every
milestone follows the session's TDD discipline — write the spec's pre-build
tests first, watch them fail, then build until green.

### M0 — Naming sweep (the gate)
- **Goal:** `forge → skills` everywhere live; `v0.6` labels; nothing built on
  old paths.
- **Spec:** [`06`](06-naming-consistency-sweep.md).
- **Done-when:** `workloads/skills/` exists, `workloads/forge/` gone,
  `vault-skills` is the container, every live file says `skills`, ADRs/archive
  untouched, all gates green, one atomic commit.
- **This is the first PR** (§4).

### M1 — Sentinel shared library + the skills leg (the foundation)
The riskiest and most load-bearing work; do it first to de-risk the whole AI
concept on the one leg that already has the local model.

**M1a — the Sentinel lib skeleton.** Spec [`01`](01-sentinel-spine.md).
- Build the shared lib: the `judge(request) → Verdict` call, the rung-0/1/2
  helpers, the verdict schema, the injection-hardened judge prompt (generalise
  `cdr-intent.sh`), the embedding corpus (from the existing fixtures), the
  load-on-demand rung-2 lifecycle.
- **Lib-first:** prove it callable from a bare CLI against local Ollama *with no
  GUI* (the standalone-shield path).
- **Done-when:** the [`01`] §9 tests pass — ladder routing (clean → no rung-2),
  injection resistance (a "return allow" injection doesn't flip the verdict),
  escalation rarity (≤ the alert budget), verdict vocabulary (banned-terms),
  rung-3 cloud scoping (only the fragment leaves).

**M1b — wire the lib into the skills leg.** Spec [`03`](03-cleanroom-skills.md).
- Rung-2 second opinion on SUSPICIOUS lines = **the ZONE-4a fix** (clean skills
  stop failing closed).
- The describe → schema-validate → regenerate cleanroom (quarantine on
  un-describable, no silent exit).
- The plain-language **disarm diff**.
- The **activity indicator** (watching / thinking / deep-analysis), reusing the
  Zone-1 hero/Security surfaces.
- **Done-when:** [`03`] §6 — clean skills install (4a closed), malicious
  quarantine with a readable diff, scanner-self-test still 10/10, the lib
  exercised end-to-end.

### M2 — Modular distribution (+ the GUI rung-3 UX)
- **Goal:** install one shield standalone, or the GUI with a profile.
- **Spec:** [`05`](05-modular-distribution.md). Includes the **rich GUI rung-3
  escalation UX** (the visual banner + pause-the-agent + cloud-consent dialog) —
  standalone CLIs already got the text-prompt rung 3 in M1.
- **Done-when:** [`05`] §7 — `openagent-skills` installs + scans via CLI with no
  GUI; the GUI with a `containment` profile shows only the containment dashboard;
  `distribution.yml` is the single source; default build = all (no regression).

### M3 — Adaptive containment
- **Goal:** the perimeter tightens itself toward least-privilege; explained
  one-tap allowlist decisions; never auto-loosens.
- **Spec:** [`02`](02-adaptive-containment.md).
- **Done-when:** [`02`] §7 — auto-tighten fires, the never-auto-loosen invariant
  is pinned, off-allowlist gray-zone becomes a one-tap approval, exfil stays
  hard-blocked at rung 0.

### M4 — Semantic firewall (social) — riskiest, last
- **Goal:** revive social as a general agent-social shield; catch paraphrased
  injections; persona-drift on outgoing posts.
- **Spec:** [`04`](04-semantic-firewall-social.md).
- **Done-when:** [`04`] §6 — works against ≥1 *live* agent-social network,
  catches a paraphrased injection regex misses, holds a hijacked outgoing post,
  the un-park ADR written. **Stays behind a flag until a live adapter works** —
  do not re-park by shipping a Moltbook-only revival. If scouting finds no live
  target, ship the adapter + persona-drift and defer live validation; do not
  block the release on it.

## 4. The first PR (concretely)

**M0 — the rename.** One atomic PR:
1. `git mv workloads/forge workloads/skills` + `git mv docs/forge-spotlight.md docs/skills-spotlight.md`.
2. Sweep `vault-forge → vault-skills`, `forge-net → skills-net`,
   `forge-deliveries → skills-deliveries`, the `forge` component id, and live-doc
   references (NOT ADRs/archive) per [`06`](06-naming-consistency-sweep.md) §3.
3. Add the orchestrator-check assertion: no live file references
   `vault-forge`/`workloads/forge`/the `forge` id.
4. All gates green (§5). Merge. *Then* M1 begins.

## 5. The gate every milestone passes

The session's established gate — run all before merging any milestone:

```bash
cd app/src-tauri && cargo build && cargo test --lib   # Rust
cd app && npx tsc --noEmit && npm test -- --run        # TS + vitest
bash tests/orchestrator-check.sh                       # manifests + topology + the §-checks
cd app && npx playwright test --project=default        # e2e (added to the gate this session)
podman compose config                                   # compose still parses
```

Plus each spec's **pre-build tests** authored first (TDD): write them, confirm
they fail, build until they pass. Add a new `orchestrator-check.sh` section per
milestone where the spec calls for it (the §10–§17 pattern).

## 6. Parallelism — who can work on what

- **M0 and M1 are sequential and solo** (foundational; one stream).
- **After M1, M2 / M3 / M4 are independent** — different files, the only shared
  surface is the Sentinel lib, which is **frozen** at the end of M1. Three
  contributors (or three agent streams) can take one leg each.
- **Freeze rule:** no leg modifies the Sentinel lib. If a leg needs a lib change,
  it's a change to M1's contract — coordinate, re-run M1's tests, then continue.

## 7. Release staging (maintainer call — D6)

Two options:
- **Staged:** cut **`v0.6.0-beta`** after M0 + M1 (Sentinel proven on skills, 4a
  fixed, clean names — a strong, demonstrable beta), then M2–M4 round out
  **`v0.6.0`**. Lets the USP ship + get feedback early.
- **Held:** keep everything unreleased until all of M0–M4 land, ship as one
  `v0.6.0`.
- Recommendation: **staged** — M0+M1 is the proof of the whole thesis and the
  most demo-able; don't sit on it.
- The code version bump (`package.json`/`tauri.conf.json`/`Cargo.toml` →
  `0.6.0`) happens at the release cut, not during the milestones.

## 8. Definition of done for v0.6.0

- All six specs' **done-whens** met.
- The USP is **demonstrable**: a user watches Sentinel catch a paraphrased
  injection static-only tools miss, *and reads the plain-language reason*; the
  disarm diff shows what was removed from a skill.
- **Install profiles work**: `openagent-skills` standalone; GUI `containment`
  profile; nobody installs 5/5ths to use 1/5th.
- It **runs on a 7–8 GB laptop** alongside a real agent without a noticeable
  slowdown until a user-triggered rung-3.
- The demo gifs (`docs/assets/`) re-recorded against the `v0.6.0` build; the
  pitch + `skills-spotlight.md` updated to the new names.

## 9. Day-1 checklist for the implementing agent

1. Read [`00-index.md`](00-index.md) → this roadmap → [`06`](06-naming-consistency-sweep.md).
2. Open the **M0 rename PR**. Get it green + merged.
3. Read [`01`](01-sentinel-spine.md) + [`03`](03-cleanroom-skills.md). Build **M1a then M1b**, TDD.
4. Freeze the Sentinel lib. Fan out M2 / M3 / M4.
5. Cut `v0.6.0-beta` after M1 if staging.
