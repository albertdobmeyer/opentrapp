# ADR-0013 — Monorepo consolidation: collapse three submodules into `workloads/` + `infra/`

**Status:** Accepted — landed 2026-05-30
**Supersedes:** none structurally; reframes the "three modules" framing introduced before [ADR-0006](0006-four-container-topology.md)
**Implemented by:** the 2026-05-30 collapse (`components/` → `workloads/` + `infra/`; `.gitmodules` deleted; build/config/Rust/TS references rewritten; CI submodule-checkout removed)
**Verified by:** `cargo test --lib`, `npm test`, `npx tsc --noEmit`, `bash tests/orchestrator-check.sh`

---

## Context

Through v0.5.0 the repository tracked three git submodules under `components/`:

- `opencli-container/` — agent runtime + the egress chain (vault-agent + vault-proxy + vault-egress)
- `openskill-forge/`   — skill scanner + CDR (vault-forge)
- `openagent-social/`  — social-feed analyser (vault-pioneer, parked)

The submodule layout dated to a period when each module was developed somewhat
independently with the aspiration that other consumers might adopt them standalone. By
late 2026-05 that aspiration was no longer load-bearing, but the submodule machinery
remained — and was costing us, even when nothing else was happening:

- The three submodule HEADs landed within **10 seconds of each other** on 2026-05-19. They did not have independent lifecycles; they co-shipped in lockstep with the parent repository.
- The parent repository's git history was littered with "chore(submodule): bump …" commits — every change in a submodule required a manual bump in the parent.
- Pre-flight checks (2026-05-30) confirmed each submodule repo had **≤1 star, 0 forks, 0 open issues, 0 open PRs, 0 external watchers**. No standalone consumers existed.
- `CLAUDE.md`'s "Submodule discipline" section documented a two-step commit workflow contributors had to learn (clone with `--recurse-submodules`, sync after each submodule change, never modify a submodule without also pushing to its own remote) — process tax with no offsetting benefit.
- Writing the [`docs/perimeter-explained.md`](../perimeter-explained.md) one-pager (Thread B of MISSION.md) surfaced a deeper structural problem: `opencli-container` had become a junk drawer because it bundled the agent workload with the L7 + L3 egress chain that forge and social *also* depended on. The submodule layout was no longer 1:1 with the conceptual decomposition.

The standard test for whether a submodule earns its cost is: *does it have an independent
release cadence consumed by more than one parent?* Ours failed both halves of that test.

## Decision

Collapse to a single monorepo with a flat workload/infra split.

```
opentrapp/
├── app/                              Tauri 2 + React 18 desktop application (the orchestrator)
├── workloads/                        one directory per workload container
│   ├── agent/                          → vault-agent   (runtime containment)
│   ├── forge/                          → vault-forge   (skill scanner + CDR)
│   └── social/                         → vault-social  (agent-social-feed analyser; parked)
├── infra/                            shared infrastructure containers
│   ├── proxy/                          → vault-proxy   (L7 egress policy)
│   └── egress/                         → vault-egress  (L3 egress policy)
├── compose.yml                       five-service perimeter compose definition
├── schemas/component.schema.json     workload manifest contract
├── config/orchestrator-workflows.yml cross-workload workflow definitions
└── tests/                            orchestrator-check + dogfood harness + e2e
```

Three principles drive the layout:

1. **One concern per directory.** Each workload directory builds exactly one container that
   implements exactly one area of concern (agent runtime, skill scanning, social analysis).
   The directory name matches the container name; no indirection.
2. **Infra is shared, not owned.** The L7 + L3 egress chain (vault-proxy + vault-egress) is
   infrastructure that all three workloads depend on — it does not belong to any single
   workload. Lifting it into `infra/` makes the sharing explicit and ends the "opencli-container
   owns three of the five containers" awkwardness that ADR-0009's split exposed.
3. **The orchestrator is the parent.** `compose.yml` (the topology), `app/src-tauri/`
   (the lifecycle), and `schemas/` + `config/` (the contract) all live at the root,
   because composition is a property of the whole, not of any subdirectory.

### Identity rename

Workload identifiers (the `identity.id` field in each `component.yml`, plus every Rust
constant and TypeScript string that references them) are renamed to match the new
directory names:

| Before | After |
|--------|-------|
| `opencli-container` | `agent` |
| `openskill-forge`   | `forge` |
| `openagent-social`  | `social` |
| `vault-pioneer`     | `vault-social` |
| `pioneer-net`       | `social-net` |

The container/network rename (vault-pioneer → vault-social) was independently warranted —
"pioneer" was a Moltbook-specific legacy name that no longer fit the un-parking direction
laid out in MISSION.md Thread C (generalized agent-social shield).

### Implementation steps (executed 2026-05-30)

1. `rsync -a --exclude=.git` each submodule's working tree into its new home.
2. `git submodule deinit -f` for all three; `git rm -rf components/`; delete `.gitmodules`;
   remove `.git/modules/components/*`.
3. Rewrite path references in `compose.yml`, `app/src-tauri/resources/perimeter.yml`,
   `app/src-tauri/build.rs`, the Tauri orchestrator's Rust source, the React frontend's TS
   source, `config/orchestrator-workflows.yml`, `tests/orchestrator-check.sh`,
   `tests/integration-test.sh`, the e2e-telegram test suite, `.github/workflows/ci.yml`,
   `docs/reproduce.{md,sh}`, and the live root docs (README.md, CLAUDE.md, GLOSSARY.md,
   SECURITY.md, CONTRIBUTING.md, docs/trifecta.md, docs/perimeter-explained.md,
   docs/whitepaper.md, docs/diagrams.md, docs/threat-model.md, docs/why-not-x.md).
4. Update each `component.yml`'s `identity.id` to the short name; sweep all callers.
5. Archive the three GitHub source repos with redirects in their READMEs. (Operator step,
   not part of the code commit.)

## Consequences

### Positive

- **The directory layout describes the architecture.** Reading `ls workloads/ infra/` tells
  a new contributor what the five containers do without needing to read `compose.yml` or
  follow a submodule into a separate clone.
- **One commit, one history.** A change that touches the agent workload and the egress
  chain (e.g. tightening the allowlist) lands as a single commit in a single repository,
  not as a multi-step submodule-bump dance.
- **Forge gains visibility.** Sitting as a top-level `workloads/forge/` directory with its
  own README is materially more discoverable than being a submodule one indirection away.
  This partially fulfils MISSION.md Thread D (the forge spotlight).
- **The CLAUDE.md "Submodule discipline" section disappears.** Replaced by a single line:
  "Edit, build, and commit in one place."
- **CI gets faster.** No `submodules: true` on checkouts, no submodule init step.
- **Pre-existing aspirations preserved.** If forge ever genuinely attracts a second
  consumer, it gets extracted into its own repo and consumed via a pinned OCI image or
  package — never back into a submodule. We've made the path clear; we just don't pay for
  it today.

### Negative

- **Three archived GitHub repos no longer accept issues/PRs.** Anyone with an existing
  fork is asked (via the archived-repo README) to open issues on `opentrapp` instead.
- **Git history of intra-submodule changes is preserved in the archived repos**, not in
  the consolidated history. Anyone doing archaeology on a pre-collapse commit needs to
  consult the archived repo — the consolidated history has only the flat-copy commit. We
  considered subtree-merge to preserve history inline but rejected it because (a) zero
  external forks justified the simpler flat copy and (b) the archived repos remain a
  live reference.
- **Repository on-disk size grows.** Forge in particular brings its skills/ corpus and
  templates/ directory with it. Mitigated by the fact that the alternative was the same
  bytes spread across three working trees plus `.git/modules/`.
- **Contributors with existing standalone clones** of the submodules have to adapt: their
  workflow was "edit in `~/openskill-forge/`, push, then bump submodule in `~/opentrapp/`."
  Now it's "edit in `~/opentrapp/workloads/forge/`, push." Communicated in the v0.6.0
  release notes.

### Risks accepted

- **Independent release of a single workload is harder.** If we ever want to ship a
  forge-only update without touching the rest, we no longer have a separate version
  cadence to do it. Mitigation: it's still possible to tag and release a specific
  workload's container image without touching the desktop application; that's what
  GHCR per-image tagging already supports. The submodule layout was not the only way to
  achieve that; collapsing did not remove the capability.
- **Public commitment to "three modules" needs unwinding.** Marketing copy that
  referenced three submodule repositories (README, landing page, whitepaper) was updated
  in the same change. Anyone arriving via a stale external link to one of the archived
  repos will see the redirect.

## Alternatives considered and rejected

- **Keep the three submodules, just rename `opencli-container`.** Surfaced as ZONE 8 in
  `AGENT-TODO.md`. Rejected because it would have fixed the "opencli-container is a junk
  drawer" symptom without fixing the underlying cause (the submodules don't have
  independent lifecycles). The rename would have aged badly the moment any other lifecycle
  evidence surfaced.
- **Subtree-merge to preserve submodule histories inline.** Considered for fidelity.
  Rejected because (a) pre-flight showed zero external attention on the submodule repos,
  (b) `git log --follow` on a flat-copied file still works for intra-file history, and
  (c) the archived repos remain as a live reference for anyone doing deep archaeology.
- **Promote the three submodules to peers (multi-repo, no submodules).** Rejected because
  it preserves all the lifecycle-tax overhead of the submodule layout (separate clones,
  separate CI, separate `npm install` for any shared tooling) without the on-by-default
  cross-repo wiring submodules at least provide. If genuine independent lifecycle ever
  materialises, this is the configuration to revisit — but as an *extraction*, not the
  starting point.
- **Extract `infra/` to its own repo, keep workloads as submodules.** Considered when
  ADR-0009 split vault-proxy from vault-egress. Rejected on the same lifecycle test:
  `infra/` would also have shipped in lockstep with the parent, and we'd be back to two
  submodules instead of three.

## Cross-references

- [`docs/perimeter-explained.md`](../perimeter-explained.md) — the one-page architecture explainer that surfaced the consolidation question.
- [ADR-0006 — Four-container topology](0006-four-container-topology.md) — established the "submodule per concern" framing this ADR moves away from.
- [ADR-0009 — Five-container perimeter](0009-five-container-perimeter.md) — by splitting vault-proxy from vault-egress, made it impossible for `opencli-container` to be a coherent "one container per submodule" boundary.
- `MISSION.md` (gitignored) — the multi-session plan that scheduled this work (Thread B3) and identifies the downstream threads that benefit (C: social re-aim lands in `workloads/social/`; D: forge spotlight is partially satisfied by structure).
