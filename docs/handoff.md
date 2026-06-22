# Handoff (session-state)

**Last updated: 2026-06-22.** The roadmap is [`ROADMAP.md`](../ROADMAP.md); the operating bar is [`CLAUDE.md`](../CLAUDE.md) §12. This doc is "where we stopped and the immediate next steps."

## The frame for next session

This session course-corrected from a visibility push back to the **security foundation** and ratified the bar (CLAUDE.md §12): **end-user-faithful tests only** (drive the product daemon `opentrapp-daemon vault <verb>`, never `make perimeter-up` / `podman-compose`), **root-cause fixes, no glossing, protect the user, substance before visibility.** We then started **Section 1** of the roadmap (the security foundation). Finish Section 1 first; everything in Track B (opencode pitch, awesome lists) waits on it.

## Verified this session

- **T0 boundary holds, cold == resumed**, via the perimeter (dev bring-up): `pass=7 fail=0`, CA unchanged. The boundary contains the agent. NOT yet proven via the product daemon (see below).
- **#76 daemon image-staging fix** (PR #149, merged on main `2124de5`): `supervisor::perimeter_up` now calls `fetch_perimeter_images()` before `podman load`, fail-closing if staging fails. Verified at the wiring level: the daemon now attempts the download and fail-closes on a 404, instead of dying at "tar not found".
- **The v0.7.2-rc2 release bundle is digest-consistent** (verified on the actual draft artifact): `vault-egress.tar` `podman load`s to the digest the overlay recorded, and the runtime `BundleVerifier` check (`podman image exists repo@digest`) passes. The rc2-style mismatch is NOT present. The release pipeline (podman build / push / save-by-tag) produces a runtime-verifiable signed bundle.

## The v0.7.2-rc2 release: DRAFT, deliberately NOT published

- We cut `v0.7.2-rc2` (from main, includes the #76 fix) to produce a signed bundle for verifying the daemon T0 end-to-end. CI built it clean (all gates, image build, multi-platform installers).
- **DECISION (do not reverse without a reason): do NOT publish the product release yet.** The foundation (product-daemon T0 end-to-end, T1/T2, #75) is unverified, so per the bar we do not ship or claim. The draft is a verified, ready-to-publish-later artifact.
- Public side effects of cutting the tag (normal pre-release artifacts, low-stakes): the tag `v0.7.2-rc2`, the GHCR images at `:v0.7.2-rc2`, and the `opentrapp-core` crate-publish lane ran (a library crate; confirm whether it actually published 0.7.2 or no-op'd, low-stakes). The installers / release page stay a private draft.
- If a clean slate is wanted, the draft + tag + GHCR images can be deleted without losing the verification knowledge.

## Section 1 remaining (finish FIRST)

1. **Product-daemon T0 end-to-end.** `opentrapp-daemon vault up` needs the signed tars to download. The debug binary (current, has #76) is at `app/src-tauri/target/debug/opentrapp-daemon`; `make daemon` builds it. Two ways:
   - **(a)** Publish a release when genuinely ready, then `vault up` downloads from it, then `vault verify` cold + resumed. The faithful path; do NOT publish prematurely.
   - **(b) Interim:** download the rc2 draft's tars into `~/.opentrapp/perimeter/images/` and stage the rc2 overlay, so the daemon's fetch finds them present (idempotent) and proceeds to load + verify + run + `vault verify`. Tests the BundleVerifier + bring-up + boundary; skips the HTTP download GET (already proven at the wiring level). Borderline "manual" per the bar; use only as an interim and state the skip.
2. **WS0-0a (T1): idle auto-pause FIRES.** Gated off (`IDLE_AUTO_PAUSE_ENABLED = false`). Read ADR-0018 for the INTENDED enable mechanism (a runtime flag/env, not a const hack); enable it properly, run the daemon, idle ~12 to 15 min, confirm dormant + RAM drop.
3. **WS0-0c (T2): wake exactly-once + security-correct resume.** From dormant, one message to `@opentrappbot` → wakes, replies once, and re-passes the boundary self-test.
4. **#75: credential `--env-file` hardening.** Independent. Move secrets off the `podman run` command line (host process-table window). Verify via the product path.

## Running the perimeter / T0 on this box (verified workable)

- The 7.2 GB laptop RUNS the full perimeter + T0 when cleaned of heavy apps (Cursor/Brave): ~3.6 GB free, no swap-storm. Images are pre-built (`podman images`).
- podman operations need `dangerouslyDisableSandbox`. Stop any running daemon (it holds a RunGuard) before re-running; tear down with `vault down` or by killing the pid.
- `podman-compose` verbose output ECHOES the API keys in plaintext: a dev-tool artifact, NOT the product. The shipped daemon uses the native orchestrator (no echo, redacted logging). **Rotate the two dev keys** (Anthropic + Telegram); they were echoed into a session transcript on 2026-06-22.

## Open tasks / state

- Tasks: **#75** (credential --env-file), **#76** (daemon image-staging: wiring done + verified, end-to-end pending a real published release). Plus the standing Section 2 to 6 items in ROADMAP.md.
- All session PRs merged: README skeptic-proof, the Skill Firewall `v1` tags + Marketplace projection (`opentrapp/skill-firewall`, live) + verified one-way sync, the dependency/migration merges, #76 fix, the bar + roadmap (CLAUDE.md §12 + ROADMAP.md), and the Scorecard token-permissions fix.
- Memory (auto-loaded) has: the bar, footprint-and-headless reality (the perimeter runs here cleaned; bloat is the Tauri GUI; daemon ~85% separated), the Skill Firewall projection, product identity, Scorecard solo ceiling, verify-the-consumption-end.

## Reminder of the bar (CLAUDE.md §12)

End-user-faithful tests only (the product daemon, not dev scaffolding). Root-cause fixes, no glossing or handwaving. Protect the user from agent dangers first. Substance before visibility.
