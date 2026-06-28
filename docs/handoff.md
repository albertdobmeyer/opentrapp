# Handoff (session-state)

**Last updated: 2026-06-27.** Canonical roadmap: [`ROADMAP.md`](../ROADMAP.md). Operating bar: [`CLAUDE.md`](../CLAUDE.md) §12. This doc is *only* "where we stopped + the immediate next steps" — it does not restate status that lives in `ROADMAP.md` or the ADRs.

## Where we are (one paragraph)

Two multi-session arcs are **done on `main`**: the **lean-down campaign** (de-Tauri + goproxy + alpine; ROADMAP Rung 2) and **bar-c, the danger-gated control plane** ([ADR-0021](adr/0021-danger-gated-agentic-control-plane.md)). The product is the lean **headless `opentrapp-daemon`** that owns the perimeter, with an **optional on-demand browser viewer** as the GUI projection. The Rung-1 boundary self-test (cold == resumed) is **green through the product daemon** ([product-path T0, 2026-06-26](../ROADMAP.md)). This session also ran a **documentation-consolidation sweep** ([CLAUDE.md](../CLAUDE.md) §13): reconciled stale facts (counts → 115, ADR statuses 0009/0022/0023, deleted Tauri paths), marked the 19 GTK3/Tauri advisories **resolved-by-removal** in [`known-advisories.md`](known-advisories.md), retired the dead mitmproxy watch-items, and bannered the whitepaper. What is verified vs gated is in the ROADMAP rungs — not repeated here.

## The next frontier — registry / MCP / CLI-first command surfaces

The next session continues toward the **north star** ([ADR-0020](adr/0020-product-identity-and-distribution.md)): *a registry-installable CLI/daemon orchestrator + signed images, with the GUI and an optional MCP adapter as thin projections of one manifest-driven daemon — agent-operable, danger-gated.* The control-surface architecture is [ADR-0022](adr/0022-daemon-control-surface.md) §1: **one command surface, three projections** (CLI / loopback GUI / optional MCP), each inheriting the danger-gate unchanged.

### What EXISTS today (the foundation to build on)

- **Daemon CLI** — `opentrapp-daemon vault <verb>`: `up | down | status | verify | pause | resume | restart` (`crates/daemon/src/main.rs` → `dispatch_vault` / `print_vault_help`). The weakening verbs (`down`, `pause`) are **HELD for out-of-band approval** (ADR-0021), not applied from the control channel.
- **Loopback GUI projection** — `viewer-server` (127.0.0.1-only + Host/Origin allowlist + 256-bit bearer + nonce→HttpOnly-cookie; [ADR-0022](adr/0022-daemon-control-surface.md) §3). Routes in `crates/viewer-server/src/routes.rs`; frontend client in `app/src/lib/tauri.ts` (dual-mode: Tauri IPC / `fetch /api/<cmd>`).
- **Danger-gate mechanics** — `crates/core/src/boundary.rs` (`BoundaryImpact` neutral|weakening, fail-closed default), `control.rs` (verb→impact), `approvals.rs` (pending queue), `supervisor.rs` (`gate_inbox_request` / `apply_approved`). Approval surface: `/api/list_pending_approvals` + `/api/approve_weakening` + `WeakeningApprovalsCard`.
- **Registry/release lane (BUILT, not cut)** — cargo-dist (`dist-workspace.toml` + SHA-pinned `.github/workflows/release.yml`) + `cargo publish` of `opentrapp-core` ([ADR-0023](adr/0023-distribution-and-packaging.md)). The actual tag cut is **owner-gated** (see the maintainer tail).

### What's NEXT (the work to pick up)

1. **CLI-first command surface beyond `vault`** ([ADR-0020](adr/0020-product-identity-and-distribution.md) tenet 2 + [ADR-0024](adr/0024-product-structure-three-concerns.md) three concerns). Make each concern independently CLI-operable under one binary: the **Skill Firewall already ships standalone** (`workloads/skills/skill scan …` + the GitHub Action), so the model exists — extend it to a unified `opentrapp <concern> <verb>` surface (`opentrapp vault …`, `opentrapp skill …`, social on-demand). The bare `opentrapp` command arrives with the GUI demotion ([ADR-0022](adr/0022-daemon-control-surface.md) Phase 3); today it is `opentrapp-daemon vault`.
2. **Registry-native distribution** ([ADR-0023](adr/0023-distribution-and-packaging.md)). Lane is built; the cut is the owner tagging v0.9.0 → crates.io `opentrapp-core` + cargo-dist installers, which **flips the Scorecard Packaging check** legitimately. Needs the owner tag + `CARGO_REGISTRY_TOKEN`.
3. **Optional MCP adapter** ([ADR-0022](adr/0022-daemon-control-surface.md) §1; shape deferred). A thin wrapper over the **same** command API, **for the external host operator only** — **never** an MCP server for the *contained* agent (a security inversion, explicitly rejected in [ADR-0020](adr/0020-product-identity-and-distribution.md)). Same danger-gate. Design it before building.

### The invariants the new surfaces must never break

- **Danger-gate ([ADR-0021](adr/0021-danger-gated-agentic-control-plane.md)).** Any new CLI verb or MCP tool that weakens the boundary (`boundary_impact: weakening`) MUST route through the approval queue (`supervisor::gate_inbox_request` → human `apply_approved`), **never apply directly** — there is no agent-call edge to a weakening writer, regardless of who asks.
- **Two agents, one rule ([ADR-0020](adr/0020-product-identity-and-distribution.md) tenet 4).** The external operator (Claude Code) is a trusted *operator*; the contained agent never controls its own cage. The MCP adapter serves the external operator only.
- **Verify at the consumption end ([CLAUDE.md §11](../CLAUDE.md)).** A new surface is "done" only when exercised through the **product binary** (not `make`/`podman-compose`), with the cold == resumed boundary self-test re-passing. Local green ≠ CI green ([§7](../CLAUDE.md)).

## Maintainer-gated tail (human decisions, not building)

- **v0.9.0 cut** (#103). Owner go/no-go on the tag. `git tag v0.9.0` fires `release.yml` (host binaries, as a **draft**) + `build-images` (signed perimeter images onto the draft) + `publish-crate.yml` (crates.io; needs `CARGO_REGISTRY_TOKEN`); the human verifies the draft and publishes. **Do not push a `v*` tag without the owner's go/no-go** (outward-facing). The post-publish BundleVerifier digest-staging T0 is then exercised on a clean box.
- **Win/macOS browser-runtime** (#104). The de-Tauri browser model is Linux-proven; Win/macOS are portable-by-construction but runtime-unverified — owner hardware. Runbook: [`perimeter-test-handoff.md`](perimeter-test-handoff.md) (its `.msi` install path is now historical; the purpose stands).
- **Co-maintainer Scorecard** #43 (Code-Review) + #1 (Branch-Protection) (#78). A second human, not a code change — honestly OPEN, never dismissed.

## Running the perimeter / T0 on this box (still true)

- The 7.2 GB laptop runs the full perimeter + T0 **when cleaned** of heavy apps (Cursor/Brave): ~3.6 GB free, no swap-storm. Images are pre-built (`podman images`).
- podman ops need `dangerouslyDisableSandbox`; local builds need fully-qualified image names (`docker.io/library/…`). Stop any running daemon (it holds a `RunGuard`) before re-running.
- `vault down` / `vault pause` are **HELD** (boundary-weakening; ADR-0021) — they no longer stop the daemon from the control channel. To tear down: SIGTERM the `vault up` pid. To exercise a pause→resume cycle: use the idle knob `OPENTRAPP_IDLE_TIMEOUT_MS`, not `vault pause`.
- **Credentials:** dev keys were rotated 2026-06-22; never run `podman-compose` *verbose* with real keys (it echoes them). The boundary self-test needs only a **placeholder** key (no real credential). Move the real `~/.opentrapp/.env` aside UNREAD and restore after box tests — never handle a real key.

## Remaining doc debt (small, named — not glossed)

- **OpenSSF badge answers** ([`openssf-badge-answers.md`](openssf-badge-answers.md), [`openssf-best-practices-application.md`](openssf-best-practices-application.md)) still describe OpenTrApp as a "desktop application" in the Description + interface fields. These are owner-facing form text submitted to bestpractices.dev — they want a de-Tauri pass before the next badge submission (the *version* line was fixed this session; the framing was left for owner review).

## The bar ([CLAUDE.md §12](../CLAUDE.md))

End-user-faithful tests only (the product daemon, not dev scaffolding). Root-cause fixes, no glossing or handwaving. Protect the user from agent dangers first. Substance before visibility.
