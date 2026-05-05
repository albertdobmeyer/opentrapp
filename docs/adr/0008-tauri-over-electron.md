# ADR-0008 — Tauri 2 over Electron, native, and web-only

**Status:** Accepted
**Decision date:** 2026-03-23 (initial application scaffold)
**Implemented by:** [`app/src-tauri/Cargo.toml`](../../app/src-tauri/Cargo.toml) (Tauri 2 dependency); [`app/src-tauri/tauri.conf.json`](../../app/src-tauri/tauri.conf.json); [`app/package.json`](../../app/package.json) (React 18 + Vite frontend)
**Verified by:** Build pipeline produces 9 platform binaries per release ([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) `build-and-release` matrix); per-binary size verified at release time

---

## Context

The application needs to be a desktop product that a non-developer user can install on Linux, macOS, or Windows. It also needs to:

- Own the lifecycle of four containers (start, stop, restart, observe)
- Read and write configuration files in the user's home directory with appropriate permissions
- Render a rich UI (a wizard, a status dashboard, configuration editors, a workflow runner)
- Be small enough that a user is willing to download and install it
- Be auditable enough that a security-conscious reader can read it end-to-end
- Pose a small attack surface itself, since the application is the perimeter's lifecycle owner and a compromise of the application would defeat much of the architecture

Three desktop-application architectures could plausibly meet these requirements:

1. **Electron** — Chromium + Node.js bundled with the application. Mature ecosystem; large community; ports cleanly to the three target platforms.
2. **Native** — per-platform native UI (Cocoa on macOS, GTK or Qt on Linux, WinUI on Windows). Smallest binary; tightest platform integration; highest implementation cost (three UIs in three different toolkits).
3. **Tauri** — the host platform's WebView (WebKit on macOS, WebKitGTK on Linux, WebView2 on Windows) + a Rust backend. Small binary; modern web frontend; one codebase for the UI; Rust for the backend.
4. **Web-only** — host the UI as a website, expect the user to run the perimeter manually.

The fourth option is the architectural baseline — it is what a user would do without this application — and the gap between "user runs `compose up` from a terminal" and "user clicks through a wizard" is exactly the value proposition. The choice is among the first three.

## Decision

The application is implemented in **Tauri 2** with a React 18 + TypeScript + Vite frontend and a Rust backend.

The choice was driven by four properties of the design problem:

**(a) Binary size.** A Tauri release bundle is 5–10 MB per platform (compared with Electron's typical 80–150 MB). The user is asked to download a desktop application from a small open-source project; download size is a non-trivial friction layer. The smaller bundle also reduces the supply-chain surface — fewer bytes to verify against the SLSA build provenance ([`docs/reproduce.md`](../reproduce.md) §4).

**(b) Memory footprint.** The Rust backend at idle uses ~30–50 MB; the WebView reuses the host platform's existing browser engine rather than bundling a fresh Chromium. On the maintainer's dev laptop (7.2 GB RAM, four containers running, Cursor IDE open), the headroom matters operationally — the difference between "the application can run alongside the perimeter" and "the application contests memory with the perimeter" is felt directly during development.

**(c) Rust backend.** The application houses lifecycle ownership, manifest orchestration, secret redaction, signal handlers, and the perimeter watchdog. These are responsibilities that benefit from Rust's type system and memory safety. The backend is the security-relevant layer; making it Rust rather than Node makes it easier to reason about end-to-end.

**(d) Modern web frontend.** The UI is information-rich (a wizard with progressive disclosure, a dashboard with real-time status, configuration editors, workflow runners). The web stack — React, TypeScript, Vite — is the right tool for that kind of UI. The alternative (three native UIs in three toolkits) would have produced a substantially less-rich UI for substantially more implementation cost.

The choice trades off against three Electron strengths:

- Electron's ecosystem is larger; finding answers to specific Electron questions on Stack Overflow is faster.
- Electron's API surface is more uniform across platforms; Tauri's WebView abstraction has per-platform corner cases (mostly around drag-and-drop and clipboard behaviour).
- Electron's debugging tools are more mature; Tauri's are catching up.

These costs are real but they have not been blockers for the project. The Tauri 2 framework's Rust API and frontend bridge have been sufficient for every backend command the application needs.

## Consequences

### Positive

- **The release bundle is small.** A Linux `.AppImage` is 4–6 MB; the `.deb` is similar; the `.rpm` is similar; the macOS `.dmg` is 4–8 MB; the Windows `.msi` is 4–8 MB; the Windows `.exe` is similar. The 9 platform binaries on a v0.3.0 release total ~50 MB; an Electron equivalent would total ~600 MB.
- **The runtime memory footprint is modest.** The application at idle (perimeter running, status aggregator polling at 60-second cadence) consumes ~80–120 MB. On a 7.2 GB development machine running the full stack, this leaves headroom for the four containers (~600 MB) plus an IDE plus a browser plus the user's other tools.
- **The Rust backend is auditable.** A security-conscious reader can read every line of the backend in a sitting. The main entry points — `src-tauri/src/main.rs`, `src-tauri/src/lifecycle.rs`, `src-tauri/src/status_aggregator.rs`, the orchestrator and command modules — are around 2,500 lines total. There is no opaque framework behaviour to take on faith.
- **The frontend is fast to iterate on.** React + Vite + TypeScript is the modern web stack; HMR works as expected; UI work proceeds at frontend-development pace rather than at desktop-development pace.
- **Cross-platform builds are straightforward.** The `tauri-action` GitHub Action handles per-platform builds in CI ([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) `build-and-release`); the maintainer does not need access to per-platform development hardware.
- **The auto-updater is built in.** Tauri's signing key on release tags produces an `updater.json` that the application reads to discover and apply updates. The user does not need to manually re-download new versions.

### Negative

- **The Tauri ecosystem is younger.** A library or feature that exists in Electron but not in Tauri occasionally requires writing the bridge ourselves. The notification-permission gate during Pass 7 was an example: tauri-plugin-notification did not (at the time) handle macOS permission prompts uniformly, so the application's `osIntegration.ts` wraps the plugin with browser-mode-safe fallbacks.
- **The WebView is the host platform's WebView.** This means the application runs in WebKit on macOS, WebKitGTK on Linux, and WebView2 on Windows. Behaviour differences across these are real (e.g. clipboard shortcuts behave subtly differently). The frontend tests against all three browsers in CI to catch the differences early.
- **Installer signing is updater-only at present.** Builds are signed with the Tauri auto-updater key, not with OS-level code-signing certificates (Apple Developer ID, Windows Authenticode). macOS Gatekeeper and Windows SmartScreen will display a first-launch warning. This is a known limitation, documented in the README and in [`docs/threat-model.md`](../threat-model.md) T4 row 4. Adding OS-level signing is a future-work item.
- **WebKitGTK on Linux requires system libraries.** A user on Linux installs `libwebkit2gtk-4.1-dev` (or the runtime equivalent) via their package manager. On Ubuntu 22.04+ this is straightforward; on older or unusual distributions it can be a friction point.

### Neutral

- **The choice is reversible at significant cost.** Switching to Electron later would require re-writing the backend in Node and re-validating the security posture; switching to native would require three platform-specific UI re-implementations. Neither is realistic at this point in the project's life. The decision is, in practice, durable.

## Alternatives considered

**(A) Electron.** The default choice for cross-platform desktop applications. Rejected for the binary-size and memory-footprint reasons above. The Rust-backend argument applies to Electron + a separate sidecar process, but combining Electron with Rust adds operational surface (managing the sidecar's lifecycle from Electron) without recovering the binary-size savings.

**(B) Native (three UIs in three toolkits).** Cocoa on macOS, GTK or Qt on Linux, WinUI on Windows. Rejected because the implementation cost is roughly tripled and the benefit (best per-platform integration) is marginal for a UI that is mostly information-display and form-rendering.

**(C) Web-only (perimeter is run from a terminal; UI is a website).** The user installs the four containers manually (or with a small shell script) and visits a website to observe state. Rejected because the gap between "user runs `compose up` from a terminal and reads a YAML file" and "user clicks through a wizard and sees a friendly status hero" is the application's primary value. A website cannot own the perimeter's lifecycle on the user's machine, cannot perform startup verification, and cannot react to OS-level signals.

**(D) Flutter.** Cross-platform UI framework with native compilation. Rejected because the desktop-application support was less mature than the web/mobile support at the time of decision (early 2026), and because the Flutter ecosystem is centred on Google rather than the open-source Rust community the project otherwise leans on.

**(E) Wails (Go + WebView).** Conceptually similar to Tauri but with Go instead of Rust. Rejected because the security-critical backend benefits more from Rust's type system than from Go's; the rest of the perimeter (mitmproxy in Python, scripts in Bash) is comfortable with Rust as the native-application layer.

**(F) Pure web app served from `vault-proxy`.** The proxy already runs in the perimeter and could serve a UI on a localhost port. Rejected because the lifecycle ownership problem (the perimeter's parent process needs to *be* an OS process, not be hosted *by* the perimeter) cannot be solved by an in-perimeter UI.

## References

- Tauri framework: [tauri.app](https://tauri.app)
- The Cargo configuration: [`app/src-tauri/Cargo.toml`](../../app/src-tauri/Cargo.toml)
- The Tauri configuration: [`app/src-tauri/tauri.conf.json`](../../app/src-tauri/tauri.conf.json)
- The frontend stack: [`app/package.json`](../../app/package.json) (React 18, TypeScript, Vite)
- The build pipeline: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) (`build-and-release` matrix)
- Whitepaper: [`docs/whitepaper.md`](../whitepaper.md) §7 (implementation paragraph that frames the Tauri choice)
- Threat-model row referencing the installer-signing limitation: [`docs/threat-model.md`](../threat-model.md) T4 (out-of-scope but documented)
