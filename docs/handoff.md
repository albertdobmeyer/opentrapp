# Handoff (session-state)

**Last updated: 2026-06-26.** The roadmap is [`ROADMAP.md`](../ROADMAP.md); the operating bar is [`CLAUDE.md`](../CLAUDE.md) §12. This doc is "where we stopped and the immediate next steps."

## The frame for next session

The **radical lean-down + USP-sharpening campaign** ("reel it in" — the plan in `.claude/plans/`, mapped to ROADMAP Rung 2) is **structurally complete on `main`**. The product is now what the north star always wanted: a lean headless **`opentrapp-daemon`** + CLI, with the GUI an optional on-demand browser projection. What remains before a release are the **Rung-1 security-correctness gates** (product-daemon T0 end-to-end) and a couple of **maintainer-controlled final gates** — not more building. Hold the bar (CLAUDE.md §12): end-user-faithful tests only, root-cause fixes, no glossing, protect the user, substance before visibility.

The method that carried the campaign — **pin-first**: for every security-relevant refactor we wrote characterization tests that had to be **green before AND after** (mutation-proven to bite), so containment could not silently drift. Keep using it.

## The lean-down campaign — what shipped (all on `main`, verified on the 7.2 GB box)

| WS | What | Result | Evidence |
|---|---|---|---|
| **A** | Tune `compose.yml`/`perimeter.yml` mem_limits to measured + SIGTERM teardown | Resting-cap sum 6.3 GB → ~3 GB; clean idle shutdown | `podman stats`; orchestrator-check §31 |
| **B** | Lean every workload base to alpine (PR #191) | vault-skills 233→**72 MB**, vault-social 153→**74 MB** | self-test 10/10, test 168/168 + 48/48, scan 0/25, PyYAML musllinux wheel — all on musl |
| **C** | Replace the leaky Python mitmproxy with a Go `goproxy` chokepoint (PR #189/#190; ADR-0026) | 250 MB→**15.6 MB**, 54→550 MB leak → **<50 MB flat** | proxy-level: off-allowlist **403 via real MITM under seccomp**, CA at agent-trusted path, 1 MB/10 MB caps, redaction; `go test ./...` green |
| **D** | Delete the Tauri/GTK GUI; re-root workspace at `crates/{core,daemon,viewer-server}` (shipped 2026-06-24) | ~220 MB resting + **19 GTK3 advisories gone**; `Cargo.lock` GTK-free; `deny.toml` ignore list empty | de-Tauri end-to-end Linux-proven (session-bootstrap → events WS → viewer) |
| **E** | Sharpen USPs + harmonize the public narrative to the lean reality | README/landing/docs de-Tauri-current; Skill Firewall is the standalone lean wedge | (ongoing — see "the public release" below) |

Net: every base image is alpine (or the 15.6 MB Go proxy / node:22-alpine agent), the Python interpreter is out of the keys-holding container, and the heaviest single thing in the old footprint (the ~442 MB WebKitGTK webview) is deleted.

## Rung-1 boundary + goproxy live gate — VERIFIED via the product daemon (2026-06-26)

The load-bearing security gate is **crossed**. On the 7.2 GB box (cleaned), through the **product daemon** (not dev scaffolding):
`opentrapp-daemon vault up` → `vault verify` → **`pass=7 fail=0` cold**, then `vault pause`→`resume`→`vault verify` → **`pass=7 fail=0` with B5 CA fingerprint UNCHANGED** → `vault down` (clean). B1 isolation · B2 allowlist (deny 403 / allow) · **B3 credential-separation (goproxy injects; no vendor key in `vault-agent`)** · B4 L3 egress · B5 CA-stable cold==resumed · B6 read-only. So **the #1 non-negotiable bar (zero-trust air-gap) is verified on the shipped goproxy stack**, AND **the goproxy live-boundary gate is green**. Scope (honest): DevVerifier/from-source mode + a **placeholder** key — the boundary checks need no valid key (P1-1/#96), so no real credential was handled. Repro: build the 3 always-on images tagged `ghcr.io/albertdobmeyer/opentrapp/<svc>:latest`, placeholder `~/.opentrapp/.env`, no overlay → DevVerifier.

## What is NOT done — the gates that stand between "built" and "released"

1. **The BundleVerifier digest-staging path (the *released*-product T0).** The run above used DevVerifier (from-source, no signed overlay) — faithful to how from-source runs today. The signed-overlay `podman load` + digest-pin path engages only with a real release overlay; it is naturally exercised at the **post-release T0** once the cargo-dist release lane lands (P4 / ADR-0023; #76).
2. **Retire `vault-proxy.py`** (P2-1) — now UNBLOCKED by the green live gate; remove the Python fallback + its embedded/pinned copies in a focused PR.
3. **Win/macOS browser-model runtime (maintainer hardware).** The de-Tauri browser model is **Linux-proven**; Windows/macOS are portable-by-construction (pure-Rust daemon + browser + a 3-line opener) but **runtime-unverified**. The cutover dropped the v0.8.0 Win/macOS desktop installers — a one-way product decision already taken.
4. **Co-maintainer-gated Scorecard items** (#43 Code-Review, #1 Branch-Protection) — a second human, not a code change.

> **Resolved 2026-06-27 (was item: "the 9 Go advisories"):** govulncheck found **24 reachable** stdlib advisories (more than the guessed 9) — all because the goproxy built on **go 1.23.0**. Fixed at the root: `go.mod` → `go 1.25.11` (the max "Fixed in"), `golang.org/x/net` → v0.56.0 (the one reachable module vuln, GO-2026-4918), and the Containerfile builder pinned to `golang:1.25.11-alpine@sha256:523c3eff…` (digest, closing the unpinned-builder gap). **govulncheck now clean**, `go test ./...` green, and the rebuilt 15.6 MB proxy re-passed the live boundary self-test on the box (`pass=7` cold **and** resumed, B5 CA unchanged). goproxy is **Tier-2 release-gating** ([`known-advisories.md`](known-advisories.md)) and is now clean. See [`known-advisories.md`](known-advisories.md) for the standing posture.

## The public release — consolidated status

- **Last tagged release: [v0.8.0](https://github.com/albertdobmeyer/opentrapp/releases/latest)** (2026-06-23) is the **pre-cutover Tauri desktop app**. `main` is a full product generation ahead of it (de-Tauri + goproxy + alpine).
- **The README Status block is the honest public face** and is current: de-Tauri shipped, runs from source, installers pending (cargo-dist, ADR-0023), v0.8.0 is still the old desktop app.
- **Next release would be v0.9.0** (the de-Tauri lean release). It is **built and box-verified** — and now the **Rung-1 product-path T0 + the goproxy live boundary gate are GREEN** (above), so the containment claim is earned. What still stands between here and a release: (a) **no release *lane* exists** — the de-Tauri cutover deleted the publish job; a `git tag` produces only GHCR images, no installers/binaries (P4 / cargo-dist / ADR-0023 — the keystone); (b) the **BundleVerifier digest-staging path** is verified only at the post-release T0; (c) **#43+#1 co-maintainer** Scorecard posture; (d) the 3 open code-scanning alerts are **Tier-3-exempt** (contained-agent npm pins — policy says never hash-pin those). **Do not `gh release create` without the maintainer's go/no-go** — outward-facing, and the lane must be built first. Scope any release copy to what is verified (§11).

## Running the perimeter / T0 on this box (verified workable)

- The 7.2 GB laptop RUNS the full perimeter + T0 when cleaned of heavy apps (Cursor/Brave): ~3.6 GB free, no swap-storm. Images are pre-built (`podman images`).
- podman operations need `dangerouslyDisableSandbox`; local builds need fully-qualified image names (`docker.io/library/…`) since there is no unqualified-search registry. Stop any running daemon (it holds a RunGuard) before re-running; tear down with `vault down` or by killing the pid.
- The dev keys (Anthropic + Telegram) were **rotated 2026-06-22** after `podman-compose` verbose output echoed them; the shipped daemon uses the native orchestrator (redacted logging, no echo). Never use `podman-compose` verbose with real keys.

## Open tasks / state

- Standing: **#35/#40** (Rung-1 T0 + wake exactly-once), **#76** (daemon image-staging end-to-end pending a published release). Plus the Section 2 enablement defaults (idle-pause-on, daemon-default-on) and the goproxy live gate.
- Memory (auto-loaded) carries: the bar, footprint-and-headless reality (now with the alpine numbers), de-Tauri handler-lift status, Skill Firewall projection, product identity, Scorecard solo ceiling, trust-tier triage, verify-the-consumption-end, frontend-needs-Node-22.

## Reminder of the bar (CLAUDE.md §12)

End-user-faithful tests only (the product daemon, not dev scaffolding). Root-cause fixes, no glossing or handwaving. Protect the user from agent dangers first. Substance before visibility.
