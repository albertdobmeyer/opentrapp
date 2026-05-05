# Fuzzing harnesses

This crate hosts the [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) targets for the two trust boundaries inside the orchestrator that consume untrusted input:

| Target | Surface fuzzed | Why it matters |
|--------|----------------|----------------|
| `manifest_parse` | YAML → `Manifest` decoding via `serde_yaml::from_slice` | Manifests live inside third-party components. A panic or stack-overflow here is reachable by any contributor who adds a malicious `component.yml` to a vendored submodule. |
| `runner_interpolate` | Argument interpolation into a manifest-declared command template | The interpolator escapes user-supplied arguments. A quoting bypass would yield shell command injection; a panic would yield a denial-of-service on the GUI thread. |

Both targets call into a `fuzz_api` module exposed only when the parent crate is built with the `fuzzing` cargo feature. Production builds do not include the public surface.

## Local invocation

`cargo-fuzz` requires a nightly Rust toolchain.

```bash
rustup toolchain install nightly
cargo install cargo-fuzz

cd app/src-tauri/fuzz
cargo +nightly fuzz run manifest_parse        # runs forever; ctrl-C to stop
cargo +nightly fuzz run manifest_parse -- -max_total_time=60   # 60s budget

cargo +nightly fuzz run runner_interpolate
cargo +nightly fuzz run runner_interpolate -- -max_total_time=60
```

Findings (crashing inputs, panics, slow inputs) are written to `fuzz/artifacts/<target>/`. Each artifact is a single byte sequence that reproduces the issue; commit it to the repository as a regression seed and address the underlying defect.

## CI invocation

`.github/workflows/fuzz.yml` runs every target with a `-max_total_time=60` budget on every push to `main` and on every pull request. New crashes fail the workflow and surface in the PR's check list.

## Adding a new target

1. Add a Rust file under `fuzz_targets/<new_target>.rs` with a `fuzz_target!` macro body.
2. Register the binary in `Cargo.toml` (`[[bin]] name = "<new_target>", path = "fuzz_targets/<new_target>.rs"`).
3. Add a seed corpus directory at `corpus/<new_target>/` with one or more representative inputs.
4. Add a step to `.github/workflows/fuzz.yml`.
5. If the target reaches new internal code, expose the entry point through `fuzz_api` in the parent crate's `lib.rs` rather than making the underlying module public.
