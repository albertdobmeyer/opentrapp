//! Diagnostic bundle generation — a thin Tauri shim over `opentrapp_core::diagnostics`.
//!
//! The collection logic and the security-critical redaction (and its unit tests) were lifted
//! into `opentrapp-core` (ADR-0022 migration step 1) so the on-demand loopback web GUI route
//! calls the exact same transport-neutral fn — no duplicate (CLAUDE.md §5). This shim only
//! injects the timestamp (chrono) and the app version, then delegates.

/// Returns a freshly generated, redacted diagnostic bundle as a single string.
#[tauri::command]
pub async fn generate_diagnostic_bundle() -> Result<String, String> {
    let generated_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    opentrapp_core::diagnostics::generate_bundle(&generated_at, env!("CARGO_PKG_VERSION"))
}
