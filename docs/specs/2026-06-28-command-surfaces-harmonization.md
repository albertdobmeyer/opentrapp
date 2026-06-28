# Command surfaces — harmonization + the followable roadmap

**Date:** 2026-06-28 · **Author:** session prep · **Status:** DRAFT for owner review.
Ties together the three command-surface specs into one sequence with explicit dependencies, a shared contract, the inherited danger-gate, and a parallelization plan.

- Spec 1 — [CLI-first command surface](2026-06-28-cli-first-command-surface.md)
- Spec 2 — [Registry-native distribution](2026-06-28-registry-native-distribution.md)
- Spec 3 — [MCP adapter](2026-06-28-mcp-adapter.md)

**Roadmap status lives in [`ROADMAP.md`](../../ROADMAP.md) (§2/§5), not here** (CLAUDE.md §13: one source of truth per concern). This doc is the *plan* (design detail + sequence); ROADMAP holds the *rung status*.

---

## 1. The unifying insight: one command API, three projections (ADR-0022 §1)

All three surfaces are projections of the **same** manifest-driven command API already in `opentrapp-core`. None adds orchestration logic; none is a parallel system.

```
                         ┌──────────────────────────────────────────────┐
                         │  opentrapp-core  (GUI-free, CI WebKit-free)    │
                         │  execute::run_command · workflow_ops           │
                         │  control::submit · discovery · status · health │
                         │  ── the danger-gate ──                         │
                         │  boundary.rs · control.rs · approvals.rs ·     │
                         │  supervisor::gate_inbox_request / apply_approved│
                         └──────────────────────────────────────────────┘
                              ▲              ▲                 ▲
                    ┌─────────┘              │                 └──────────┐
            (Spec 1) CLI            (shipped) loopback GUI        (Spec 3) MCP
        opentrapp <concern> <verb>   viewer-server /api/*        opentrapp mcp (stdio)
                              │              │                 │
                              └──────── all inherit the SAME gate ────────┘
                         Spec 2 = how the binary that carries them all is distributed
```

The loopback GUI projection already exists and consumes these functions (`viewer-server/src/routes.rs`) — it is the **reference projection**. Spec 1 and Spec 3 make the CLI and MCP projections match it.

## 2. The shared contract (what keeps the three projections honest)

1. **Single engine.** Every projection calls the same `core` functions. A new concern command is added once (in a `component.yml`) and appears in **all** projections for free.
2. **Two security axes, never conflated** (Spec 1 §5.4): `boundary_impact` (ADR-0021 — carried by *both* the control verbs *and* **every manifest command**, fail-closed to `weakening`) vs `danger` (an operational UI hint). **`boundary_impact` is enforced at one chokepoint** (`execute::run_command`), a gap closed in **Phase 0** (a manifest command classified weakening is refused there, so all three projections inherit it); `danger` is not gated. *(Corrected 2026-06-28 — an earlier draft wrongly said only the control verbs carry the axis.)*
3. **One danger-gate, inherited unchanged** (§3).
4. **Parity is tested, not trusted.** The existing `orchestrator-check.sh` §6 route-parity pattern generalizes: CLI verbs, MCP tools, and viewer routes all derive from the same manifest/core surface, and each projection has a "no weakening edge" pin (Spec 1 T7, Spec 3 M1–M3).

## 3. The danger-gate invariant every surface inherits (ADR-0021)

Verified mechanics (one implementation, three consumers):
- Weakening control op (`pause`/`down`/`shutdown`) → `control::submit` → `supervisor::gate_inbox_request` **HOLDS** it in the approvals queue. Neutral (`resume`/`restart`) applies.
- The **sole** edge from held→applied is `supervisor::apply_approved`, reachable **only** from the out-of-band GUI two-tap (`/api/approve_weakening`).
- Therefore: **no agent-reachable transport (CLI, MCP, loopback API) has a call edge to a weakening writer.**

Each new surface adds *only* a thin "this projection has no weakening edge" pin; it writes **no** new gate logic. A prompt-injected external operator still cannot disarm the cage — it can only *queue* a request a human must approve out-of-band.

**Phase 0 extends this same gate to manifest *commands*** (not just control verbs). Today `Command.boundary_impact` (ADR-0021, fail-closed to weakening) is classified but **not runtime-enforced** — `execute::run_command` runs commands ungated (the GUI relies on a documented assumption). Phase 0 closes that at the chokepoint so the CLI/MCP/GUI cannot run a weakening command. This is a prerequisite of the agent-reachable CLI/MCP surfaces (decision: owner, 2026-06-28).

## 4. Dependency graph + sequencing

```
Spec 1 (CLI)  ──────────────►  Spec 3 (MCP)        # MCP reuses Spec 1's concern→component resolution
   │                                                # and is cleaner once the unified surface exists
   │ (independent)
Spec 2 (registry) ── agent-preparable parts run ANYTIME (in parallel) ──►  owner tag (gated)
```

- **Spec 1 → Spec 3:** MCP maps the same neutral surface the CLI dispatches; building the CLI first fixes the concern→component resolution and arg-mapping that MCP tools reuse. Not a hard compile dependency, but the right order (less rework).
- **Spec 2 is independent** and mostly owner-gated. Its agent-preparable parts (the readiness harness, CHANGELOG, metadata polish) can proceed in parallel with Spec 1 from day one.

### The release-sequencing decision (owner call, recommended answer included)

Should the v0.9.0 tag wait for the unified CLI (Spec 1), so the first registry release ships `opentrapp`?

- **Recommended: cut v0.9.0 now; ship the unified CLI in v0.10.0.** v0.9.0 (the de-Tauri daemon + goproxy + alpine) is *ready* and unblocks the Scorecard *Packaging* check and the opencode-pitch foundation. Holding a ready, verified release hostage to the CLI work violates CLAUDE.md §11 *"gate the claim, not the workstream."* The binary rename (Spec 1 §5.5) then lands in v0.10.0 with its own release notes.
- **Alternative:** do Spec 1 first, rename the binary, then cut a single v0.9.0 carrying the unified `opentrapp`. Cleaner first-impression naming, but delays a ready release behind net-new work. Only choose this if the owner wants to avoid shipping `opentrapp-daemon` as the public name even once.

## 5. The followable roadmap (phases)

Each phase is end-user-faithfully verified (§11) before the next. Ordered by the bar: containment correctness first, then surface, then reach.

| Phase | Work | Gate (consumption-end) | Spec |
|---|---|---|---|
| **P-A** | **Spec 2 readiness harness** (parallel, low-risk): run `cargo publish --dry-run` (R1) to find true red/green; add R2–R5 pins + CHANGELOG. | harness green; R1 exit 0 | Spec 2 §4 |
| **P-B** | **Spec 1 CLI dispatch + arg-mapper + red-first tests T1–T10** (no binary rename yet). | `cargo test --lib` T1–T10 green; on-box `opentrapp skill/social …` matches the in-perimeter path; `vault` regression green | Spec 1 §5,§7,§8 |
| **P-C** | **Spec 1 binary rename** `opentrapp-daemon`→`opentrapp` + alias + doc lockstep + orchestrator-check parity section. | parity section green; docs reconciled; installers (dry `dist plan`) name `opentrapp` | Spec 1 §5.5,§7 |
| **P-D** | **Owner cut** of the registry release (v0.9.0 *or* v0.10.0 per §4 decision): secret + tag + draft verification + post-publish BundleVerifier T0. | installers run; crate resolves; images cosign-verify; BundleVerifier T0 on a clean box | Spec 2 §7,§8 |
| **P-E** | **Spec 3 MCP** (after §9 dep-vs-hand-roll decision): crate + tool registry + M1–M9 + threat-model section. | protocol integration test; weakening-held proof; real-Claude-Code tail named | Spec 3 §7,§8,§9 |

P-A runs alongside P-B/P-C. P-D's *timing* depends on the §4 decision. P-E follows P-B/P-C.

## 6. Parallelization plan (where Sonnet agents can do zero-decision pure coding)

Per the maintainer's rule — parallel agents *only* for work that requires **zero decisions, pure coding from a frozen spec**:

**Eligible (after the relevant spec is frozen + its open decisions resolved):**
- Spec 1: the `map_cli_args` mapper (§5.3) + tests T1–T10 + the orchestrator-check parity section.
- Spec 2: the R2/R3/R4 checks + the `make release-dryrun` target.
- Spec 3 (only after §9 resolved): the tool-schema structs (1:1 from `routes.rs`) + tests M1–M9.

**NOT eligible (decisions / outward-facing / security-judgment) — keep with the lead/owner:**
- The binary rename + dist wiring (Spec 1 §5.5 — outward-facing artifact names).
- The MCP dep-vs-hand-roll choice (Spec 3 §9) and the threat-model wording.
- The release secret, tag, go/no-go, keyword/category choices, draft verification (Spec 2 §7).
- The §4 release-sequencing decision.

Each parallel chunk is gated by its red-first tests (green before *and* after, per CLAUDE.md §11 / the TDD discipline) and must clear the **full** CI set (§7), not a local subset.

## 7. Open decisions consolidated (for the owner-review pass)

1. **Release sequencing** (§4): cut v0.9.0 now vs after the unified CLI. *(Rec: cut now; CLI in v0.10.0.)*
2. **Binary rename timing** (Spec 1 §9.1): rename in this arc with a one-release alias. *(Rec: yes.)*
3. **`opentrapp skill scan <path>` no-perimeter alias** (Spec 1 §6.3). *(Rec: defer.)*
4. **MCP protocol: hand-roll vs official SDK** (Spec 3 §9). *(Rec: hand-roll minimal.)*

None blocks writing/freezing the specs; all should be confirmed before the corresponding implementation phase.

## 8. ROADMAP reconciliation (the docs-lockstep this plan implies)

When this plan is approved, update [`ROADMAP.md`](../../ROADMAP.md) in lockstep (do not duplicate status into the specs):
- §2 row "Unify the CLI as `opentrapp`" ⬜ → point at Spec 1 + Phase P-B/P-C.
- §2 add a row "Optional MCP adapter (external-operator-only)" ⬜ → Spec 3 / Phase P-E.
- §2 row "Status-streaming API" — note it stays separate (Spec 1 §6.3 non-goal).
- §5 rows already track the registry cut (P5-1 #103) → cross-link Spec 2 + Phase P-D.
- Refresh [`docs/handoff.md`](../handoff.md) "next frontier" to point at these four specs as the now-written plan.
