# Spec — Loopback web GUI (de-Tauri) implementation plan

**Date:** 2026-06-17 · **Status:** Ready to implement (spike-gated) · **Owner:** albertd
**Decides nothing new** — it operationalizes [ADR-0022](../adr/0022-daemon-control-surface.md)
into a buildable plan. Read ADR-0022 first; it is the *why* and the security model. This is the
*how*, the API contract, and the verification gates. Companion handoff:
[`docs/handoff-web-gui.md`](../handoff-web-gui.md). Scaffold:
[`app/src-tauri/crates/viewer-server/`](../../app/src-tauri/crates/viewer-server/) (excluded from
the workspace until the spike passes).

---

## TL;DR

Replace the Tauri/GTK3 desktop webview with an **on-demand, loopback-only (`127.0.0.1`) axum
HTTP/WS server** that serves the *existing* React app and exposes the *same* manifest-driven
command set as JSON. The React frontend and its tests are preserved; only the transport changes
(`invoke()`→`fetch`, `listen()`→one WebSocket). When the migration completes and the `opentrapp`
Tauri crate + `tauri-plugin-*` are deleted, the entire GTK3/WebKit tree leaves `Cargo.lock` and the
**21 Scorecard advisories clear genuinely** (verified: every one roots in the Tauri tree; the
`opentrapp-core`/`opentrapp-daemon` spine is already advisory-clean — see
[`docs/known-advisories.md`](../known-advisories.md)).

**This is the single riskiest part of the mission** (a network service on a security tool). The
work is therefore **spike-gated**: nothing in §"Migration" begins until the §0 spike passes its
threat-model review. Tauri stays the shipped default throughout the migration; the cutover commit
is the last step.

---

## §0 — The de-risking spike (do this FIRST; merge nothing)

Per ADR-0022, a throwaway scratch-branch spike must prove the riskiest assumptions before any
migration. **Build the minimum that exercises the security model and the bursty event channel:**

- A minimal axum server on `127.0.0.1:0` (ephemeral port) with the **full §2 security middleware**.
- Serve the **already-built** React app (`app/dist/`, produced by `npm run build`).
- Wire **3 real handlers** end-to-end: `list_components`, `get_status`, `run_command` (manifest
  render + a live read + a command execution — the manifest-driven core path).
- Wire the **`stream-line` / `stream-end`** channel over the `/api/events` WebSocket (the
  high-frequency burst path — the thing most likely to break).
- Load it in **two real browsers** (Firefox + Chromium) as the config panel.

**SUCCESS (all required):**
1. App renders and is interactive; manifest cards/commands work; a streamed command shows live lines.
2. `cargo tree` on the spike crate is **gtk/webkit/wry-free** (`grep -iE 'tauri|wry|webkit'` empty).
3. Security middleware is **live-verified** (not just unit-tested): a forged `Host:` header → 403; a
   tokenless request → 401; the token is **absent** from URL, browser history, and `Referer`; a
   DNS-rebinding-style `Host` (e.g. `Host: attacker.com`) → 403; binding is loopback-only.
4. UX judged acceptable for the *occasional config-panel* use (not a daily driver).

**KILL → re-scope viewer or accept the documented Tauri/Scorecard residual. C1 (wait for GTK4) is
NOT a fallback** (ruled out — see ADR-0022 / `de-tauri-viewer-research.md`). Kill if: the loopback
surface fails threat-model review; UX unacceptable; the WS can't carry `stream-line`; or any
webview-only load-bearing capability surfaces (the inventory found none).

The spike lives at `app/src-tauri/crates/viewer-server/` (scaffolded, `[workspace].exclude`d). Keep
it scratch; the migration re-derives production code from it.

---

## §1 — The API surface (the contract)

Every `#[tauri::command]` becomes `POST /api/<command_name>` (JSON body = the command's named args;
JSON response = the command's return value; errors → HTTP 4xx/5xx + `{error}`). Every Tauri event
becomes a named message on the single `/api/events` WebSocket. **The frontend's public `tauri.ts`
API is preserved**, so only `tauri.ts` internals change, not the ~73 components that call it.

### 1.1 Commands → endpoints

Grouped by the `opentrapp-core` module they lift into (ADR-0022 migration step 1). `bi` =
`boundary_impact` (ADR-0021): **N**eutral (agent-operable) / **W**eakening (out-of-band human
confirmation required; no agent call edge). Confirm each tag against the ADR-0021 implementation.

| Endpoint (`POST /api/…`) | Args | Returns | bi | Core target |
|---|---|---|---|---|
| `list_components` | — | `Vec<DiscoveredComponent>` | N | `orchestrator::discover` |
| `get_component` | `component_id` | `DiscoveredComponent` | N | `orchestrator::discover` |
| `set_monorepo_root` | `path` | `Vec<DiscoveredComponent>` | N | `orchestrator::discover` |
| `get_status` | `component_id` | `ComponentStatus` | N | `orchestrator::status` |
| `run_health_probe` | `component_id, probe_command, timeout_seconds` | `HealthResult` | N | `orchestrator::health` |
| `run_command` | `component_id, command_id, args` | `CommandResult` | N* | `orchestrator::runner` |
| `load_options` | `component_id, command_string, timeout_seconds` | `Vec<String>` | N | `orchestrator::runner` |
| `start_stream` | `component_id, command_id, args` | `()` (+ WS events) | N* | `orchestrator::runner` |
| `stop_stream` | `component_id, command_id` | `()` | N | `orchestrator::runner` |
| `read_config` | `component_id, config_path` | `String` | N | `orchestrator::config` |
| `write_config` | `component_id, config_path, content` | `()` | **W** | `orchestrator::config` |
| `list_workflows` | `component_id` | `Vec<Workflow>` | N | `orchestrator::workflow` |
| `execute_workflow` | `component_id, workflow_id, inputs` | `WorkflowResult` | N* | `orchestrator::workflow` |
| `get_perimeter_state` | — | `PerimeterStatus` | N | `supervisor`/`markers` |
| `get_assistant_status` | — | `AssistantStatusSnapshot` | N | `markers`/status agg |
| `restart_perimeter` | — | `()` | **W** | `control::submit(Restart)` |
| `pause_perimeter` | — | `()` | **W** | `control::submit(Pause)` |
| `resume_perimeter` | — | `()` | **W** | `control::submit(Resume)` |
| `retry_bootstrap` | — | `()` | **W** | `supervisor` |
| `check_prerequisites` | — | `PrerequisiteReport` | N | `orchestrator::prereq` |
| `init_submodules` | — | `String` | N | (host git) |
| `create_config_from_template` | `component_id, config_path, template_path` | `()` | N | `orchestrator::config` |
| `generate_diagnostic_bundle` | — | `String` | N | `diagnostics` |
| `validate_anthropic_key` | `key` | `ValidationOutcome` | N | `credentials` |
| `save_credentials` | `anthropic_key, telegram_token` | `()` | **W** | `credentials` (0600 `.env`) |
| `read_runtime_env` | — | `String` | N | `credentials` |
| `commit_activation` | — | `()` | **W** | `credentials`/`supervisor` |
| `sentinel_judge` | `request` (opaque JSON) | `Verdict` | N | `sentinel` |
| `get_sentinel_activity` | — | `SentinelActivity` | N | `sentinel` |
| `list_egress_approvals` | — | `Vec<PendingApproval>` | N | `egress` (read+judge) |
| `apply_allowlist_decision` | `host, decision` | `()` | **W** | `egress` (ADR-0016: human-only writer) |
| `derive_telegram_bot_url` | `token` | `TelegramBot` | N | `telegram` |
| `telegram_delete_webhook` | `token` | `()` | N | `telegram` |
| `telegram_poll_for_start` | `token, offset, timeout_secs` | `Option<TelegramUpdate>` | N | `telegram` |
| `telegram_send_message` | `token, chat_id, text` | `()` | N | `telegram` |
| `telegram_advance_offset` | `token, update_id` | `()` | N | `telegram` |

`N*` = neutral to *invoke*, but the underlying manifest command may itself be danger-tagged in its
`component.yml`; the existing per-command danger level still applies. **The route registry is the
source of truth for completeness** — the contract test (§6) fails if any `tauri.ts` call lacks a
route or vice-versa, so reconcile this table against the actual `#[tauri::command]` set when lifting
(the inventory counted ~33–36; do not trust this table's count — trust the test).

### 1.2 Events → `/api/events` WebSocket (named messages)

A single multiplexed WS, mirroring Tauri's `emit`/`listen`. First-frame token auth (§2). Messages:
`{event: "<name>", payload: {…}}`.

| Event | Payload | Producer |
|---|---|---|
| `perimeter-state-changed` | `PerimeterStatus` | supervisor watchdog (~30s) |
| `assistant-status-changed` | `AssistantStatusSnapshot` | status aggregator |
| `bootstrap-step-started` | `{step, total_steps, current, detail}` | bootstrap |
| `bootstrap-step-failed` | `{cause, message}` | bootstrap |
| `stream-line` | `{component_id, command_id, line, stream}` | `start_stream` (per line — bursty) |
| `stream-end` | `{component_id, command_id, exit_code}` | stream on exit |
| `sentinel-activity-changed` | `SentinelActivity` | sentinel |
| `telegram-bot-resolved` | `{url, username}` | credentials auto-activate |

`markers remain truth` (ADR-0019): the WS is a fast path; every projection re-seeds from
authoritative reads (`get_perimeter_state`, `get_status`) on connect and reconnect.

---

## §2 — Security middleware (the crux — concrete impl of ADR-0022 §3)

All of the following are required (none sufficient alone). Implement in
`viewer-server/src/security.rs` as tower middleware + a session module.

1. **Loopback bind, ephemeral port.** `TcpListener::bind(("127.0.0.1", 0))`. **Unit-assert** the
   bound `SocketAddr` is loopback before serving.
2. **`Host` allowlist** (primary anti-rebinding): accept only `127.0.0.1:<port>` / `localhost:<port>`
   → else **403**. **`Origin` allowlist** on every request incl. the WS handshake → else 403. Do
   **not** rely on browser PNA/LNA.
3. **Token (≥256-bit).** `rand`-generated 32 bytes, hex/base64url. Daemon writes `{port, token}` to a
   **0600** file under `~/.opentrapp/viewer/` (assert mode on write). Browser is opened to a bare
   loopback URL with a **single-use, short-TTL launch nonce in the URL fragment** (`#n=…` — not sent
   to the server, not in `Referer`). Bootstrap JS reads the fragment, `POST /api/session {nonce}` →
   receives the bearer, which the server sets as an **`HttpOnly; SameSite=Strict; Secure=false`
   (loopback)** cookie. The long-lived token never appears in URL/history/`Referer`/`argv`. **WS
   auth in the first frame.**
4. **Harden secondary channels:** never log `Authorization`/`Cookie`; no open redirects; strict CSP
   header `default-src 'self'; connect-src 'self' ws://127.0.0.1:<port>`; no directory listing;
   request-size + timeout + concurrency limits; token **TTL + revocation** (revoke on session end).
5. **§11 live-verification (mandatory, every (re)start):** fold assertions into the boundary
   self-test — forged `Host` → 403, tokenless → 401, secret never logged, rebinding `Host` → 403,
   bind is loopback. Not just at cold start (the VS Code `--connection-token-file` lesson).
6. **Same-user hardening (recommended):** `SO_PEERCRED`/`LOCAL_PEERCRED` to drop foreign-UID peers;
   document the residual (an unauthenticated same-host same-UID user can probe `/`).

**On-demand is a hard requirement** (ADR-0020 GUI-role): the always-on daemon exposes **no** network
service; the server starts only on explicit user action (`opentrapp configure` / launcher) and is
torn down on session end / idle. Exposure window = minutes, not 24/7.

**Plaintext on loopback is correct** (TLS on `127.0.0.1` adds a cert-trust problem for no threat
reduction — ADR-0022 §4).

---

## §3 — Crate structure + handler lift

- **New crate `app/src-tauri/crates/viewer-server/`** (axum + tower-http + tokio-tungstenite, serde,
  rand). Pure-Rust async, **must be gtk/webkit-free** — extend the `ci.yml:82` `cargo tree` gate to
  also assert `viewer-server` pulls no `tauri|wry|webkit`. (Scaffolded; currently `[workspace].exclude`d
  — move it to `[workspace].members` when the spike passes.)
- **Lift the GUI-resident handler bodies into `opentrapp-core`** as transport-neutral `async fn`s
  (ADR-0022 step 1; independently valuable — furthers ADR-0019). The current
  `app/src-tauri/src/commands/*.rs` bodies mostly already delegate to `opentrapp-core::orchestrator`;
  the lift moves the remaining GUI-side logic (credentials, telegram, sentinel, egress, diagnostics,
  lifecycle routing) down so **both** the Tauri command shims **and** the axum routes call the same
  core fns. Tauri command shims become one-liners during the migration (deleted at cutover).
- The axum server runs **in-process in the daemon** (started on demand) — it reuses the daemon's
  `opentrapp-core` handle (`control::submit`, `markers`, `supervisor`, `orchestrator`). It does **not**
  duplicate orchestration logic (CLAUDE.md §5 generic-backend constraint).

---

## §4 — Frontend transport shim (near-1:1; preserve the public API)

- **`app/src/lib/tauri.ts`:** swap only the **internal** `invoke()` body → `fetch('/api/'+cmd, {method:'POST', credentials:'same-origin', body: JSON.stringify(args)})`; swap the `listen()` hooks → one shared **WS client** (`/api/events`) that dispatches by `event` name. **Every exported function keeps its current name + signature**, so the ~73 calling components are untouched. (Scaffold: `app/src/lib/api-transport.ts` shows the fetch+WS client to wire in.)
- **Plugin replacements** (all already the code's error-tolerant fallbacks — ADR-0022 §5):
  `@tauri-apps/plugin-shell` `open(url)` → `window.open(url, '_blank', 'noopener')`;
  `clipboard-manager` `writeText` → `navigator.clipboard.writeText`;
  `notification` → Web Notifications API; `autostart` → daemon-side service unit (the toggle calls a
  new endpoint or is dropped from the web panel); `store` → `localStorage` (UI prefs only).
- **Session bootstrap:** a tiny entry script reads `#n=` from the fragment, exchanges it at
  `/api/session`, clears the fragment (`history.replaceState`), then boots React. WS connects with the
  cookie/first-frame token.

---

## §5 — Build + serve wiring

- `npm run build` → `app/dist/` (unchanged). The viewer-server serves `dist/` (embed via `rust-embed`
  for a single binary, **or** read from a resource path next to the daemon). API + WS under `/api/*`;
  everything else → the SPA (`index.html` fallback for client routing).
- **Dev:** run `vite` on `:1420` and the viewer-server on its ephemeral port; either point vite's proxy
  at `/api` → the server, or run the server serving a dev build. Keep `npm run dev` HMR working.
- **Cutover packaging:** the per-OS installer ships `{daemon binary + viewer-server (or merged) +
  service unit + launcher}`; the Linux `.deb` **drops** `libwebkit2gtk`/`libappindicator`. (Per-OS
  service install + packaging is **hardware-gated** — test on capable hardware, not this box / not CI.)

---

## §6 — The contract test rewrite

`tests/orchestrator-check.sh` §"6. Frontend-Backend Contract" currently parses `tauri.ts` `invoke()`
calls and matches them against the Rust `generate_handler!` registry. **Rewrite it to a route
registry**: every `fetch('/api/<cmd>')` in `tauri.ts` must have a matching axum route, and every route
must have a frontend caller (info-only the other way). This keeps the API contract enforced after the
transport swap. Keep the embedded-copy sync discipline (ADR-0023) if the test is vendored.

---

## §7 — Definition of Done (the 21 clear)

1. The spike passed its threat-model review (§0) — recorded.
2. axum `viewer-server` is a workspace member, gtk/webkit-free (CI `cargo tree` gate green on it).
3. All §1 endpoints + the `/api/events` WS implemented; the §6 contract test green; the React app
   reaches **parity** with the Tauri GUI in a browser (manifest render, command exec, streaming,
   wizard/activation, egress approvals, sentinel).
4. The §2 security middleware is **live-verified in the boundary self-test** (every (re)start).
5. **Cutover:** the `opentrapp` Tauri crate + all `tauri-plugin-*` + the webview config are **deleted**;
   `Cargo.lock` contains **no** `gtk|gdk|webkit|wry|tauri*`; **`cargo audit` shows the 21 gone**;
   Scorecard *Vulnerabilities* climbs from 0 toward ~10 on the next scan. Update
   `docs/known-advisories.md` (the GTK3 table becomes "resolved by de-Tauri") and `deny.toml` (drop
   the now-absent ignores).
6. `make boundary-selftest` stays **exit 0 throughout** (the perimeter owner is untouched by the GUI
   transport change) — cold and resumed.

---

## §8 — Migration sequence (only after the spike passes; Tauri stays default until step 5)

1. **Lift handlers to `opentrapp-core`** (transport-neutral async fns). Verify: `cargo test --lib`,
   Tauri GUI still builds + works (it now calls the lifted fns).
2. **Build the axum `viewer-server`** mounting `POST /api/<cmd>` + `/api/events` WS; add it to
   `[workspace].members`; extend the WebKit-free CI gate to it.
3. **TS transport shim** (`tauri.ts` internals → fetch + WS); rewrite the §6 contract test.
4. **Displaced features** (§5 table), per-OS, on capable hardware.
5. **Cutover** (§7.5): flip the browser viewer to default, soak one release, then delete the Tauri
   crate + plugins + webview config — the commit where GTK3 leaves the build and Scorecard clears.

---

## §9 — What NOT to do

- **Do not** begin the migration before the §0 spike passes its threat-model review. Merge nothing
  from the spike.
- **Do not** make the server always-on, bind to anything but `127.0.0.1`, or skip the `Host`/`Origin`/
  token controls — each CVE in the prior-art (`de-tauri-viewer-research.md`) is one of these omitted.
- **Do not** put the long-lived token in a URL/query/`Referer`/`argv` (nonce-in-fragment only).
- **Do not** duplicate orchestration logic in the server — it calls `opentrapp-core` (CLAUDE.md §5).
- **Do not** drop the danger-gate: `boundary_impact: weakening` endpoints keep out-of-band human
  confirmation; no agent call edge (ADR-0021).
- **Do not** break `make boundary-selftest` — it must stay green at every step (the daemon perimeter
  owner is independent of the GUI transport).
- **Do not** weaken the CSP, log auth headers, or rely on browser PNA/LNA.
