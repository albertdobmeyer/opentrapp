# Spec 3 — The optional MCP adapter (external-operator-only)

**Date:** 2026-06-28 · **Author:** session prep · **Status:** DRAFT for owner review (greenfield; design-before-building, as ADR-0022 requires).
**Decision frame:** [ADR-0022](../adr/0022-daemon-control-surface.md) §1 (MCP = the third projection, *"thin wrapper over the same API; same gate; detailed shape deferred"*), [ADR-0020](../adr/0020-product-identity-and-distribution.md) tenet 5 (**MCP serves the external host operator only — never the contained agent; that inversion is explicitly rejected**), [ADR-0021](../adr/0021-danger-gated-agentic-control-plane.md) (*"boundary-weakening writers have NO call edge from any agent-reachable transport (CLI, MCP, loopback API)"*).
**Roadmap home (status):** [`ROADMAP.md`](../../ROADMAP.md) §2 (new row). **Harmonized sequence:** [`2026-06-28-command-surfaces-harmonization.md`](2026-06-28-command-surfaces-harmonization.md).

> **The bar.** The MCP adapter touches the danger-gate, so it is judged first by containment (CLAUDE.md §12.3). A weakening tool call that *applies* instead of *holds* is a breach. Every tool below is classified and pinned.

---

## 1. What this is, and the one rule that shapes everything

An **optional** MCP server, `opentrapp mcp`, that lets a **host-side** AI operator (e.g. Claude Code, running *outside* the cage) drive the same daemon command API that the CLI and the web GUI drive — as structured MCP tools. It is the third projection of the one command API (ADR-0022 §1).

**The inviolable rule (ADR-0020 tenet 5 / ADR-0021):** this serves the **external operator only**. It is **never** an MCP server for the *contained* agent — exposing perimeter controls to the agent inside the cage is a security inversion, explicitly rejected. This is satisfied **by construction** (§5.1) and **pinned** (§7).

## 2. Verified current state (ground truth)

| Fact | Evidence |
|---|---|
| MCP is **greenfield** — zero code, zero deps. Every mention is prose/spec only; `Cargo.lock` has no `mcp`/`rmcp`/`modelcontextprotocol` entry. | repo-wide grep; `Cargo.lock` |
| The command API the adapter wraps already exists as the viewer-server's **22 `/api/<cmd>` routes**, each mirroring a pure `opentrapp-core` function (no duplicated logic). | `crates/viewer-server/src/routes.rs:63–102` |
| **Neutral** ops (agent-operable): the 9 reads + 3 writes (`write_config`, `run_command`, `execute_workflow`) + `list_pending_approvals` (read-only) + setup/telegram/stream helpers. | `routes.rs:63–102`; comment `routes.rs:75–76` ("none is boundary-weakening … danger-gated ops deliberately NOT mounted") |
| **The sole weakening applier** is `approve_weakening` → `supervisor::apply_approved`, callable **only** from the out-of-band approval surface (the GUI two-tap). | `routes.rs:86`; `crates/core/src/supervisor.rs:114–139` |
| Weakening control ops are **never mounted as routes**; they are queued via `control::submit` and **HELD** by `gate_inbox_request`. | `routes.rs:75–76`; `supervisor.rs:228–238` |
| `boundary_impact` classification: `Pause`/`Shutdown` = weakening (held); `Resume`/`Restart` = neutral. `agent_operable() == matches!(Neutral)`. | `crates/core/src/control.rs:59–65`; `crates/core/src/boundary.rs` |
| The contained agent runs inside `vault-agent` with no host access; a **host process** (the MCP server) is unreachable from inside the cage. | the perimeter model, `compose.yml` (internal networks); CLAUDE.md §1 |
| Official Rust MCP SDK exists (`modelcontextprotocol/rust-sdk` / `rmcp`); **not** in this tree. MCP is JSON-RPC 2.0 over stdio with a small method set (`initialize`, `tools/list`, `tools/call`). | recon; MCP spec |

## 3. The refactor / rebuild / new-code verdict

| Area | Verdict | Why |
|---|---|---|
| The command API (`core::execute`, `workflow_ops`, `control`, `discovery`, `status`, `health`, `config_ops`) | **KEEP (zero change)** | The adapter calls these **directly in-process** (it is a Rust binary linking `core`), exactly as the CLI and viewer-server do. No new orchestration. |
| The danger-gate (`boundary.rs`/`control.rs`/`approvals.rs`/`supervisor.rs`) | **KEEP (zero change)** | Inherited unchanged; the adapter routes weakening through `control::submit` (queue+hold) and **never** links `apply_approved`. |
| A new `opentrapp-mcp` crate: a JSON-RPC-over-stdio server + a tool registry that maps the **neutral** command surface to MCP tools | **NEW** | Greenfield. The only new code. Kept thin and auditable. |
| MCP protocol layer | **NEW — evaluate dep vs hand-roll (§9, the one real decision)** | Lean security tool ⇒ minimize Tier-1 deps. |

## 4. Design

### 4.1 Transport: stdio JSON-RPC subprocess (the standard MCP model)

The host operator's MCP client (Claude Code) spawns `opentrapp mcp` as a **subprocess and speaks JSON-RPC 2.0 over stdin/stdout**. This is the canonical MCP transport and it deliberately **opens no network socket** — sidestepping the loopback-server attack surface entirely. (The adapter is *not* an HTTP client of the viewer-server; it links `core` and calls it directly. "Same command API" = the same core functions, of which the routes are one projection.)

Registration is the standard MCP client config (e.g. a `.mcp.json` entry: `command: "opentrapp", args: ["mcp"]`).

### 4.2 Tool surface = the neutral command API, 1:1

Each tool's input/output schema mirrors the corresponding route's request/response structs (`routes.rs`). Map the **neutral** surface:

- **Reads (safe):** `list_components`, `get_component`, `get_status`, `list_workflows`, `read_config`, `check_prerequisites`, `load_options`, `run_health_probe`.
- **Writes (neutral):** `write_config`, `run_command`, `execute_workflow`.
- **Approval *visibility* (read-only):** `list_pending_approvals` — so the operator can *see* what is held and tell the human to approve out-of-band.
- **Vault lifecycle:** `vault_status`, `vault_verify`, and neutral `vault_resume`/`vault_restart` (→ `control::submit`, which the supervisor applies because they are neutral).
- **Vault weakening (`vault_pause`/`vault_down`):** exposed **only** as *queue-and-hold* — the tool calls `control::submit` and returns `{ held: true, id, message: "approve out-of-band via the GUI two-tap" }`. It **never** applies. Mirrors the CLI's `submit_weakening` exactly.

**Never exposed:**
- `approve_weakening` / `apply_approved` — the sole weakening applier; out-of-band human GUI only. The adapter must not even *link* it.
- Any tool for the **contained agent** — there is none; the server is host-side (§5.1).

### 4.3 Danger-gate inheritance (free, by routing)

The adapter writes **no** new gate. It inherits the existing one because:
- neutral commands call `execute::run_command`, which **after Phase 0** ([harmonization §3](2026-06-28-command-surfaces-harmonization.md)) refuses any command classified `boundary_impact: weakening` at the chokepoint — so a weakening command tool is refused there, not by adapter code; and
- weakening *control* ops go through `control::submit` → the supervisor's `gate_inbox_request` **holds** them; the adapter never calls `apply_approved`.

So the ADR-0021 property — *no weakening edge from an agent-reachable transport* — holds for MCP identically to the CLI and the loopback API. One gate, three projections.

## 5. The security argument (must be airtight)

### 5.1 External-operator-only is by construction

The MCP server is a **host process** spawned by the host operator. The contained agent lives inside `vault-agent` on `internal: true` networks with no host filesystem/process access (CLAUDE.md §9 network isolation). It cannot spawn, see, or speak to a host stdio subprocess. Therefore "external operator only" is not a policy toggle that could be misconfigured — it is a structural property of where the process runs. The spec documents this in `docs/threat-model.md` (a docs-lockstep update) and pins the structural facts (§7).

### 5.2 No new attack surface

stdio-only ⇒ no listening port, no bearer/Origin/CSRF surface to get wrong (unlike a server transport). The adapter's authority is exactly the host user's authority (it runs as them), same as the CLI. It adds **no** capability the host user/operator does not already have via the CLI — it only reshapes it as MCP tools.

## 6. Out of scope (YAGNI / explicit non-goals)

1. **MCP `resources`, `prompts`, `sampling`.** Tools only. Add later if a concrete need appears.
2. **Any agent-facing MCP / in-cage MCP.** Permanently rejected (ADR-0020 tenet 5).
3. **Exposing `approve_weakening`.** Permanently out-of-band/human-only.
4. **A network/SSE MCP transport.** stdio only, for the no-new-surface argument (§5.2). Revisit only with a security ADR.
5. **Telegram helper tools** (`telegram_*`) — operator-irrelevant; omit from the tool set unless asked.

## 7. Red-first test goalposts (concrete; containment-first)

Rust tests in the new `opentrapp-mcp` crate (`cargo test --lib`, CI gate `check-rust`). Red now (crate absent), green when built.

| # | Test name | Asserts | Category |
|---|---|---|---|
| M1 | `tool_registry_excludes_approve_weakening` | no tool maps to `approve_weakening`/`apply_approved`; the crate does not even reference `apply_approved`. | **security pin** |
| M2 | `weakening_tool_only_queues_and_reports_held` | `vault_pause`/`vault_down` call `control::submit` + return `{held:true,id}`; perimeter **not** paused; `apply_approved` never called. | **security pin** |
| M3 | `tools_list_mirrors_the_neutral_surface` | the exposed tool set == the neutral command surface (parity with the viewer-server neutral routes, minus `approve_weakening`); a new weakening op cannot silently appear as a tool. | **security pin / parity** |
| M4 | `neutral_read_tool_returns_data` | `get_status`/`list_components` tool calls return the same shape as the core functions. | functional |
| M5 | `run_command_tool_passes_args_verbatim` | args reach `execute::run_command` unmodified (core escapes; the adapter must not). | functional |
| M6 | `initialize_handshake_advertises_tools_capability` | JSON-RPC `initialize` → `serverInfo` + `capabilities.tools`. | protocol |
| M7 | `tools_call_unknown_tool_is_a_jsonrpc_error` | unknown method/tool → a well-formed JSON-RPC error, not a panic. | protocol/robustness |
| M8 | `server_opens_no_network_socket` | the adapter binds/listens on nothing (stdio only) — the §5.2 no-surface property. | **security pin** |
| M9 | `list_pending_approvals_tool_is_read_only` | the approvals tool returns `{id,verb}` and cannot apply. | **security pin** |

**Threat-model docs goalpost:** `docs/threat-model.md` gains a section stating the MCP adapter is host-side / external-operator-only / inherits the gate, cross-referenced from ADR-0022. (Docs-lockstep, CLAUDE.md §13.)

## 8. Verification at the consumption end (the bar, §11)

1. **Protocol integration test** (a Rust integration test or a `bash`+`jq` harness): spawn `opentrapp mcp`, pipe JSON-RPC `initialize` → `tools/list` → `tools/call get_status`; assert correct responses.
2. **The weakening proof:** drive `tools/call vault_pause`; assert it returns *held* and that the perimeter is still up + the request sits in the approvals queue (cross-checks the existing `approvals`/`supervisor` tests) — and that the **GUI two-tap remains the only applier**.
3. **End-user-faithful tail (owner/manual):** register `opentrapp mcp` in a real Claude Code `.mcp.json`, call a read tool and a weakening tool from the actual host operator; confirm the read works and the weakening is held pending the GUI. Needs Claude Code ⇒ **named as the maintainer/owner verification tail, not claimed here** (§11 unverifiable-is-not-verified).

## 9. The one real open decision (resolve before building)

**MCP protocol layer: official Rust SDK (`rmcp`) vs a minimal hand-rolled JSON-RPC-over-stdio.**

- **Hand-roll (recommended):** MCP is JSON-RPC 2.0 with a tiny method set (`initialize`, `tools/list`, `tools/call`). A ~one-file `serde_json` stdio loop is small, fully auditable, and adds **zero** new dependency to a security tool whose bar is *fewer deps on the trusted host* (CLAUDE.md §12.3). The tool surface is small and stable.
- **Official SDK:** less protocol code to own and tracks spec changes, but pulls an early-ecosystem dependency tree into the Tier-1 host surface (must be hash-pinned + audited; note this is **Tier-1 trusted-host** code, so pinning is appropriate — unlike Tier-3 contained-agent deps, which we never pin).

**Recommendation:** hand-roll the minimal server, given the small/stable surface and the security-tool low-dependency bar. Confirm with the owner before building (this is the only decision that materially changes the implementation).

## 10. Parallelizable (zero-decision) chunks — only AFTER §9 is resolved

- The tool-schema structs (1:1 from `routes.rs` request/response shapes) — pure coding.
- M1–M9 red-first tests — names + assertions specified above.
- The neutral-surface parity table generator (M3) — mechanical from `routes.rs`.

Stays with the lead/owner: §9 (dep vs hand-roll), the threat-model section wording, the real-Claude-Code verification.
