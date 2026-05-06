// Fuzz target — exercises the secret-redaction filter that runs on
// subprocess stderr before it's written to the app's log file. The
// production caller is `lifecycle::redact_secrets`; the function looks
// for `TELEGRAM_BOT_TOKEN=…`, `ANTHROPIC_API_KEY=…`, `OPENAI_API_KEY=…`
// substrings and replaces what follows with the literal "<redacted>"
// up to the next whitespace / quote.
//
// Failure modes worth catching:
//   - panic, OOM, or stack overflow on pathological input
//   - infinite loop (regression of the search_from-after-replace fix)
//   - mis-identified boundary letting a real token escape (unicode
//     whitespace edge cases, quoting tricks)
//
// The harness can't easily assert "no real token leaked" without a
// reference implementation; libFuzzer's contract here is the universal
// one — the function must return without panic for every input.
//
// Run locally:  cd app/src-tauri/fuzz && cargo fuzz run redact_secrets
// Run in CI:    .github/workflows/fuzz.yml runs each target for 60s/PR.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The redactor only ever receives valid UTF-8 in production (subprocess
    // stderr is decoded as String before this point). Drop invalid input.
    let Ok(s) = std::str::from_utf8(data) else { return; };
    let _ = lobster_trapp_lib::fuzz_api::redact_secrets(s);
});
