# Architecture Decision Records

This directory contains the project's Architecture Decision Records (ADRs). Each record documents a single architectural decision that has been made, the context in which it was made, the alternatives that were considered, and the consequences (positive and negative) that the decision produces.

Format follows the convention popularised by Michael Nygard and documented at [adr.github.io](https://adr.github.io/).

## Status definitions

- **Proposed** — under discussion; not yet implemented
- **Accepted** — implemented and currently load-bearing
- **Deprecated** — no longer reflects the current implementation; retained for historical context
- **Superseded by ADR-NNNN** — replaced by a later record

## When to write an ADR

Write an ADR when an architectural decision will:

- Be hard to reverse later (e.g. data formats, public APIs, schema choices, infrastructure dependencies)
- Be cited or questioned by future contributors who weren't in the room
- Encode a judgement call that the source code itself cannot self-document (e.g. *why* an approach was chosen over alternatives that would also have worked)

A bug-fix is not an ADR. A library version bump is not an ADR. A change in coding style is not an ADR. A change in *what the system does* or *how the system is structured* is.

## Index

| # | Status | Title |
|---|--------|-------|
| [0001](0001-proxy-side-api-key-injection.md) | Accepted | Proxy-side API-key injection |
| [0002](0002-adaptive-shell-levels.md) | Accepted | Adaptive shell levels (Hard / Split / Soft) as a system state |
| [0003](0003-content-disarm-reconstruction.md) | Accepted | Content Disarm and Reconstruction for skills |
| [0004](0004-parking-openagent-social.md) | Accepted | Parking openagent-social |
| [0005](0005-deserve-to-exist-scope-test.md) | Accepted | The "deserve-to-exist" scope test |
| [0006](0006-four-container-topology.md) | Accepted | Four-container compose topology |
| [0007](0007-manifest-driven-generic-backend.md) | Accepted | Manifest-driven generic backend |
| [0008](0008-tauri-over-electron.md) | Accepted | Tauri 2 over Electron, native, and web-only |
| [0009](0009-five-container-perimeter.md) | Accepted — implemented in v0.5.0 | From four-container perimeter to five: separating L7 and L3 egress policy |
| [0010](0010-pinned-resolver-dns.md) | Accepted — implemented in v0.5.0 | Pinned-resolver DNS as a perimeter primitive (companion to ADR-0009) |
| [0011](0011-zero-trust-self-sufficient-bootstrap.md) | Accepted — implemented in v0.5.0 | Zero-trust, self-sufficient first-launch bootstrap (native orchestrator + release-asset signed image delivery) |
| [0012](0012-subscription-oauth-auth-feasibility.md) | Proposed — research only | Subscription / OAuth authentication feasibility (can a Claude Pro/Max login replace the pasted API key without weakening the perimeter?) |
| [0013](0013-monorepo-consolidation.md) | Accepted — landed 2026-05-30 | Monorepo consolidation: collapse the three submodules into `workloads/` + `infra/` (no independent lifecycle; submodule tax for unused benefit) |
| [0014](0014-monorepo-modular-distribution.md) | Proposed — 2026-05-31 | Monorepo dev-home + modular distribution + `openagent-*` naming (install one shield standalone, or the GUI with a profile; extends 0013, does not revert it) |
| [0015](0015-local-ai-judgment-layer.md) | Accepted — M1 shipped | Local-AI judgment layer (Sentinel): escalation ladder rung 0–3, model-tiering principle, lib-first design, acceptable-on-host rationale |
| [0016](0016-host-mediated-allowlist-loosening.md) | Accepted — v0.6 Item A shipped | Host-mediated allowlist loosening (read+recommend via the judge; the human tap is the sole writer) |
| [0017](0017-unpark-social-live-adapter.md) | Accepted — v0.6 Item C shipped | Un-park the social shield behind a live protocol adapter (supersedes ADR-0004) |
| [0018](0018-idle-auto-pause-host-waker.md) | Accepted — design (Phase 3 Slice D) | Idle auto-pause and the host-side wake-on-message waker (peek-only getUpdates; ~0 resting RAM) |
| [0019](0019-headless-daemon-gui-viewer-split.md) | Accepted — implemented behind opt-in; unverified pending hardware | Headless daemon + on-demand GUI viewer split (the daemon is the product) |
| [0020](0020-product-identity-and-distribution.md) | Accepted — direction ratified; implementation staged (0021/0022) | Product identity & distribution: a CLI/daemon with projected viewers (CLI/GUI/MCP); not a desktop app; not MCP-for-the-contained-agent |
| [0021](0021-danger-gated-agentic-control-plane.md) | Proposed — must be accepted before any agentic control surface is built | Danger-gated agentic control plane: T7 (prompt-injected host operator), `boundary_impact` vs operational `danger`, out-of-band human gate for boundary-weakening (no agent call edge) |
| [0022](0022-daemon-control-surface.md) | Proposed — architecture + transport security decided; implementation spike-gated | Daemon control surface: CLI · on-demand loopback web GUI (de-Tauri) · optional MCP; the ephemeral loopback-server security model; C1 (wait for GTK4) ruled out |
| [0023](0023-distribution-and-packaging.md) | Proposed — strategy decided; registry publish can land early, installers with de-Tauri | Distribution & packaging: OS-agnostic, vendor-neutral; `cargo publish` opentrapp-core → crates.io (flips Scorecard Packaging) + cargo-dist prebuilt installers (all 3 OSes) + GHCR; no Homebrew/OS-store lock |

## Future ADRs

- **macOS/Windows runtime install** — `podman` is not present by default on those platforms; ADR-0011's bootstrap is Linux/AppImage-only so far. A future record should decide the Podman Desktop / Colima / WSL2 strategy.
