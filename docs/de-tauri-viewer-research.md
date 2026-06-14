# De-Tauri viewer — grounding research (security prior-art + GTK4 upstream)

Evidence base for [ADR-0022](adr/0022-daemon-control-surface.md) (the daemon control surface /
de-Tauri spec). Two questions the de-Tauri decision hinges on were researched against primary
sources (2026-06-14): **(A)** how to securely serve a web UI from a privileged local daemon to the
user's browser, and **(B)** whether simply waiting for Tauri's GTK4 migration would clear the 19
GTK3 advisories without a rewrite (the "C1" fallback).

---

## A. Securing a loopback browser ↔ privileged-daemon channel — prior art

Surveyed: Jupyter, Syncthing, Home Assistant, Docker (Desktop/Engine), Ollama, VS Code
(servers/tunnels) + LSP, Open WebUI; plus the DNS-rebinding / "0.0.0.0-day" / PNA→LNA literature.
The consensus is strong and consistent.

### Patterns to ADOPT (none is sufficient alone — layer all three server-side controls)

1. **Strict `Host`-header allowlist** (`127.0.0.1:PORT` / `localhost:PORT` → 403 otherwise) — the
   **primary** anti-DNS-rebinding control. A rebounded request carries the attacker's hostname and is
   rejected. (Syncthing's Host check since v0.14.6; Jupyter Notebook PR #3714; Oligo "0.0.0.0-Day".)
2. **`Origin`/`Referer` validation** (incl. the WS handshake) — stops the no-credentials browser
   drive-by, but `Origin` can be absent or `null`, so it can't stand alone.
3. **Unguessable secret token** — stops requests that omit/forge `Origin` and all non-browser clients;
   but a token alone won't stop a rebinding page that can read responses, so the Host check stays
   primary.
4. **Keep the secret off the URL query string and off `argv`.** Query strings leak via Referer /
   history / logs (OWASP, CWE-598). `argv` is readable by any local user via `ps`/`/proc` (VS Code's
   `--connection-token`). Prefer a header, a **0600 file**, or stdin.
5. **Bootstrap handoff = URL *fragment* (`#…`) → immediately convert to an HttpOnly+SameSite=strict
   cookie on first contact** (Jupyter's single-use-launch-token). The fragment is not sent to the
   server and not in `Referer` — but it is NOT secret-grade (persists in history; readable by any XSS),
   so it's a usability win layered on top of the token check, never a replacement.
6. **Loopback bind is necessary but NOT sufficient.** It restricts which interface accepts TCP; it
   does nothing against the victim's own browser (rebinding / 0.0.0.0-day). **Do not rely on
   browser-side network controls** (PNA/LNA) — they're OS/version-dependent and currently don't cover
   cross-origin *local* requests.
7. **Strongest option where feasible: no inbound TCP listener at all** — a Unix socket / named pipe
   (Docker's model) or stdio (the LSP model) has no port for a rebinding browser or another local
   process to reach. A browser can't speak those, which is exactly why a browser viewer forces a
   loopback port — so this is the security cost the browser viewer pays, and the reason the listener
   must be **ephemeral**.
8. **Harden the secondary channels:** never log auth headers/cookies (Jupyter CVE-2022-24758); no
   open redirects (multiple Jupyter CVEs); no XSS in the served UI (it escalates to token theft);
   strict CSP; plan token **revocation** (short TTL + revocation list — Open WebUI's stateless-JWT gap).
9. **Verify the secure path actually works at the consuming end (§11).** VS Code's
   `--connection-token-file` was silently non-functional in 1.90.1 (the file was never read). "Saved
   to a file" ≠ "used." Assert live that a forged `Host` is rejected, the token is required, and the
   secret is never logged — on every (re)start, not just cold.

### ANTI-PATTERNS that caused real CVEs (do NOT do)

| Anti-pattern | Consequence (real CVE) |
|---|---|
| Loopback + CORS but **no auth, no Host check** | Ollama: unauthenticated, mass-exposed, RCE (CVE-2024-37032 "Probllama") — CORS is not auth and not a rebinding defense |
| **Token in the query string** | Jupyter token-leak chain (CVE-2023-39968 + CVE-2024-22421): open-redirect + traversal exfiltrate the `?token=` cross-origin |
| **Logging auth headers** | Jupyter CVE-2022-24758 (secrets in logs on 5xx) |
| **Plaintext TCP to a privileged daemon** | "reach dockerd = root": Docker `:2375` cryptojacking in the wild; Docker Desktop CVE-2025-9074 (unauth Engine API, CVSS 9.3) |
| Open redirect / XSS in the served UI | escalates to token theft (Jupyter, Open WebUI JWT theft) |
| Trusting proxy headers (`X-Forwarded-For`) without pinning the proxy | Home Assistant `trusted_networks` bypasses |
| Blocklist-style SSRF/URL filters | Open WebUI fix bypassed twice (IPv6-mapped / redirects) — use allowlists |
| Secret on `argv` | VS Code `--connection-token` readable via `ps` |

### Ephemeral / on-demand vs always-on

The mechanism that reduces attack surface is **"no standing inbound listener."** Every product that
suffered the worst in-the-wild abuse runs **always-on** (dockerd, Ollama, Home Assistant) — a standing
target found by continuous internet scanning within minutes. On-demand (Jupyter is user-launched, per
session, fresh token; VS Code tunnels are outbound-only with no listener; LSP-over-stdio has no port)
bounds the exposure window. **Caveat (no overclaiming):** on-demand reduces the exposure *window*, not
per-request *strength* — Jupyter's on-demand model didn't save it from the token-leak CVEs. On-demand
is a multiplier on top of the three server-side controls, not a substitute.

**This validates and refines OpenTrApp's earlier design** (ADR-0022 §loopback-server): ephemeral
loopback + Host/Origin/token is exactly the consensus. The refinements to fold into ADR-0022: the
fragment-nonce→HttpOnly-cookie handoff (not a bare token in the URL), no-logging, CSP, revocation/TTL,
and the §11 "verify the secure path actually works" assertion on every resume.

---

## B. C1 — "wait for Tauri's GTK4 migration"? **Verdict: NOT viable (HIGH confidence).**

As of 2026-06-14, there is **no shipped GTK4 / webkitgtk-6.0 path in any Tauri/wry release, no
committed version target, and no public timeline.** Primary-source findings:

- Latest stable **wry 0.55.1 (2026-05-04)** still depends on `webkit2gtk 2.0.2` (GTK3) and `gtk ^0.18`;
  no GTK4 feature flag. The tracking issue **wry#1474 "Upgrade wry to gtk4-rs and webkit6"** (opened
  2025-01-30) is **Open / Todo / not started**. The windowing layer **tao#1104** (GTK4 port) is still a
  **draft**. The GTK4 WebKit binding crate (`webkit6`) exists but wry does not consume it yet.
- **Even a GTK4 webview would not clear the advisories on its own.** GTK3 enters a Tauri app via
  *multiple* sibling crates: the webview (wry), **the system tray** (`tray-icon → libappindicator → gtk
  0.18` — libappindicator has no GTK4 version; the ksni rewrite, tray-icon#201 + tauri#12319, is
  **open/unmerged** and libappindicator stays the default), and **menus** (`muda`, still GTK3). Clearing
  the ~10–11 advisories requires wry AND tao AND tray-icon(ksni) AND muda to all land on GTK4.
- Non-GTK Linux paths are not production-ready: **Servo/Verso** integration is dormant and the Verso
  browser was archived 2025-10-08; **CEF**-in-Tauri exists only macOS-only / PoC.
- Maintainer statements: GTK4 "at some point yes. We can't provide any timeline yet"; the official
  roadmap doesn't mention GTK4. Realistic read: **a year or more, no committed path**, possibly slipping
  to a future major or sidestepped by CEF.

### Decision impact

1. **C1 is removed as a viable fallback.** The plan's "no-regret parallel" (just bump Tauri when GTK4
   lands) does not exist on any plannable timeline. This **strengthens the de-Tauri (browser-viewer)
   commitment** — subject to the spike still proving the UX and the security model.
2. **De-Tauri must also drop the tray + native menus**, not just the webview — otherwise `tray-icon →
   libappindicator → gtk 0.18` keeps the largest GTK3 inflow. This is *already* the direction: the
   daemon is the persistent presence (no tray) and the browser viewer has no native menus (ADR-0019/
   0020). So the browser-viewer + daemon-presence model is what removes the *entire* GTK3 set; a partial
   measure would not.
3. **One useful corollary:** these GTK3 crates are Linux-`cfg`-gated — they appear in `Cargo.lock`
   (audit noise that Scorecard counts) but are not compiled on macOS/Windows. The advisories only
   genuinely clear on the *Linux* build, which is exactly the build the de-Tauri move changes.

**Net:** waiting is off the table; the de-Tauri browser-viewer (with the daemon as presence and no
tray/menus) is the path that actually removes the GTK3 stack, and the loopback security model above is
the consensus design — now to be specified, and spike-validated, in ADR-0022.
