# OpenTrApp roadmap

What is left to build, ordered by the bar in [`CLAUDE.md`](CLAUDE.md) §12: **security correctness first, the lean foundation next, visibility last.** Status reflects what is actually verified per §11, not what merely exists.

Legend: ✅ done and verified · 🔶 in progress or needs end-user-faithful re-verification · ⬜ not started.

Detailed specs: [`docs/specs/2026-06-09-lean-verified-perimeter-roadmap.md`](docs/specs/2026-06-09-lean-verified-perimeter-roadmap.md) (WS0 to WS5), [`docs/road-to-recommendable.md`](docs/road-to-recommendable.md), the decisions in [`docs/adr/`](docs/adr/).

---

## The benchmark ladder (multi-session)

The north star: OpenTrApp becomes the perimeter people trust to run open-source agents, earning thousands of stars and forks **because it is the best**, not through promotion ([`CLAUDE.md`](CLAUDE.md) §12.4: substance first, visibility follows). Four rungs, each a measurable benchmark that gates the next. We do not climb a rung before the one below it is verified through the product's own entrypoints (§11). Sections 1 to 7 below are the tactical detail for each rung.

**Rung 1: Provably contained (security correctness).**
*Benchmark:* the perimeter provably contains the agent, verified through the product CLI (`opentrapp-daemon vault up` / `vault verify`), cold and after every resume.
- T0 boundary self-test exit 0 cold AND resumed, end-user-faithful (not dev scaffolding).
- T1 idle auto-pause fires; T2 wake exactly-once with a security-correct resume.
- Red-team breakout and proxy soak green.
*Gate:* all green via the product path. (Section 1; tasks #35, #40, #76.) **This is the current frontier.**

**Rung 2: Lean and CLI-first (the de-Tauri product).**
*Benchmark:* the three concerns are CLI-operable, the GUI is optional, and the full perimeter runs on a 7.2 GB laptop with idle auto-pause collapsing resting RAM toward zero.
- The de-Tauri cutover (the daemon is the product; the GUI an optional projection). ✅ **shipped** (clears Vulnerabilities #46 at the code level, pending Scorecard re-scan).
- Each of Vault / Skill / Social independently CLI-operable and distributable.
- Leanness verified on the 7.2 GB box (resting RSS; idle auto-pause firing). The 7.2 GB floor is non-negotiable.
*Gate:* de-Tauri done and lean verified on the laptop. (Section 2.) **Cutover + lean-down (WS-A…WS-D) DONE and box-verified; the enablement defaults (idle-pause-on, daemon-default-on) and the goproxy live-boundary gate remain — they sit under Rung 1.** **Release-critical under the hard-gate.**

**Rung 3: Best-in-class posture (the trust signals).**
*Benchmark:* the posture is verifiably best-in-class: clean Scorecard on the trusted-tier items, CII Gold, signed reproducible releases, a second maintainer.
- Scorecard Tier-1/2 alerts cleared (#46 via Rung 2; #43 and #1 via a co-maintainer).
- CII Best Practices Gold; signed releases (SignPath); reproducible builds.
- Co-maintainer recruited (also unlocks the people-gated checks).
*Gate:* the trust signals are real and verifiable; the release hard-gate (owner 2026-06-22) binds here. (Sections 4 and 5.)

**Rung 4: Adopted (the stars follow).**
*Benchmark:* OpenTrApp is the referenced perimeter for open-source agents; stars and forks in the thousands, earned.
- The opencode institutional reference (highest-leverage lever; gated on Rung 1).
- Public launch: the CDR / supply-chain story (Show HN, r/netsec), the evolution article, the Skill Firewall on the Marketplace.
- Awesome-lists, community contributions, the standalone-module adoption path.
*Gate:* every visibility push is scoped to the substance behind it (§12.4); never promote ahead of the verified foundation. (Section 6.)

---

## 1. Security correctness (the foundation, highest priority)

The perimeter must provably contain the agent, cold and after every resume. This is the load-bearing gate for the opencode pitch and the proof the security promise is real.

| Item | Status | Notes and gate |
|---|---|---|
| T0 boundary self-test, cold == resumed | ✅ | **Verified via the product CLI 2026-06-26 on the live goproxy perimeter** (not dev scaffolding): `opentrapp-daemon vault up` → `vault verify` `pass=7 fail=0` cold, then `vault pause`→`resume`→`vault verify` `pass=7 fail=0` with **B5 CA-fingerprint UNCHANGED**. End-user-faithful (the product daemon's native orchestrator, not `podman-compose`). Scope: DevVerifier/from-source mode + a placeholder key (B1–B6 need no valid key — P1-1); the BundleVerifier digest-staging path is the post-release T0 (gated on P4). |
| WS0-0a: idle auto-pause actually fires (T1) | ⬜ | **Always-on, token-gated** — the `IDLE_AUTO_PAUSE_ENABLED` const was removed (2026-06-09); the supervisor auto-pauses whenever a Telegram token is present (`should_auto_pause`; fail-safe — never pauses without a wake path). The gap is **verification, not enablement**: run the daemon with a token, idle past the threshold, confirm dormant + RAM drop (a short-threshold knob makes this fast). |
| WS0-0c: wake exactly-once + security-correct resume (T2) | ⬜ | One message wakes it, delivers exactly once, and the resumed perimeter re-passes T0. |
| Credential hardening: `--env-file` / podman secrets (#75) | ⬜ | Keys are inline `-e` on the `podman run` line, visible in the host process table for the ~1s startup window (same-user-local). Close it; verify via the product path. |
| WS1: bound proxy memory over long sessions | ✅ | **Resolved by replacement (WS-C).** The mitmproxy RSS leak (54→550+ MB) is gone — `vault-proxy` is now the Go `goproxy` chokepoint (<50 MB RSS measured, leak-free; ADR-0026, Section 2). The 256 MB `mem_limit` is the blast-radius cap, not a working set. |

## 2. Lean foundation / de-Tauri endgame (GUI optional, CLI-first)

The north star (ADR-0019 / ADR-0020 / ADR-0022): a lean headless daemon + CLI as the product, the GUI an optional projection. **The lean-down campaign (WS-A…WS-D) shipped on `main` (2026-06): the Tauri GUI is deleted, the proxy is a 15 MB Go chokepoint, and every base image is alpine. The cutover/lean parts are done and verified on the 7.2 GB box; the *enablement* parts (idle-auto-pause-on, daemon-default-on) stay gated on Rung 1 product-daemon T0.**

| Item | Status | Notes and gate |
|---|---|---|
| Headless daemon operation (`make daemon`, `docs/headless.md`) | ✅ | Lean GUI-free binary + operating docs shipped. |
| Delete the Tauri crate + GTK3 deps (the cutover) | ✅ | **Shipped on `main` 2026-06-24** (WS-D). Workspace re-rooted at `crates/{core,daemon,viewer-server}`; `Cargo.lock` has 0 `tauri`/`wry`/`webkit` entries; `deny.toml` ignore list empty. Clears Vulnerabilities #46 at the code level (Scorecard re-scan pending). |
| Loopback-viewer de-risking spike + threat model | ✅ | ADR-0022 §0 passed; `viewer-server` built, session-bootstrap + events WS wired, Linux-proven this campaign. |
| WS-A: tune `compose.yml`/`perimeter.yml` mem_limits to measured + SIGTERM teardown | ✅ | Resting-cap sum 6.3 GB → ~3 GB; clean idle shutdown (orchestrator-check §31). |
| WS-B: lean every workload base to alpine | ✅ | **Shipped 2026-06-26** (PR #191). vault-skills 233→72 MB, vault-social 153→74 MB; vault-egress + goproxy already alpine; vault-agent node:22-alpine (distroless non-viable, #87). Verified on musl (self-test/test/scan green; PyYAML musllinux wheel). |
| WS-C / WS5: replace mitmproxy with a lean Go L7 proxy | ✅ | **Built + switched in on `main`** (PR #189/#190; ADR-0026). `elazarl/goproxy`, 15.6 MB, leak-free, drops the Python interpreter from the keys-holding container. **Live-boundary gate GREEN (2026-06-26):** the live `boundary-selftest.sh` ran via `opentrapp-daemon vault verify` against the goproxy perimeter — B1 isolation, B2 allowlist (deny 403 / allow), **B3 credential-separation (goproxy injects; no key in agent)**, B4 L3 egress, B5 CA-stable cold==resumed, B6 read-only — `pass=7` cold and resumed. A placeholder key suffices for the boundary (P1-1). |
| Enable idle auto-pause by default | ⬜ | After WS0-0a/0c verified (Rung 1). |
| Flip the daemon default on (`OPENTRAPP_DAEMON_DEFER`) | ⬜ | After resting-RSS + viewer-survival verified on the product path. |
| Status-streaming API (Unix socket or loopback) | ⬜ | Today the CLI reads markers + stderr. |
| Unify the CLI as `opentrapp` (retire `opentrapp-daemon`) | ⬜ | Phase 3. |
| OS autostart launches the daemon, not the GUI | ⬜ | systemd user unit / launchd / Windows task. |

## 3. Supply-chain and dependency hygiene

| Item | Status | Notes |
|---|---|---|
| ESLint 9 to 10 migration (unblocks unicorn 66, #107) | ⬜ | Blocked upstream on eslint-plugin-react ESLint-10 support; `@dependabot ignore` posted. Revisit when upstream ships it. |
| Dependabot majors (react-router v7, form-data) | ✅ | Merged. New ones handled per the bar: verify, then merge. |

## 4. Security posture / Scorecard / best practices

| Item | Status | Notes |
|---|---|---|
| Token-Permissions least privilege | ✅ | PR #139. |
| Dependabot #19 (js-yaml DoS) | ✅ | Overridden to js-yaml ≥4.2.0; `npm install` reports 0 vulnerabilities (PR #152). |
| Pinned-Dependencies #80/#81/#82 | 🔶 | `npm install -g pkg@VERSION` in the agent/skills build scripts: version-pinned, but Scorecard wants integrity pins. Hash-pin or document the rationale (the built image is digest-pinned downstream by the signed bundle). |
| Vulnerabilities #46 | 🔶 | **De-Tauri cutover shipped (Section 2): the GUI crate is deleted, `Cargo.lock` is GTK-free.** The 19 GTK3/WebKit advisories are gone at the code level; the Scorecard badge clears on its next re-scan cycle. |
| Code-Review #43 | ⬜ | Solo-maintainer ceiling: needs a second human reviewer. |
| Branch-Protection #1 | 🔶 | Partly settable now via `gh api` (require status checks, no force-push, require PR); a required-review count ≥1 is meaningless without a co-maintainer. |
| CII Best Practices + Scorecard climb | 🔶 | Several checks unlock with a co-maintainer. |

## 5. Distribution and packaging (ADR-0023)

| Item | Status | Notes |
|---|---|---|
| Skill Firewall on the Marketplace + one-way sync | ✅ | Action published; projection sync verified end to end. |
| Publish the de-Tauri release (next: v0.9.0) | ⛔ | **HARD-GATED (owner decision 2026-06-22): no release until every code-scan alert is closed.** Progress: #80-82 done; **#46 de-Tauri cutover shipped** (clears on the next Scorecard re-scan). **Still blocking:** #43 + #1 (co-maintainer), WS0/Rung-1 product-daemon T0, and the goproxy **live-boundary self-test** (Section 2). The lean-down work is built + box-verified and ready to tag; the publish stays gated. The last tagged release ([v0.8.0](https://github.com/albertdobmeyer/opentrapp/releases/latest)) is still the pre-cutover Tauri desktop app. |
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

**Release hard-gate (owner, 2026-06-22):** no new version ships until *all* code-scan alerts are closed. Because #46 needs the de-Tauri cutover (Section 2) and #43/#1 need a co-maintainer (Section 6), those are now on the **release critical path**, not deferrable if a release is wanted. The fixable alerts (#80-82, #1 settings) are being closed now; #19 is done.
