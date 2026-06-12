//! opentrapp-core — the tauri-free orchestration core (Phase B, [ADR-0019]).
//!
//! The headless `opentrapp-daemon` links this crate and nothing GUI-shaped; the
//! Tauri viewer will (in a later slice) link it too, so the two processes share
//! one definition of the perimeter's durable state. Slice B1 landed the
//! cross-process **marker contract**; slice B2 migrates the orchestration core
//! itself — the perimeter spec, the podman lifecycle, manifest discovery, the
//! command runner, the workflow engine, the allowlist, `AppState`, and the idle
//! waker mechanism — all of which were already tauri-free. The Tauri viewer now
//! re-exports `orchestrator`/`util` from here, so the two processes share one
//! definition. Later slices give the daemon perimeter ownership + the socket.
//!
//! Invariant: **no `tauri`/`wry`/`webkit` dependency, ever** — verified in CI by
//! inspecting the daemon's dependency tree.
//!
//! [ADR-0019]: ../../../../docs/adr/0019-headless-daemon-gui-viewer-split.md

pub mod control;
pub mod idle;
pub mod markers;
pub mod orchestrator;
pub mod runguard;
pub mod supervisor;
pub mod util;
