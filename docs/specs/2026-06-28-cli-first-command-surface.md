# Spec 1 — The unified CLI-first command surface (`opentrapp <concern> <verb>`)

**Date:** 2026-06-28 · **Author:** session prep · **Status:** DRAFT for owner review (implementation not started).
**Decision frame:** [ADR-0020](../adr/0020-product-identity-and-distribution.md) tenet 2 (CLI-first), [ADR-0022](../adr/0022-daemon-control-surface.md) §1 (one command API, three projections), [ADR-0024](../adr/0024-product-structure-three-concerns.md) (three CLI-operable concerns), [ADR-0021](../adr/0021-danger-gated-agentic-control-plane.md) (the danger-gate every surface inherits).
**Roadmap home (status, not duplicated here):** [`ROADMAP.md`](../../ROADMAP.md) §2 — "Unify the CLI as `opentrapp`" (currently ⬜ Phase 3). **Harmonized sequence:** [`2026-06-28-command-surfaces-harmonization.md`](2026-06-28-command-surfaces-harmonization.md).

> **The bar ([CLAUDE.md §11–§12](../../CLAUDE.md)).** This is a security application. A surface is "done" only when exercised through the **product binary** (not `make`/`podman-compose`), with the cold == resumed boundary self-test still green. Local green ≠ CI green (§7). Every claim below is grounded in verified code (file:line); nothing is assumed.

---

## 1. The goal, in one sentence

Make all three concerns operable from **one binary** — `opentrapp vault …`, `opentrapp skill …`, `opentrapp social …` — where the binary is a **thin projection over the manifest-driven command API that already exists in `opentrapp-core`**, adding *no* new orchestration logic and *no* parallel system.

## 2. Why this is refinement, not a rebuild (the code already anticipates it)

The daemon binary's own source states the destination:

- `crates/daemon/src/main.rs:29–33` — *"The bare `opentrapp` command + GUI demotion lands in Phase 3 (de-Tauri); until then the operator CLI is `opentrapp-daemon vault <verb>`."*
- `crates/daemon/src/main.rs:165–168` — *"This is the CLI-first projection (ADR-0020/0024) over the same `opentrapp-core` calls the engine flags use — **no new perimeter logic**."*
- [`ROADMAP.md`](../../ROADMAP.md):72 already tracks *"Unify the CLI as `opentrapp` (retire `opentrapp-daemon`)"* as Phase 3.

We are executing a planned, partially-built direction — the definition of "refine, don't rebuild."

## 3. Verified current state (ground truth)

| Fact | Evidence |
|---|---|
| The daemon binary is the CLI entry; arg-parsing is a **hand-rolled `match`** on `std::env::args()` (no clap). | `crates/daemon/src/main.rs:26–60` |
| `vault` is the **only** concern subcommand today: `up·down·status·verify·pause·resume·restart`. Plus `configure`, control verbs, and `--*` flags. | `crates/daemon/src/main.rs:34–59`, `169–208` |
| The generic, manifest-driven command runner already exists and is **GUI-free**: `execute::run_command(components, runtime_data_dir, component_id, command_id, args) -> RunOutcome`. | `crates/core/src/execute.rs:39–48` |
| The generic workflow runner exists: `workflow_ops::execute_workflow(components, component_id, workflow_id, inputs)`. | `crates/core/src/workflow_ops.rs:33–60` |
| Discovery exists: `discovery::discover_components(monorepo_root) -> Vec<DiscoveredComponent>`. | `crates/core/src/orchestrator/discovery.rs:33–79` |
| Arg interpolation is **already injection-safe** (single-quote escaping), pinned by a test. The CLI must pass values **verbatim** and let core escape — never pre-mangle. | `crates/core/src/orchestrator/runner.rs:26–36`; test `podman_exec_keeps_escaped_arg_as_single_argv` |
| `core` is CI-asserted free of tauri/wry/webkit — any crate (a CLI) may link it. | `crates/core/Cargo.toml:1–27` |
| The `viewer-server` GUI already consumes exactly these core functions — proving they are caller-agnostic. | `crates/viewer-server/src/routes.rs` (`run_command`, `execute_workflow`) |
| **Concern → target mapping** (now exact): `skill` → component `skills` (`role: toolchain`); `social` → component `social` (`role: network`); `vault` → the **perimeter control path** (`control::submit`), *not* a manifest component. | `workloads/skills/component.yml:3–11`; `workloads/social/component.yml:3–11`; `crates/core/src/control.rs` |
| The danger-gate is enforced **downstream** in `supervisor::gate_inbox_request` (weakening → approval queue; neutral → apply), *not* in the CLI. The CLI only queues via `control::submit`. | `crates/core/src/supervisor.rs:228–238`, `264–275` |
| **Gap:** there is **no test** asserting the CLI's dispatch table and help text stay in sync (verbs implemented == verbs documented == verbs reachable). | confirmed absent in `crates/daemon/src/main.rs` tests |

## 4. The refactor / rebuild / new-code verdict (per area)

| Area | Verdict | Why |
|---|---|---|
| Perimeter orchestration, command running, workflow running, discovery, arg-escaping | **KEEP (zero change)** | All already in `core`, GUI-free, pinned by tests. The whole point is to project them. |
| `vault` dispatch (control path + danger-gate) | **KEEP (zero change)** | `dispatch_vault` + `submit_weakening` already correct; gate proven this session. |
| The bash skill/social tools + `component.yml` command declarations | **KEEP (zero change)** | They are the single source of each concern's commands (e.g. `make scan SKILL=${skill}`). The CLI projects them. |
| The standalone `skill` bash CLI + GitHub Action (the no-perimeter wedge, ADR-0024) | **KEEP (zero change)** | A *distribution form*, not a duplicate. See §6.3. |
| A `skill`/`social` dispatch arm + a **generic manifest-driven arg-mapper** + generic `--help`/`list` | **NEW (small, generic)** | The only new code. Driven entirely by the manifest; no per-command logic. ~1 module + tests. |
| Binary name `opentrapp-daemon` → `opentrapp` (+ back-compat alias) | **REFACTOR (mechanical, outward-facing)** | ROADMAP Phase-3 item; touches `Cargo.toml` + `dist-workspace.toml` + docs. Owner-aware (artifact-name change). |

**No parallel system is created:** there is exactly one orchestration engine (`core`), one set of concern commands (the manifests + bash tools), one danger-gate (the supervisor). The CLI is a fourth caller of the same engine the GUI already calls.

## 5. Design

### 5.1 Surface

```
opentrapp vault   <up|down|status|verify|pause|resume|restart>   # control path (unchanged)
opentrapp skill   <command-id|workflow-id> [--<arg> <value> …]   # → component "skills"
opentrapp social  <command-id|workflow-id> [--<arg> <value> …]   # → component "social"
opentrapp configure                                              # on-demand web viewer (unchanged)
opentrapp list                                                   # concerns + their declared commands
opentrapp <concern> --help                                       # commands/args rendered FROM the manifest
opentrapp --help | --status | --selftest | --boundary-selftest   # engine flags (unchanged)
(no args)                                                        # own + supervise the perimeter (unchanged)
```

### 5.2 Dispatch (extends the existing hand-rolled `main`)

Add two arms alongside the existing `vault`/`configure` checks in `main.rs:34–41`:

```
if args[0] == "skill"  -> dispatch_concern("skills",  args[1..])
if args[0] == "social" -> dispatch_concern("social", args[1..])
```

`dispatch_concern(component_id, rest)`:
1. `discover_components(monorepo_root)` (resolve root the same way the daemon already does).
2. Find the component by `component_id`; if absent → exit 2 with a clear message.
3. `rest[0]` is the verb. Resolve it against the manifest:
   - matches a **command id** → build args (§5.3) → `execute::run_command(...)`.
   - matches a **workflow id** → build inputs (§5.3) → `workflow_ops::execute_workflow(...)`.
   - `--help`/none → render the manifest's commands+args (generic).
   - else → exit 2, list available verbs (from the manifest).
4. Print `stdout`/`stderr`; map the command's `exit_code` to the process exit code.

### 5.3 The generic arg-mapper (the one genuinely new, testable unit)

A pure function `map_cli_args(command: &Command, tokens: &[String]) -> Result<HashMap<String,String>, ArgError>`:

- Accepts `--<arg-id> <value>` pairs for any arg the command declares (`Command.args[].id`).
- Ergonomic positional form: if the command has exactly one **required** arg, a single bare token binds to it (`opentrapp skill scan myskill` == `--skill myskill`).
- Applies declared **defaults** for omitted optional args (e.g. social `feed-scan` `count` default `50`).
- **Rejects** (exit 2) a missing required arg or an unknown `--flag`, naming it.
- Passes values **verbatim** into the map — `core::runner` does the shell-escaping (§3). The mapper must never quote/escape (double-escaping is a bug; pinned by a test).

This function is the meat of the work and is **100% manifest-driven** — adding a new command to any `component.yml` needs **zero** CLI code.

### 5.4 Two security axes — and the boundary-impact gap this arc closes

> **Correction (2026-06-28, verified in code).** An earlier draft of this section claimed concern commands "aren't boundary-impacting / a different axis." That was **wrong**. A manifest `Command` carries the ADR-0021 axis directly.

A manifest `Command` carries **two** independent security axes (`manifest.rs:119,124`):
- **`danger`** (`safe`/`caution`/`dangerous`) — an operational-disruption UI hint. *Not* a perimeter boundary. `--help` surfaces it; the CLI does not gate on it (an interactive confirm would break non-interactive agent operation — §6.1).
- **`boundary_impact`** (`neutral`/`weakening`) — the **ADR-0021** axis: does the op reduce the perimeter's protection? **Fail-closed**: `BoundaryImpact::default() == Weakening` (`boundary.rs:30`), so a command that omits it is treated as weakening.

**The gap (verified 2026-06-28):** every skill/social command currently *omits* `boundary_impact` → all classify as `Weakening` by the fail-closed default, **and no runtime code consumes `Command.boundary_impact`** — `execute::run_command` runs commands ungated; the shipped GUI relies on a documented "these are neutral in fact" assumption (`routes.rs:75`), not enforcement. The only enforced ADR-0021 gate today is the control channel (vault verbs, `supervisor::gate_inbox_request`).

**The decision (owner, 2026-06-28): close it properly — Phase 0 of the [implementation plan](2026-06-28-command-surfaces-implementation-plan.md), a prerequisite of this CLI.** Audit + honestly classify every concern command; enforce `boundary_impact` at the **single chokepoint** `execute::run_command` (fail-closed: a weakening/unclassified command is **refused**, never run), so **all** projections — CLI, MCP, *and* the existing GUI — inherit one gate. The CLI then needs *no* boundary code of its own: it calls `run_command`, which is now the gate.

vault control verbs are unchanged: `pause`/`down` stay routed through `submit_weakening` (queue + HELD); `resume`/`restart` neutral. The invariant for the new surface: **no concern verb can run a weakening command** — enforced at the chokepoint, pinned by §7 T7 + the Phase-0 tests.

### 5.5 Binary rename (`opentrapp-daemon` → `opentrapp`)

- `crates/daemon/Cargo.toml:16–18`: change `[[bin]] name` to `opentrapp` (path unchanged). Keep `[package.metadata.dist] dist = true`.
- Back-compat: ship a thin `opentrapp-daemon` alias for one release (a second `[[bin]]` that `exec`s `opentrapp`, or a documented symlink) so existing invocations + `docs/headless.md` keep working through the transition. Remove the alias in the release after.
- `dist-workspace.toml`: no target change needed (it ships whatever bins are `dist = true`); the installer artifact name changes `opentrapp-daemon` → `opentrapp` — **outward-facing**, so it lands with Spec 2's release notes, not silently.
- Update `print_*help` strings, `docs/headless.md`, README, and any `opentrapp-daemon vault` references in lockstep (CLAUDE.md §13 doc-lockstep rule).

## 6. Out of scope (YAGNI / explicit non-goals)

1. **Per-command `danger:` gating / interactive confirmation.** Not a boundary control; would break non-interactive agent operation. Deferred unless a concrete need appears.
2. **Adopting `clap`.** The surface is `<concern> <verb> --<arg> <value>` where the per-command args are *manifest-driven* (not statically known), so a static clap-derive is a poor fit; the hand-rolled dispatch + the generic arg-mapper is leaner and matches the existing style. Revisit only if the static surface grows.
3. **A status-streaming API** (ROADMAP §2:71) — separate item; the CLI keeps reading markers + stderr.
4. **OS-autostart launching the daemon** (ROADMAP §2:73) — separate item.
5. **Folding the standalone `skill` wedge into Rust** — it stays bash (§6.3).

### 6.3 `opentrapp skill` vs the standalone `skill` — two distribution forms, not a duplicate

ADR-0024 makes Skill *both* an in-perimeter concern *and* a standalone wedge ("ships standalone, no perimeter required — the adoption wedge"). Accordingly:

- **`opentrapp skill <verb>`** projects the **in-perimeter** manifest commands (`skills/component.yml`), which run inside `vault-skills` (e.g. `vet-skill` = lint→scan→verify on a named skill). This is the unified, perimeter-context surface.
- **`skill scan <path>`** (the standalone bash CLI + the GitHub Action) remains the **no-perimeter** quick path on an arbitrary path. Same underlying scanner logic (`workloads/skills/tools/skill-*.sh`), different context. **Unchanged.**

These are not a parallel system: one scanner implementation, two ADR-0024-sanctioned entrypoints. The spec deliberately does **not** make `opentrapp skill scan` shell to the bash wedge — keeping the unified surface uniformly manifest-driven (one mechanism for skill *and* social). If a no-perimeter `opentrapp skill scan <path>` is later wanted, it is a thin alias that `exec`s the existing `skill` script — still no new logic. Flagged as an open ergonomic for owner review (§9).

## 7. Red-first test goalposts (concrete; define "done")

All Rust tests drop into `crates/daemon/src/main.rs` `#[cfg(test)]` (matching the existing `resolve_viewer_bin_*` style) or a new `crates/daemon/src/cli.rs` module if the dispatch is extracted there. They are **red now** (the code doesn't exist) and green when implemented. Run via `cd app/src-tauri && cargo test --lib` (CI gate `check-rust`).

| # | Test name | Asserts | Red because |
|---|---|---|---|
| T1 | `skill_verb_resolves_to_run_command` | `dispatch_concern("skills", ["scan","--skill","X"])` resolves command `scan` + args `{skill:"X"}` and calls `execute::run_command` for component `skills`. | dispatch arm absent |
| T2 | `unknown_concern_verb_exits_2_and_lists_available` | `["skills",["bogus"]]` → exit 2, output names real command ids from the manifest. | dispatch arm absent |
| T3 | `missing_required_arg_is_rejected` | `map_cli_args(scan, [])` → `Err(MissingRequired("skill"))`; process exit 2. | mapper absent |
| T4 | `optional_arg_default_is_applied` | `map_cli_args(feed-scan, [])` yields `{count:"50"}` from the manifest default. | mapper absent |
| T5 | `single_required_arg_accepts_a_positional` | `map_cli_args(scan, ["myskill"])` == `{skill:"myskill"}`. | mapper absent |
| T6 | `arg_values_pass_through_verbatim` | a value `a'; rm -rf / #` reaches the map **unmodified** (core escapes, the CLI must not). | mapper absent |
| T7 | `weakening_command_is_refused_through_the_cli` (**security pin**) | a command classified `boundary_impact: weakening` invoked via `opentrapp skill <verb>` is **refused** by the Phase-0 chokepoint gate in `execute::run_command` (never executed); and the `skill`/`social` dispatch path has no edge to `control::submit`. | inherits ADR-0021 via Phase 0; must never regress |
| T8 | `vault_dispatch_table_is_unchanged` (regression) | `up·down·status·verify·pause·resume·restart` all still present; `down`/`pause` still HELD via `submit_weakening`. | guards the refactor |
| T9 | `cli_help_lists_manifest_command_ids` | `opentrapp skill --help` output contains the manifest's command ids (generic projection). | help-render absent |
| T10 | `unknown_arg_flag_is_rejected` | `map_cli_args(scan, ["--bogus","x"])` → `Err(UnknownArg("bogus"))`. | mapper absent |

**Shell-level parity goalpost (closes the §3 gap).** Add a section to `tests/orchestrator-check.sh` (mirroring its §6 route-parity pattern) asserting: every concern the CLI dispatches (`vault`/`skill`/`social`) is documented in `print_help`, and `skill`/`social` each resolve to a real `component.yml` (`identity.id ∈ {skills, social}`). CI gate `check-orchestration`.

## 8. Verification at the consumption end (the bar, §11)

1. **Build the renamed binary**, then on the 7.2 GB box (perimeter up):
   - `opentrapp skill scan --skill <name>` → same exit code + report as the in-perimeter `vet`/`scan` path. `opentrapp skill vet-skill --skill <name>` runs the lint→scan→verify workflow.
   - `opentrapp social level-status` and one read command return correct output.
   - `opentrapp list` and `opentrapp skill --help` render the manifest commands.
2. **Regression:** `opentrapp vault up` → `opentrapp vault verify` still `pass=7 fail=0` cold; `pause`/`down` still print HELD and do **not** stop the perimeter from the control channel (cold == resumed T0 unchanged — the ADR-0021 + Rung-1 invariant).
3. **CI green (full set, §7):** `cargo test --lib` (T1–T10), `orchestrator-check.sh` (new parity section), `npm run lint`, `tsc`, `integration-test.sh`. Local green ≠ CI green.
4. **Unverifiable-here is named, not claimed:** none — all of the above runs on this box.

## 9. Open decisions for owner review (do not block the spec; confirm before/at implementation)

1. **Binary rename timing.** Rename in this CLI work (so the first registry release ships `opentrapp`), or keep `opentrapp-daemon` until a later release? (Recommendation: rename here with a one-release alias; it is the tracked Phase-3 goal and the registry release is the natural moment.)
2. **`opentrapp skill scan <path>` no-perimeter alias** (§6.3) — add the thin `exec`-the-wedge alias now, or leave the standalone `skill` as the only no-perimeter path? (Recommendation: leave it; add only if a user asks.)

## 10. Parallelizable (zero-decision) implementation chunks

Once this spec is frozen, these require **pure coding, no decisions** — candidates for a parallel Sonnet agent working from the spec:
- The 10 red-first Rust tests (T1–T10) — names + assertions are fully specified above.
- The `map_cli_args` mapper — behavior fully specified in §5.3.
- The `orchestrator-check.sh` parity section — pattern specified in §7.

Decisions that stay with the lead/owner: the binary rename + dist wiring (outward-facing, §5.5/§9), and the §9 ergonomic.
