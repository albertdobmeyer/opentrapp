# Release notes — v0.8.0 (the de-Tauri foundation)

This release lands the **foundation** for OpenTrApp's product direction ([ADR-0020](adr/0020-product-identity-and-distribution.md)): the perimeter-owning daemon is the product, and the GUI becomes a **thin, on-demand projection** of one manifest-driven core. v0.8.0 makes that real in code — a tested, WebKit-free loopback **web-GUI server** and a dual-transport frontend — without yet changing what you install. It also modernises the whole toolchain.

> **Honest framing (the bar, [`CLAUDE.md` §12.4](../CLAUDE.md)).** This is a **foundation** release, not the finish line. **OpenTrApp still ships as the Tauri 2 desktop app**; the browser viewer is built and verified but is **not** the shipped default, **GTK3/WebKit is still in the installed binary**, and the cross-OS service/packaging work is **not** done (it is hardware-gated). Nothing below claims de-Tauri is complete. What is claimed is verified.

---

## Headline — the GUI is becoming a projection, and the core is transport-neutral

Until now, every user-facing command lived in the Tauri command layer, fused to the GUI's `AppState`/`AppHandle`. This release **lifts that logic into the Tauri-free `opentrapp-core`** and stands up a **second transport** over it — a loopback HTTP/WS server — so the exact same orchestration core now backs both the desktop app and a browser. The desktop app is unchanged for users; underneath, the GUI is now one projection among (eventually) several.

### What landed (all verified)

- **10 command handlers lifted into `opentrapp-core`** (migration step 1): `diagnostics`, `telegram`, `health`, `status`, `execute` (run_command/load_options), `workflow`, `config`, `prerequisites`, `credentials`, and the `sentinel` judge. Each is now a transport-neutral function; the `#[tauri::command]` wrappers became one-line shims. Several **security guards gained their first direct tests** in the process — the path-traversal containment (config + template copy), the **0600 `.env` writer**, and the **never-silently-allow** judge verdict.
- **`viewer-server` — a real, tested, WebKit-free loopback web-GUI server** ([ADR-0022](adr/0022-daemon-control-surface.md)). It serves the existing React app over a token-gated `127.0.0.1` HTTP/WS surface and calls `opentrapp-core` (no duplicated logic). Its **§2 security middleware** is the riskiest part and is covered by tests: Host allowlist (anti-DNS-rebinding), Origin allowlist incl. the WS handshake, bearer via **HttpOnly cookie** (constant-time), a ≥256-bit token + single-use launch nonce + **0600 session file**, the `/api/session` nonce→bearer bootstrap, and the full component-operation route surface. CI asserts both the daemon **and** viewer-server dependency graphs are **gtk/webkit/wry-free**.
- **A dual-transport frontend shim.** The single `invoke()` chokepoint in `tauri.ts` now uses the native Tauri IPC in the webview **and** `POST /api/<cmd>` (same-origin, cookie-bearer) in a plain browser — with the public API unchanged, so the 28 consumer files don't change. The Tauri plugins (open-URL, clipboard, settings store) gained web-shims (`window.open`, the Web Clipboard API, `localStorage`).
- **The loopback threat model is written and accepted** as `threat-model.md` **T8**: a STRIDE decomposition of the loopback server + an in-process-WebKit-vs-browser attack-surface comparison, with the hostile-extension residual called out as the load-bearing judgment. This was the gate the whole direction depended on, and it cleared.

---

## Toolchain modernised (verified)

- **ESLint 9 → 10** (+ `eslint-plugin-import` → the maintained `import-x` fork, unicorn 68), which required and brought a **Node 20 → 22** bump across all CI jobs. The perimeter is containerized, so this only affects the GUI build + dev tooling.
- **React 18 → 19** (react/react-dom/@types). The codebase was already React-19-ready (createRoot, no legacy patterns), so it was a clean dependency migration: `tsc` clean, full test suite green.
- A **CVSS-7.5 advisory** in a transitive dep (`quinn-proto`) was caught by CI and patched, and the credential-passing path was hardened to keep keys **off the process table** ([#75](https://github.com/albertdobmeyer/opentrapp/pull/151)).

---

## The perimeter is unchanged — and that is the point

This release adds **no** new capability to the contained agent and **changes nothing** about the five-container security perimeter, the zero-trust air-gap, the proxy-side credential injection, or the skill-firewall CDR pipeline. The credential separation, the network isolation, and the danger-gating invariants ([`CLAUDE.md` §1](../CLAUDE.md)) hold exactly as before. The de-Tauri work is **plumbing under the GUI**, not a change to the boundary.

---

## What is NOT in this release (honestly)

These are built-toward but **not** shipped, and are **not** claimed:

- **The browser viewer is not the default.** You still install and run the Tauri desktop app. The loopback server exists and is tested, but the daemon does not yet serve it as the user-facing GUI.
- **GTK3/WebKit is still in the installed binary.** The Scorecard `Vulnerabilities` advisories (all upstream, non-exploitable GTK3 warnings) are **unchanged** — they vanish only at the future cutover, not here.
- **Cross-OS service / launcher / packaging is not done.** Running the daemon as a `systemd`/launchd/Windows logon service, the OS launcher, the per-OS installers, and the loopback live-verification self-tests are **hardware-gated** (they need real Windows/macOS/Linux machines + the running perimeter) and are the next phase.
- **The remaining handlers** (`stream` events bridge, `egress` approvals) and the daemon-serves-the-viewer integration are follow-ups.

The resting-memory and idle-pause guarantees remain **candidates pending hardware verification** (unchanged from v0.7.2-rc; see [`footprint-and-device-usability.md`](footprint-and-device-usability.md)) — this release makes no new claim about them.

---

## Upgrade / compatibility

- No user action required; behaviour is identical to v0.7.x for end users.
- **Contributors:** the frontend now requires **Node ≥ 22** (`app/.nvmrc`); run `cd app && nvm use` before `npm` work, or the unicorn-68 lint gate will fail on Node 20.

---

## Why this is the right shape

Substance first, visibility second ([`CLAUDE.md` §12.4](../CLAUDE.md)). v0.8.0 is large because the *foundation* is large — a transport-neutral core, a tested loopback server with a real security model, a dual-mode frontend, and a modern toolchain — and it is honest because every headline here is verified by tests in this repository, while everything still hardware-gated is named as such. The vision (a daemon-first, agent-operable, browser-projected security perimeter) is now visibly under construction in the code, not just the docs.
