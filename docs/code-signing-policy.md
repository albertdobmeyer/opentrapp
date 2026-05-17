# Code Signing Policy — OpenTrApp

This document describes how the OpenTrApp project signs release artifacts and who is authorized to initiate a signing request. It exists to satisfy the [SignPath Foundation](https://signpath.org) open-source program requirements.

## What we sign

Windows release artifacts produced by the CI pipeline:

| Artifact | Format | Platform |
|----------|--------|----------|
| `OpenTrApp_<version>_x64_en-US.msi` | MSI installer | Windows x64 |
| `OpenTrApp_<version>_x64-setup.exe` | NSIS setup executable | Windows x64 |

macOS and Linux artifacts are not signed through SignPath (macOS signing requires Apple Developer Program; Linux packages are self-verified by package managers).

## When we sign

Signing occurs only on tag pushes matching `refs/tags/v*` in the GitHub Actions CI pipeline (`.github/workflows/ci.yml`). Artifacts built from pull requests or branch pushes are never submitted for signing.

## Who is authorized

Only the `build-and-release` job in the GitHub Actions workflow may submit signing requests. The workflow runs exclusively on GitHub-hosted runners (`windows-latest`). No developer or contributor can trigger a signing request manually outside this pipeline.

The repository maintainer is **Albert Dobmeyer** (GitHub: [@albertdobmeyer](https://github.com/albertdobmeyer)).

## Build reproducibility

All release builds:

- Run on GitHub-hosted runners (not self-hosted)
- Use pinned action SHAs (e.g., `actions/checkout@<sha>`, `dtolnay/rust-toolchain@<sha>`)
- Produce a SLSA Build Level 2 build-provenance attestation via `actions/attest-build-provenance`
- Produce a CycloneDX SBOM via `anchore/sbom-action`
- Are signed with cosign keyless signatures via Sigstore for supply-chain verification

All of the above are attached to each GitHub Release as downloadable assets.

## Source code

- Repository: https://github.com/albertdobmeyer/opentrapp
- License: MIT (see `LICENSE`)
- Artifact configuration: `.github/workflows/ci.yml`, `app/src-tauri/tauri.conf.json`

## Change process

Any change to this policy requires a pull request reviewed and merged by the repository maintainer. Changes take effect on the next tagged release after the policy update is merged.
