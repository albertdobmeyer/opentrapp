// Fuzz target — exercises the YAML→Manifest parser used to load every
// `component.yml` from a third-party submodule. The parser is the trust
// boundary between the orchestrator and untrusted component metadata; a
// panic, OOM, or stack overflow here is reachable by any contributor who
// adds a malicious manifest to a vendored component.
//
// Run locally:  cd app/src-tauri/fuzz && cargo fuzz run manifest_parse
// Run in CI:    .github/workflows/fuzz.yml runs each target for 60s/PR.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // We don't care whether the parse succeeds or fails — only that it
    // never panics, never overflows the stack, and returns within libFuzzer's
    // per-input timeout. `serde_yaml::Error` is a normal Result variant.
    let _ = lobster_trapp_lib::fuzz_api::parse_manifest(data);
});
