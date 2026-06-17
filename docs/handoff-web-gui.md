# Handoff — build the de-Tauri loopback web GUI (Phase 3)

**For:** the next implementing agent, in a focused session. **From:** prep session 2026-06-17.
**Status:** prep complete (ADR + spec + scaffold + this handoff). **Implementation: not started, and
spike-gated** — do the §0 spike and pass its threat-model review *before* any migration.

---

## Mission (one paragraph)

Replace OpenTrApp's Tauri/GTK3 desktop webview with an **on-demand, loopback-only (`127.0.0.1`)
axum HTTP/WS server** that serves the *existing* React app and exposes the *same* manifest-driven
command set as JSON. The React frontend + its tests are preserved; only the transport changes
(`invoke`→`fetch`, `listen`→one WebSocket). When the migration completes and the Tauri crate +
`tauri-plugin-*` are deleted, the whole GTK3/WebKit tree leaves `Cargo.lock` and the **21 OpenSSF
Scorecard advisories clear genuinely** (Vulnerabilities check 0 → ~10; aggregate 7.7 → ~8.x). This
is the **single riskiest part of the mission** (a network service on a security tool), so it is
spike-gated and §11 live-verification is mandatory.

## Read in this order
1. [`docs/adr/0022-daemon-control-surface.md`](adr/0022-daemon-control-surface.md) — the decision, the security model (§3), the spike gate, the 5-phase migration. **The authority.**
2. [`docs/specs/2026-06-17-loopback-web-gui-implementation.md`](specs/2026-06-17-loopback-web-gui-implementation.md) — the buildable plan: §0 spike, §1 full API table (33 endpoints + 8 events), §2 security middleware, §3 crate/handler-lift, §4 frontend shim, §5 build, §6 contract test, §7 Definition of Done, §8 sequence, §9 do-not.
3. [`docs/de-tauri-viewer-research.md`](de-tauri-viewer-research.md) — security prior-art + why C1 (wait for GTK4) is dead.
4. [`app/src-tauri/crates/viewer-server/README.md`](../app/src-tauri/crates/viewer-server/README.md) — the scaffold map.
5. [`docs/known-advisories.md`](known-advisories.md) — why the 21 are GUI-only + spine-clean (the thing this clears).

## What's already scaffolded for you
- **`app/src-tauri/crates/viewer-server/`** — skeleton crate (axum/tower-http/tokio-tungstenite),
  **`[workspace].exclude`d** so cargo/CI ignore it until you activate it. Files:
  `src/routes.rs` (all 33 endpoints enumerated → the contract, concrete), `src/security.rs` (the §2
  model as typed stubs — the riskiest part), `src/events.rs` (the 8-event WS multiplexer),
  `src/session.rs` (the nonce→bearer handoff), `src/main.rs` (the spike entry wiring).
- **`app/src/lib/api-transport.ts`** — the frontend fetch + WS client the migration wires into
  `tauri.ts`'s internals (lint+type clean; not yet imported anywhere).

## Start here — the §0 spike (the gate; merge nothing)
1. Branch `spike/web-gui` (throwaway). Move `viewer-server` to `[workspace].members` *on the branch only*.
2. Implement the **full §2 security middleware** + serve `app/dist/` (run `npm run build` first) +
   **3 real handlers** (`list_components`, `get_status`, `run_command`) + the `stream-line`/`stream-end`
   WS. Lift those 3 handler bodies into `opentrapp-core` as transport-neutral `async fn`s.
3. Load it in **Firefox + Chromium**. Verify the SUCCESS criteria (spec §0): renders + interactive;
   `cargo tree -p viewer-server | grep -iE 'tauri|wry|webkit'` is empty; security **live-verified**
   (forged `Host`→403, tokenless→401, token absent from URL/history/`Referer`, rebinding `Host`→403,
   loopback-only bind); UX acceptable for occasional config use.
4. **Decision gate:** SUCCESS → proceed to the migration (spec §8), re-deriving production code (do
   not merge the spike). **KILL** → re-scope the viewer or accept the documented Tauri/Scorecard
   residual and record why. **C1 (wait for GTK4) is NOT a fallback.**

## Migration (only after the spike passes — spec §8)
1. Lift the GUI-resident handler bodies into `opentrapp-core` (transport-neutral). 2. Build the axum
`viewer-server`, add to `members`, **extend the WebKit-free CI gate** (`ci.yml:82` `cargo tree`) to it.
3. Swap `tauri.ts` internals → `api-transport.ts` (fetch + WS); rewrite `orchestrator-check.sh`
§"6. Frontend-Backend Contract" to a route-registry check. 4. Displaced features (spec §5 table),
per-OS. 5. **Cutover:** flip the browser viewer to default, soak one release, then **delete the
`opentrapp` Tauri crate + all `tauri-plugin-*` + the webview config** — the commit where GTK3 leaves
the build and Scorecard clears.

## Verification gates (per phase) + Definition of Done
- `make boundary-selftest` stays **exit 0 throughout**, cold + resumed (the perimeter owner is
  independent of the GUI transport — do not regress it).
- The §2 middleware is folded into the boundary self-test and **live-verified every (re)start** (§2.5).
- **Done = the 21 clear:** post-cutover, `Cargo.lock` has no `gtk|gdk|webkit|wry|tauri*`; `cargo audit`
  shows the 21 gone; Scorecard *Vulnerabilities* climbs; update `known-advisories.md` (GTK3 table →
  "resolved by de-Tauri") + `deny.toml` (drop the now-absent ignores).

## Hardware note
This dev box (7.2 GB) builds the daemon/core spine fine (3.5 s, 420 MB) but **cannot run the full
perimeter** and **per-OS service-install + packaging is hardware-gated** — do those on capable
hardware, not this box / not CI. The spike + the axum server + the frontend shim are all light and
fine here.

## Do not (full list: spec §9)
Don't start the migration before the spike passes · don't make the server always-on or bind off
`127.0.0.1` · don't skip `Host`/`Origin`/token controls · don't put the long-lived token in a
URL/query/`Referer`/`argv` (nonce-in-fragment only) · don't duplicate orchestration logic (call
`opentrapp-core`) · don't drop the ADR-0021 danger-gate on `boundary_impact: weakening` endpoints ·
don't break `make boundary-selftest`.

## Verified context you can rely on (from the 2026-06-17 prep)
- The Tauri command surface is **33 commands across 15 files in `app/src-tauri/src/commands/`** + **8
  event types**; all enumerated in spec §1 and `viewer-server/src/routes.rs`/`events.rs`.
- `opentrapp-core` + `opentrapp-daemon` are **already Tauri/GTK/WebKit-free** (CI-gated); the daemon
  already owns the perimeter (ADR-0019) and has the control inbox (`control::submit`, vault verbs).
- **All 21 advisories root in the Tauri GUI tree**; the spine is clean — so removing Tauri is the
  whole fix (no dependency bump does it; `cargo update` moves 0).
