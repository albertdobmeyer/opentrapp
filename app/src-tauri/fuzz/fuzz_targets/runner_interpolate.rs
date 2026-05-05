// Fuzz target — exercises the command-argument interpolator that
// substitutes user-supplied values into manifest-declared command
// templates. The interpolator wraps every argument in single quotes
// with shell-escape, so a successful exploit would either find a
// quoting bypass (yielding command injection) or a panic on
// pathological input.
//
// Input layout (libFuzzer arbitrary bytes):
//   first NUL-delimited field      → command template
//   subsequent NUL-delimited pairs → key, value, key, value, ...
// Anything malformed becomes the empty case; libFuzzer cares about
// crashes, not parse correctness.
//
// Run locally:  cd app/src-tauri/fuzz && cargo fuzz run runner_interpolate
// Run in CI:    .github/workflows/fuzz.yml runs each target for 60s/PR.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    // Split the input on NUL. The first segment is the template; pairs
    // of subsequent segments populate the args map. Bytes that are not
    // valid UTF-8 are dropped — the production interpolator only ever
    // receives `&str` from the JSON IPC layer.
    let mut parts = data.split(|b| *b == 0);
    let Some(template_bytes) = parts.next() else { return; };
    let Ok(template) = std::str::from_utf8(template_bytes) else { return; };

    let mut args: HashMap<String, String> = HashMap::new();
    while let (Some(k_bytes), Some(v_bytes)) = (parts.next(), parts.next()) {
        if let (Ok(k), Ok(v)) = (std::str::from_utf8(k_bytes), std::str::from_utf8(v_bytes)) {
            args.insert(k.to_owned(), v.to_owned());
        }
    }

    let _ = lobster_trapp_lib::fuzz_api::interpolate_args(template, &args);
});
