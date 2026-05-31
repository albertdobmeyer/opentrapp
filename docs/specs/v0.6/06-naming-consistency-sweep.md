# Naming-consistency sweep — `forge → skills` (spec A)

> Part of [OpenTrApp v0.6](00-index.md). **This is the FIRST implementation
> step** — a prerequisite that runs before any v0.6 feature work, so nothing is
> built on soon-renamed paths (same lesson as the ADR-0013 collapse: do the
> structural rename on clean ground, then build on the final names).
>
> Pairs with [ADR-0014](../../adr/0014-monorepo-modular-distribution.md) SD1.

---

## 1. Why this is its own spec, and why it goes first

SD1 resolved `forge → skills` (the module does *only* skills — every Makefile
target operates on skills — so by the "name it for what it does" rule, and for
`openagent-containment`/`-skills`/`-social` consistency, `skills` is the name).

It's mechanical but touches ~everything, exactly like the ADR-0013 monorepo
collapse. Bundling it into a feature leg would mean building Sentinel against
`workloads/forge/` paths and then moving them — everything twice. So it is its
own gated step, done first, verified green, then the feature work begins.

## 2. The rename, precisely

| Layer | From | To |
|-------|------|----|
| Workload directory | `workloads/forge/` | `workloads/skills/` |
| Container / compose service | `vault-forge` | `vault-skills` |
| Internal network (if any forge-specific) | `forge-net` | `skills-net` |
| Component id (`component.yml` `identity.id`) | `forge` | `skills` |
| Distribution / install name | `openskill-forge` (legacy) | `openagent-skills` |
| Shared volume | `forge-deliveries` | `skills-deliveries` |
| CLI wrapper (Pillar B) | — | `skills` |

**Kept (NOT renamed):** "cleanroom" stays the name of the CDR capability *inside*
the skills module (the pipeline that forges a clean skill from intent). "Forge"
survives only as a legacy reference in historical docs/ADRs (which are immutable
records — do not rewrite ADR-0003/0006/etc.).

## 3. File sweep (representative, not exhaustive)

The implementing agent runs the rename, then sweeps references. Group by kind:

**Move + rename:**
- `git mv workloads/forge workloads/skills`
- `workloads/skills/component.yml`: `identity.id: forge → skills` (+ name/role copy)

**Compose + orchestration (the live wiring):**
- `compose.yml`, `app/src-tauri/resources/perimeter.yml` — `vault-forge` →
  `vault-skills`, `forge-net` → `skills-net`, `forge-deliveries` →
  `skills-deliveries`, build context `./workloads/forge` → `./workloads/skills`
- `app/src-tauri/build.rs` — `STAGED_MANIFESTS` entry `forge` → `skills`; the
  staged-resources paths
- `app/src-tauri/src/bootstrap/mod.rs` — `SHELL_SERVICES` `vault-forge` →
  `vault-skills`
- `app/src-tauri/src/orchestrator/{perimeter,podman,discovery}.rs`,
  `lifecycle.rs`, `commands/diagnostics.rs` — `vault-forge` string literals,
  the `forge` component id, any `workloads/forge` path
- `app/src-tauri/src/orchestrator/tests.rs` — the manifest-parse test for
  `forge` (id assertion `forge` → `skills`, path)
- `config/orchestrator-workflows.yml` — `component: forge` → `skills`
- `.github/workflows/ci.yml` — build matrix `[vault-forge]=workloads/forge` →
  `[vault-skills]=workloads/skills`; the per-image loop service name

**Frontend:**
- `app/src/**` — `"forge"` component-id string literals (wizard pipeline,
  DevComponents, tauri.test.ts, etc. — the same call sites the ADR-0013 rename
  touched for the old names)

**Tests + harness:**
- `tests/orchestrator-check.sh` — the §11–§15 checks that name `forge` /
  `vault-forge` / `workloads/forge`
- `tests/dogfood/`, `tests/e2e-telegram/` — `vault-forge` references
- `workloads/skills/tests/`, `Makefile`, `tools/` — self-references if any

**Docs (live only — NOT historical ADRs/archive):**
- `README.md`, `CLAUDE.md`, `GLOSSARY.md`, `SECURITY.md`, `CONTRIBUTING.md`
- `docs/forge-spotlight.md` → rename to `docs/skills-spotlight.md`; sweep body
- `docs/perimeter-explained.md`, `docs/trifecta.md`, `docs/diagrams.md`,
  `docs/reproduce.{md,sh}` — `vault-forge` references
- the gitignored `docs/pitch-opencode.md` — `forge` → `skills` (the pitch
  references the module by name)
- `docs/specs/v0.6/` — already swept where the canon names appear; the leg spec
  is `03-cleanroom-skills.md`

**Do NOT touch:** `docs/adr/0001`..`0013` and `docs/archive/**` — immutable
historical records; they correctly say `forge`/`openskill-forge` for the time
they describe. ADR-0014 already carries the forward decision.

## 4. The version-label correction (folded in)

"v6" was shorthand; the real next release is **`v0.6.0`** (current shipped:
`v0.5.0`). The v0.6 spec docs + ADR-0014 + the ADR README have already been
swept `v6 → v0.6` and the spec dir is `docs/specs/v0.6/`. **No version-number
bump in code yet** — `package.json`/`tauri.conf.json`/`Cargo.toml` stay `0.5.0`
until the v0.6.0 release is cut (a release-time step, not part of this sweep).

## 5. Order of operations

1. `git mv workloads/forge workloads/skills` + `git mv docs/forge-spotlight.md docs/skills-spotlight.md`.
2. Sweep `vault-forge` → `vault-skills`, `forge-net` → `skills-net`,
   `forge-deliveries` → `skills-deliveries` across compose/perimeter/Rust/CI/tests.
3. Sweep the `forge` component-id (Rust + frontend string literals + workflows).
4. Sweep live docs (NOT ADRs/archive).
5. Run the full gate (§6). Fix until green. Commit as one atomic rename.
6. *Then* begin the feature legs (Sentinel on skills, modular distribution, …).

## 6. Tests / verification (the gate)

Reuse the session's established gate — all must pass before feature work:

- `cd app/src-tauri && cargo test --lib` — the `forge`→`skills` manifest test
  (`tests.rs`) and all orchestrator tests green.
- `cd app && npx tsc --noEmit && npm test -- --run` — frontend string-id swap
  clean.
- `bash tests/orchestrator-check.sh` — update the §11–§15 checks that name
  `vault-forge`/`workloads/forge`; add a check that **no live file** (excluding
  `docs/adr/`, `docs/archive/`) references `vault-forge` / `workloads/forge` /
  the `forge` component id (the "rename is complete" assertion).
- `npx playwright test --project=default` — added to the gate this session;
  the wizard/install-pipeline component-id references must still resolve.
- `podman compose config` — compose still parses with `vault-skills`.

## 7. Done-when

- `workloads/skills/` exists, `workloads/forge/` is gone; `vault-skills` is the
  container; every live reference (code, compose, CI, live docs) says `skills`;
  historical ADRs/archive untouched; all gates green; landed as one atomic
  "rename forge → skills" commit. The feature legs then build on final names.
