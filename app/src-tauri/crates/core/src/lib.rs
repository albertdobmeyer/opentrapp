//! opentrapp-core — the tauri-free orchestration core (Phase B, [ADR-0019]).
//!
//! The headless `opentrapp-daemon` links this crate and nothing GUI-shaped; the
//! Tauri viewer will (in a later slice) link it too, so the two processes share
//! one definition of the perimeter's durable state. Slice B1 lands the
//! cross-process **marker contract** — the durable source of truth between the
//! daemon (writer) and the viewer (reader). Later slices migrate the perimeter
//! lifecycle, watchdog, idle waker, and bootstrap into this crate.
//!
//! Invariant: **no `tauri`/`wry`/`webkit` dependency, ever** — verified in CI by
//! inspecting the daemon's dependency tree.
//!
//! [ADR-0019]: ../../../../docs/adr/0019-headless-daemon-gui-viewer-split.md

pub mod markers;
