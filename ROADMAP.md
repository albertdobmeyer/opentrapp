# OpenTrApp roadmap

What is left to build, ordered by the bar in [`CLAUDE.md`](CLAUDE.md) §12: **security correctness first, the lean foundation next, visibility last.** Status reflects what is actually verified per §11, not what merely exists.

Legend: ✅ done and verified · 🔶 in progress or needs end-user-faithful re-verification · ⬜ not started.

Detailed specs: [`docs/specs/2026-06-09-lean-verified-perimeter-roadmap.md`](docs/specs/2026-06-09-lean-verified-perimeter-roadmap.md) (WS0 to WS5), [`docs/road-to-recommendable.md`](docs/road-to-recommendable.md), the decisions in [`docs/adr/`](docs/adr/).

---

## 1. Security correctness (the foundation, highest priority)

The perimeter must provably contain the agent, cold and after every resume. This is the load-bearing gate for the opencode pitch and the proof the security promise is real.

| Item | Status | Notes and gate |
|---|---|---|
| T0 boundary self-test, cold == resumed | 🔶 | Held 2026-06-22 via the dev path (`pass=7` cold and resumed, CA unchanged). Re-run **through the product CLI** (`opentrapp-daemon vault up` + `vault verify`) to make it end-user-faithful (bar §1). |
| WS0-0a: idle auto-pause actually fires (T1) | ⬜ | Built but gated off (`IDLE_AUTO_PAUSE_ENABLED=false`). Enable the proper way (not a const hack), run the daemon, confirm dormant + RAM drop. |
| WS0-0c: wake exactly-once + security-correct resume (T2) | ⬜ | One message wakes it, delivers exactly once, and the resumed perimeter re-passes T0. |
| Credential hardening: `--env-file` / podman secrets (#75) | ⬜ | Keys are inline `-e` on the `podman run` line, visible in the host process table for the ~1s startup window (same-user-local). Close it; verify via the product path. |
| WS1: bound proxy memory over long sessions | 🔶 | Measure mitmproxy RSS over load and time; if it climbs, apply the measured fix and confirm bounded. |

## 2. Lean foundation / de-Tauri endgame (GUI optional, CLI-first)

The north star (ADR-0019 / ADR-0020 / ADR-0022): a lean headless daemon + CLI as the product, the GUI an optional projection. About 85% built; this is the last mile. The cutover also clears the top code-scan alert.

| Item | Status | Notes and gate |
|---|---|---|
| Headless daemon operation (`make daemon`, `docs/headless.md`) | ✅ | Lean GUI-free binary + operating docs shipped. |
| Enable idle auto-pause by default | ⬜ | After WS0-0a/0c verified. |
| Flip the daemon default on (`OPENTRAPP_DAEMON_DEFER`) | ⬜ | After resting-RSS + viewer-survival verified on the product path. |
| Loopback-viewer de-risking spike + threat model | ⬜ | ADR-0022 §0 kill criteria; `crates/viewer-server` is scaffolded but excluded from the build. |
| Status-streaming API (Unix socket or loopback) | ⬜ | Today the CLI reads markers + stderr. |
| Unify the CLI as `opentrapp` (retire `opentrapp-daemon`) | ⬜ | Phase 3. |
| OS autostart launches the daemon, not the GUI | ⬜ | systemd user unit / launchd / Windows task. |
| Delete the Tauri crate + GTK3 deps | ⬜ | The cutover; clears Vulnerabilities #46 (the GTK3/WebKit advisories). |
| WS5: replace mitmproxy with a lean Rust L7 proxy | ⬜ | Drop the Python interpreter from the keys-holding container. |

## 3. Supply-chain and dependency hygiene

| Item | Status | Notes |
|---|---|---|
| ESLint 9 to 10 migration (unblocks unicorn 66, #107) | ⬜ | Blocked upstream on eslint-plugin-react ESLint-10 support; `@dependabot ignore` posted. Revisit when upstream ships it. |
| Dependabot majors (react-router v7, form-data) | ✅ | Merged. New ones handled per the bar: verify, then merge. |

## 4. Security posture / Scorecard / best practices

| Item | Status | Notes |
|---|---|---|
| Token-Permissions least privilege | ✅ | PR #139. |
| Pinned-Dependencies | 🔶 | Version-pinned; re-run Scorecard to refresh; consider hash pins. |
| Vulnerabilities (#46) | ⬜ | De-Tauri-gated (Section 2). |
| Code-Review + Branch-Protection (required review, signed commits) | ⬜ | The solo-maintainer ceiling: needs a second reviewer / org settings. |
| CII Best Practices + Scorecard climb | 🔶 | Several checks unlock with a co-maintainer. |

## 5. Distribution and packaging (ADR-0023)

| Item | Status | Notes |
|---|---|---|
| Skill Firewall on the Marketplace + one-way sync | ✅ | Action published; projection sync verified end to end. |
| Publish v0.7.x | ⬜ | Gated on WS0 verified; claims scoped to what is proven. |
| CLI/registry distribution (crates.io / Homebrew / curl-sh; GHCR) | ⬜ | OS-agnostic, no lock-in. |
| Code signing (SignPath reapply) | ⬜ | After visibility signals land (rejected 2026-06-15). |

## 6. Visibility and adoption (Track B, AFTER the foundation)

Substance first, visibility follows. The opencode reference is the highest-leverage lever and is gated on the security foundation in Section 1.

| Item | Status | Notes and gate |
|---|---|---|
| opencode pitch (institutional reference) | ⬜ | Gated on T0 + T1/T2 verified end-user-faithfully. |
| Awesome-list submissions (4 lists) | 🔶 | Drafted and turnkey in `drafts/`; owner submits. |
| Co-maintainer recruitment | ⬜ | Also unlocks the Scorecard/CII ceiling. |
| Evolution article (albertdobmeyer.dev) | 🔶 | Drafted and deployed; diagram SVGs in-repo. |
| Show HN / r/netsec / r/selfhosted | 🔶 | Drafted in `drafts/`. |

## 7. Housekeeping / owner-only

| Item | Status |
|---|---|
| Rotate the two dev keys (echoed into a session transcript 2026-06-22 by podman-compose) | ⬜ |
| Marketplace publish + `SKILL_FIREWALL_SYNC_TOKEN` | ✅ |

---

**The single most important next move is to finish Section 1** through the product's own entrypoints. Everything in Section 6 waits on it.
