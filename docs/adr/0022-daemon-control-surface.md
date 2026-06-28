# ADR-0022 — Daemon control surface: CLI, on-demand loopback viewer (de-Tauri), optional MCP

**Status:** Accepted — the de-risking spike passed and the cutover **shipped on `main`** (de-Tauri
[#184](https://github.com/albertdobmeyer/opentrapp/pull/184), 2026-06-24): the headless daemon owns the
perimeter and the on-demand loopback `viewer-server` is the GUI projection (the transport security
model below is implemented in [`crates/viewer-server/`](../../app/src-tauri/crates/viewer-server/)). The
CLI-first and optional-MCP command surfaces remain **forthcoming**
([ADR-0020](0020-product-identity-and-distribution.md)). Inherits the danger-gate of
[ADR-0021](0021-danger-gated-agentic-control-plane.md) unchanged.
**Cross-references:** [ADR-0019](0019-headless-daemon-gui-viewer-split.md) (the Tauri-free daemon owns
the perimeter) · [ADR-0020](0020-product-identity-and-distribution.md) (three projections of one
daemon) · [ADR-0021](0021-danger-gated-agentic-control-plane.md) (authorization model) ·
[ADR-0008](0008-tauri-over-electron.md) (Tauri choice — being phased out for the viewer) ·
[ADR-0011](0011-zero-trust-self-sufficient-bootstrap.md) ·
[`de-tauri-viewer-research.md`](../de-tauri-viewer-research.md) (security prior-art + C1 verdict) ·
[`threat-model.md` T8](../threat-model.md) (the loopback-viewer threat model + attack-surface comparison this ADR's spike gate consumes; T7 for the agentic control plane) · [CLAUDE.md §10/§11](../../CLAUDE.md)

---

## Context

ADR-0020 ratified that OpenTrApp's product is the daemon, projected through **one manifest-driven
command set** to humans and host agents via a CLI, a web GUI, and an optional MCP adapter. ADR-0021
ratified the authorization model (the danger-gate). This ADR decides **the control surface itself** —
how the projections expose the daemon — and, in doing so, resolves the de-Tauri question, because the
GUI projection is currently a Tauri/GTK3 webview and that is the only thing carrying the 19 Scorecard
advisories.

Two researched facts ([`de-tauri-viewer-research.md`](../de-tauri-viewer-research.md)) drive the design:

- **C1 ("wait for Tauri's GTK4 migration") is ruled out (HIGH confidence).** No GTK4 path in any wry
  release, no committed timeline (~1+ year), and even a GTK4 webview would not clear the set because
  the **tray** (`tray-icon → libappindicator → gtk 0.18`) and **menus** (`muda`) independently pull
  GTK3. The advisories clear only by removing the webview **and** the tray/native-menus — which is the
  browser-viewer + daemon-as-presence model.
- **The loopback browser↔daemon channel has a strong security consensus** (Jupyter, Syncthing, Docker,
  Ollama, VS Code/LSP, the DNS-rebinding/0.0.0.0-day literature), with a well-documented CVE record of
  what happens when it is done wrong. The model below adopts that consensus.

## Decision

### 1. One command surface, three projections, one gate

The daemon exposes a single manifest-driven command/event API. Three projections sit over it, each
**inheriting ADR-0021's danger-gate unchanged** (`boundary_impact: weakening` → out-of-band human
confirmation; no agent call edge to weakening writers):

- **CLI** (`opentrapp …`) — the universal human + host-agent surface; the primary, always-available
  projection.
- **On-demand loopback web GUI** — the non-technical config panel (ADR-0020 tenet 6); replaces the
  Tauri webview (the de-Tauri move).
- **Optional MCP adapter** — for host agents preferring structured tools; a thin wrapper over the same
  API; same gate. Detailed shape deferred.

`markers remain truth` (ADR-0019): every projection re-seeds state from authoritative reads; the
event stream is a fast path, never the source of truth.

### 2. De-Tauri: the GUI becomes an on-demand loopback web panel; the daemon is the presence

The daemon (already headless, Tauri-free, owns the perimeter — ADR-0019) gains an **on-demand** HTTP/WS
server that serves the existing React app to the user's browser. **No bundled webview ⇒ no GTK3.** The
tray and native menus are **dropped** (the daemon is the persistent presence; an OS launcher opens the
loopback URL) — which, with the webview gone, removes the *entire* GTK3 inflow. The React frontend and
its test coverage are preserved; only the transport changes (`invoke`→`fetch`, `listen`→WS).

### 3. The on-demand loopback server security model (the crux — we are a security tool)

Adopts the researched consensus; **on-demand/ephemeral is a hard requirement, not an option**
(ADR-0020 GUI-role constraint): the always-on daemon exposes **no network service**; the config server
is started only on explicit user action (launcher / `opentrapp configure`) and torn down on session
end / idle. The exposure window is minutes, not 24/7.

While up, **all three server-side controls (none sufficient alone):**

1. **Bind `127.0.0.1` only, ephemeral port**; unit-assert the bound address is loopback.
2. **Strict `Host`-header allowlist** (`127.0.0.1:PORT`/`localhost:PORT` → 403) — the *primary*
   anti-DNS-rebinding control; **+ `Origin` allowlist** (incl. WS handshake) **+ an unguessable ≥256-bit
   token**. Loopback bind alone is necessary-but-insufficient (rebinding / 0.0.0.0-day); browser-side
   controls (PNA/LNA) are not relied upon.
3. **Token handoff that does not leak:** daemon writes `{port, token}` to a **0600** file; opens the
   browser to a bare loopback URL carrying a single-use, short-TTL **launch nonce in the URL fragment**
   (`#n=…`, not sent to the server, not in `Referer`); the bootstrap JS exchanges it at `/api/session`
   for the bearer, stored in memory/`sessionStorage` and **converted to an HttpOnly+SameSite=strict
   cookie** on first contact. The long-lived token never appears in URL/history/`Referer`/`argv`. WS
   auth in the first frame.
4. **Harden secondary channels:** never log auth headers/cookies; no open redirects; strict CSP
   (`default-src 'self'; connect-src 'self' ws://127.0.0.1:PORT`); no directory listing;
   request-size/timeout/concurrency limits; token **TTL + revocation**.
5. **§11 — verify the secure path actually works, at the consuming end, on every (re)start** (the VS
   Code `--connection-token-file`-was-silently-broken lesson): assert live that a forged `Host` is
   rejected, the token is required, and the secret is never logged — fold these into the boundary
   self-test, not just cold start.
6. **Same-user hardening (recommended):** `SO_PEERCRED`/`LOCAL_PEERCRED` to drop foreign-UID
   connections; document the residual (an unauthenticated same-host user can probe `/`).

**§10 reconciliation (explicit, not a slip).** CLAUDE.md §10 forbids "network services (no
remote-management surface)." A **token-gated, 127.0.0.1-only, Origin/Host-validated, on-demand,
ephemeral** config surface is **local control, not remote management**: unreachable off-host by
construction, authenticable only by the owning user, exposing only the same projection the in-process
Tauri IPC already exposed. This amends ADR-0019's "never a TCP port" line deliberately (a browser
cannot speak a Unix socket). **If the spike's threat-model review finds the surface unacceptable, that
is a kill criterion.** That review is now written as [`threat-model.md` T8](../threat-model.md) (STRIDE
decomposition + the in-process-WebKit-vs-loopback attack-surface comparison); its load-bearing residual
is the hostile-extension path, and maintainer acceptance of that residual is the explicit go/no-go.

### 4. Transport: WebSocket over SSE

A single multiplexed `/api/events` WebSocket (named events, mirroring Tauri's `emit/listen`); clean
first-frame token auth (SSE would force a query-string token → the leak we just designed out); handles
the bursty high-frequency `stream-line` channel; near-1:1 frontend change. Server crate: **axum +
tower-http + tokio-tungstenite** — pure-Rust async, **verified zero gtk/webkit** (the existing CI
`cargo tree` WebKit-free gate extends to it); loopback plaintext (TLS on `127.0.0.1` adds a cert-trust
problem for no threat reduction).

### 5. Displaced native features (per-OS)

| Feature | After de-Tauri |
|---|---|
| Autostart | **The daemon** as a `systemd --user` unit / launchd LaunchAgent / Windows logon task — starts the headless ~30–60 MB daemon (no WebKit); strictly better than starting a webview app |
| Tray + single-instance | Gone: the daemon is the persistent presence + holds the `RunGuard`; an OS launcher opens the loopback URL via `xdg-open`/`open`/`start` (no `plugin-shell`) |
| Settings persistence | Browser `localStorage` (pure UI prefs); anything the daemon needs stays a daemon-side marker |
| Updater | A small daemon self-update (verify the existing minisign pubkey + atomic replace + service restart) — simpler than a webview-bundle updater |
| Notifications / clipboard / shell-open | Web Notifications API / `navigator.clipboard` / `window.open` (all already the code's fallbacks) |
| Packaging | Thin per-OS installer (daemon binary + service unit + launcher); Linux `.deb` drops `libwebkit2gtk`/`libappindicator`. Registry-native publish is Tier-3 (ADR-0020) |

## Consequences

**Positive**
- Removes the entire GTK3 stack from the shipped Linux binary → the 19 advisories clear *genuinely*
  (Scorecard Vulnerabilities), not relocated; the best-practices claim becomes defensible.
- One command surface → CLI, web, MCP are thin and consistent; the danger-gate is inherited once.
- The daemon-as-presence model is leaner (no resident WebKit) and aligns with ADR-0018/0019.
- The renderer becomes the user's own continuously-patched browser, not a pinned aging webkit2gtk.

**Negative / cost (honest)**
- **A loopback server is a real attack surface on a security tool.** The model above is the consensus
  defense, but it must be *implemented and verified* exactly — the spike's threat-model review is a
  gate, and §11 live-verification is mandatory. This is the single riskiest part of the whole mission.
- UX shift for the non-technical persona (background daemon + browser config panel vs native window);
  mitigated by the launcher + optional PWA install, and softened because the GUI is occasional config,
  not a daily-use surface (ADR-0020).
- Per-OS service-install + packaging is the long pole — hardware-gated (§11; this dev box can't run the
  full perimeter), tested on capable hardware, not CI.
- New `boundary_impact` tags, the contract test rewrite (handler↔invoke → route registry), and the
  handler-lift are real engineering (see Migration).

## Alternatives considered

- **C1 — wait for Tauri GTK4.** Ruled out by research (no path / ~1+ yr / tray+menus also block).
- **C2 — pure-Rust GUI (egui/Slint).** Rejected: discards the React app + its 80% test coverage for
  non-exploitable advisories; highest effort.
- **Extract the GUI to a separate repo.** Rejected in ADR-0020 (optics-only; relocates, doesn't remove).
- **Keep Tauri as-is.** Rejected: Scorecard-illegible packaging + Tauri-bound, and C1 is dead, so the
  advisories never clear.
- **Always-on local server.** Rejected: every worst-abused prior-art product is always-on; on-demand
  bounds the exposure window (ADR-0020 GUI-role constraint).

## The de-risking spike (throwaway; gates the migration)

Before any migration, a scratch-branch spike (merge nothing) proves the riskiest assumptions: a minimal
axum server on `127.0.0.1:0` with the **full §3 security middleware**, serving the built React app, 3
real handlers + the `stream-line` WS, loaded in real browsers. **SUCCESS:** app renders/interactive;
`cargo tree` on the server crate is gtk/webkit-free; the security middleware is **live-verified**
(forged `Host` rejected, tokenless rejected, token absent from URL/history/`Referer`, rebinding `Host`
rejected); UX judged acceptable for the config-panel use. **KILL** (→ re-scope or accept the documented
Tauri/Scorecard residual; C1 is *not* a fallback): the loopback surface fails threat-model review, or
the UX is unacceptable, or WS can't carry `stream-line`, or a webview-only load-bearing capability
surfaces (none found in the inventory).

## Migration phasing (only if the spike passes; Tauri stays the shipped default throughout)

1. **Lift the ~12 GUI-resident handler bodies into `opentrapp-core`** as transport-neutral `async fn`s
   (independently valuable; furthers ADR-0019).
2. **axum server** (new `crates/viewer-server` or a daemon module) mounting them as `POST /api/<cmd>` +
   the `/api/events` WS; extend the WebKit-free CI gate.
3. **TS transport shim:** swap only `tauri.ts`'s `invoke()` body → `fetch` and the `listen()` hooks → one
   WS client; the public `tauri.ts` API is preserved so the ~73 frontend files don't change. Rewrite the
   `orchestrator-check.sh §5` handler↔invoke contract test to the route registry.
4. **Displaced features** (§5), per-OS, on capable hardware.
5. **Cutover:** flip the browser viewer to default, soak one release, then **delete the `opentrapp` Tauri
   crate + all `tauri-plugin-*` + the webview config** — the commit where GTK3 leaves the build and
   Scorecard goes clean. Boundary self-tests stay green throughout (the perimeter owner is untouched).

## What this ADR does NOT decide

- **Registry packaging mechanics** (crates.io/Homebrew/recognized publish; GHCR) → Tier-3 spec (ADR-0020).
- The concrete **`boundary_impact` tags** and the out-of-band confirmation UX → ADR-0021 implementation.
- The **MCP adapter's** detailed schema → a later record; it inherits §1–§3 unchanged.
