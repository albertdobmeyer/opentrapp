# OpenTrApp v0.3.2 — Release Notes

**Tagged:** _(pending — to be set at the v0.3.2 tag push)_
**Container baseline:** four-container perimeter as in v0.3.0 / v0.3.1; default Split Shell
**Target audience:** existing v0.3.1 users; non-technical end users
**Type:** patch — security and dependency updates, no user-facing application functionality changes

## Summary

A security-and-supply-chain patch release. No user-facing application functionality changes. Every shipped surface from v0.3.1 is preserved verbatim. The release closes ten transitive Rust security advisories, corrects the GitHub Actions SHA-pinning posture so the OpenSSF Scorecard awards full Pinned-Dependencies credit, and adds a `cargo-fuzz` harness over the orchestrator's two untrusted-input boundaries.

## Changes since v0.3.1

### Security

- **Ten RustSec vulnerabilities resolved.** `rustls-webpki` 0.103.9 → 0.103.13 (closes RUSTSEC-2026-0049, -0098, -0099, -0104), `tar` 0.4.44 → 0.4.45 (closes RUSTSEC-2026-0067, -0068), `time` 0.3.36 → 0.3.47 (closes RUSTSEC-2026-0009), and `reqwest` 0.11 → 0.12 to drop the `rustls 0.21` / `rustls-webpki 0.101.7` dependency path that re-introduced the rustls-webpki advisories. After this release `cargo audit` reports zero vulnerabilities.
- **Twenty-one unmaintained-but-not-vulnerable transitive advisories** are now documented with explicit `[advisories.ignore]` entries in [`app/src-tauri/deny.toml`](../app/src-tauri/deny.toml). Each entry records the upstream remediation path (Tauri/wry GTK4 migration; `idna` / `unicode-bidi` migration to `icu4x`) so the next maintainer who reads the policy file has the rationale for re-evaluation.
- **Fuzzing harness landed.** [`app/src-tauri/fuzz/`](../app/src-tauri/fuzz/) hosts two `cargo-fuzz` targets that run under `libFuzzer`: `manifest_parse` exercises the YAML→`Manifest` decoder used to load every third-party `component.yml`, and `runner_interpolate` exercises the command-argument interpolator that escapes user-supplied values into manifest-declared command templates. A new GitHub Actions workflow ([`.github/workflows/fuzz.yml`](../.github/workflows/fuzz.yml)) runs each target with a 60-second budget on every push and pull request.
- **Workflow secrets and tokens are scoped tighter.** `ci.yml` now declares a top-level `permissions: contents: read` block, with the `build-and-release` job explicitly opting up to `contents: write`, `id-token: write`, and `attestations: write` — the minimum required to publish release artefacts, obtain a sigstore OIDC token, and submit SLSA build provenance.

### Supply-chain integrity

- **GitHub Actions are pinned by commit SHA, not by tag.** Five action references in v0.3.1 were inadvertently pinned to the **tag-object** SHA rather than the commit the tag points to. GitHub Actions accepts both in `uses:`, but the OpenSSF Scorecard evaluates pinning against the action repository's commit history and rejects tag-object SHAs as "imposter commits". Corrected for `ossf/scorecard-action`, `Swatinem/rust-cache` (two occurrences), `tauri-apps/tauri-action`, `sigstore/cosign-installer`, `actions/attest-build-provenance`, and (in a follow-up correction) `github/codeql-action`. Pinned-Dependencies now scores 10/10.
- **Python dependencies are hash-pinned.** `pip install pyyaml` becomes `pip install --require-hashes -r tests/requirements.txt`. The new requirements file enumerates every `pyyaml 6.0.2` wheel and sdist hash from PyPI, with the regeneration procedure documented inline.
- **OpenSSF Scorecard workflow gains a `workflow_dispatch` trigger** so a maintainer can refresh the public score on demand via `gh workflow run scorecard.yml`. Previously the score updated only on push to `main`, weekly cron, or branch-protection-rule events.

### Documentation

- [`SCORECARD.md`](../SCORECARD.md) — the project's posture against each OpenSSF Scorecard check, distinguishing controls earned by the repository, controls pending a maintainer action, and controls that resolve with time.
- [`RELEASING.md`](../RELEASING.md) — auditable release procedure documenting versioning, the eight-step checklist, the cosign / SLSA verification commands end users can run on a downloaded artefact, and the yanked-release protocol.
- [`docs/scorecard-plan-2026-05-05.md`](scorecard-plan-2026-05-05.md) — per-check assessment, score arithmetic, and a sequenced action plan with expected score-point gains. Includes a CII Best Practices cheat-sheet that maps each questionnaire criterion to the in-repository file or fact that satisfies it.
- [`docs/handoff-code-quality-2026-05-05.md`](handoff-code-quality-2026-05-05.md) — scope and starting commands for the next code-quality session, including baseline-capture commands for `cargo clippy --pedantic`, ESLint complexity rules, `knip`, and `cargo machete`.

### Dependency updates

GitHub Actions: `actions/setup-python` 5 → 6, `actions/checkout` 4 → 6, `ossf/scorecard-action` 2.4.0 → 2.4.3.

Rust crates: `thiserror` 1 → 2, `tauri-plugin-updater` 2.10 → 2.10.1, `tokio` 1.49 → 1.50, `tauri` 2.10 → 2.11, `rustls-webpki` 0.103.9 → 0.103.13, `tar` 0.4.44 → 0.4.45, `time` 0.3.36 → 0.3.47, `reqwest` 0.11 → 0.12.

npm packages: production-deps and dev-deps groups updated. `vite 5 → 8`, `jsdom 25 → 29`, and `typescript 5 → 6` major-version proposals were declined and will be re-evaluated as smaller hops on Dependabot's next cycle.

### Bug fixes

None. v0.3.2 contains no functional defect fixes; it is a security and supply-chain release.

## What is _not_ in this release

- No user-facing application functionality changes. The wizard, dashboard, security monitor, and developer-mode shell behave identically to v0.3.1.
- No manifest-schema changes. Existing `component.yml` files are valid without modification.
- No container-perimeter topology changes. Compose files, network rules, and the four-container layout are unchanged.
- No installer-format changes. Linux `.deb` / `.rpm` / `.AppImage`, macOS `.dmg` (Apple Silicon and Intel), and Windows `.msi` / `.exe` are produced as in v0.3.1.

## Verification of release artefacts

Every release artefact ships with three companion files in the same GitHub Release: a CycloneDX SBOM (`*.cyclonedx.json`), a sigstore signature (`*.sig`), and a sigstore certificate (`*.pem`). Independent third-party verification commands are documented in [`RELEASING.md`](../RELEASING.md) §"Verifying a published release".

## Commit range

`vPREV..vNEW`: to be filled in at tag time with `git log --oneline v0.3.1..v0.3.2`.
