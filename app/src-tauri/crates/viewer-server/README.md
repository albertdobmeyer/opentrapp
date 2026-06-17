# viewer-server — SCAFFOLD (ADR-0022 de-Tauri loopback web GUI)

> **This is a scaffold, not working code.** It is `[workspace].exclude`d in
> `../../Cargo.toml` so `cargo` ignores it and CI does not build it. It exists to make the
> shape of the work obvious. **Do not wire it into the workspace until the §0 spike passes.**

## What this becomes
The on-demand, **loopback-only** (`127.0.0.1`) axum HTTP/WS server that replaces the Tauri
webview: it serves the existing React app (`app/dist/`) and exposes the same manifest-driven
command set as JSON (`POST /api/<cmd>`) + a single `/api/events` WebSocket. No webview ⇒ no
GTK3 ⇒ the 21 Scorecard advisories clear at cutover.

## Read first
1. [`docs/adr/0022-daemon-control-surface.md`](../../../../docs/adr/0022-daemon-control-surface.md) — the decision + the security model (§3) + the spike gate.
2. [`docs/specs/2026-06-17-loopback-web-gui-implementation.md`](../../../../docs/specs/2026-06-17-loopback-web-gui-implementation.md) — the buildable plan: API table, security middleware, migration phases, Definition of Done.
3. [`docs/handoff-web-gui.md`](../../../../docs/handoff-web-gui.md) — the agent handoff (start here).
4. [`docs/de-tauri-viewer-research.md`](../../../../docs/de-tauri-viewer-research.md) — security prior-art + why C1 (wait for GTK4) is dead.

## Skeleton map
| File | Becomes |
|---|---|
| `src/main.rs` | Spike entry: bind `127.0.0.1:0`, build router, apply security layer, open the browser to the nonce URL. |
| `src/security.rs` | The §2 middleware: `Host`/`Origin` allowlist, token, loopback assert, CSP, `SO_PEERCRED`, the §11 live-verify hooks. **The riskiest part.** |
| `src/routes.rs` | The route registry — every `POST /api/<cmd>` mapped to its `opentrapp-core` fn. The API contract, made concrete. |
| `src/events.rs` | The `/api/events` WebSocket multiplexer (8 named events; first-frame auth). |
| `src/session.rs` | The launch-nonce → bearer exchange (`POST /api/session`), HttpOnly cookie. |

## The spike (do this first — see spec §0)
Minimal server + full §2 security + serve `app/dist/` + 3 handlers (`list_components`,
`get_status`, `run_command`) + the `stream-line` WS, loaded in Firefox + Chromium.
**SUCCESS:** renders + interactive; `cargo tree` gtk/webkit-free; security live-verified
(forged `Host`→403, tokenless→401, token not in URL/history/`Referer`, rebinding→403). **KILL** →
re-scope or accept the documented Tauri residual (NOT "wait for GTK4").
