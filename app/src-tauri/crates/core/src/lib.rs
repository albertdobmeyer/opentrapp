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

pub mod config_ops;
pub mod control;
pub mod credentials;
pub mod diagnostics;
pub mod execute;
pub mod health;
pub mod idle;
pub mod markers;
pub mod orchestrator;
pub mod prerequisites;
pub mod runguard;
pub mod selftest;
pub mod sentinel;
pub mod status;
pub mod supervisor;
pub mod telegram;
pub mod util;
pub mod workflow_ops;

/// Thin wrappers over the parsing / interpolation / redaction functions the
/// fuzz harness drives (`fuzz/fuzz_targets/*`). Lives here in the tauri-free
/// core (not the GUI crate) so the fuzz build never compiles `tauri-build` —
/// building the GUI crate under cargo-fuzz fails its `build.rs`. Gated on the
/// `fuzzing` feature so it costs nothing in normal builds.
#[cfg(feature = "fuzzing")]
pub mod fuzz_api {
    use std::collections::HashMap;

    /// Parse a YAML byte slice as a `component.yml` manifest. Mirrors the
    /// production parser invoked by `orchestrator::discovery`.
    pub fn parse_manifest(
        input: &[u8],
    ) -> Result<crate::orchestrator::manifest::Manifest, serde_yaml::Error> {
        serde_yaml::from_slice(input)
    }

    /// Interpolate user-supplied arguments into a manifest-declared command
    /// template. Mirrors the production path in `orchestrator::runner`.
    pub fn interpolate_args(command: &str, args: &HashMap<String, String>) -> String {
        crate::orchestrator::runner::interpolate_args_for_test(command, args)
    }

    /// Redact known token-bearing environment variables from a string. The
    /// production caller is the perimeter stderr logger — failure modes worth
    /// surfacing are panics, infinite loops, or under-redaction.
    pub fn redact_secrets(s: &str) -> String {
        crate::util::secrets::redact_secrets(s)
    }
}
