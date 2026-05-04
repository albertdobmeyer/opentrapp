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

## Future ADRs

The post-launch roadmap ([`../roadmap-post-launch.md`](../roadmap-post-launch.md) §3) lists ADRs queued for future writing:

- Parking `moltbook-pioneer` following Meta's acquisition of Moltbook (2026-03-10)
- The 2026-05-02 vision recheck and the "deserve-to-exist" scope test
- Four-container compose vs. single-container vs. VM-level isolation
- The manifest-driven generic backend (no component-specific logic in Rust or React)
- The choice of Tauri 2 over Electron, native, and web-only alternatives
