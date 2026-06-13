# Code Signing Policy — OpenTrApp

This document describes how the OpenTrApp project signs release artifacts and who is authorized to initiate a signing request. It exists to satisfy the [SignPath Foundation](https://signpath.org) open-source program requirements.

## What we sign

Windows release artifacts produced by the CI pipeline:

| Artifact | Format | Platform |
|----------|--------|----------|
| `OpenTrApp_<version>_x64_en-US.msi` | MSI installer | Windows x64 |
| `OpenTrApp_<version>_x64-setup.exe` | NSIS setup executable | Windows x64 |

macOS and Linux artifacts are not signed through SignPath (macOS signing requires Apple Developer Program; Linux packages are self-verified by package managers).

## macOS

macOS artifacts (`.app` / `.dmg`) are signed with an **Apple Developer ID Application** certificate and **notarized** with Apple, so Gatekeeper accepts them without a first-launch warning. The `APPLE_*` env block is a **ready-to-activate template** (commented out in the `build-and-release` job's `tauri-action` step). It is **not** wired live: `tauri` treats a *present-but-empty* `APPLE_CERTIFICATE` as "sign now" and fails the macOS bundle on an empty cert — so the env lines must be added only **once the secrets are actually populated**, not left passing empty values. Until then the pipeline is unchanged.

Required repository secrets (provision via Apple Developer Program → *Developer ID Application*):

| Secret | What it is |
|--------|------------|
| `APPLE_CERTIFICATE` | base64 of the `.p12` Developer ID Application cert |
| `APPLE_CERTIFICATE_PASSWORD` | password for that `.p12` |
| `APPLE_SIGNING_IDENTITY` | the identity string, e.g. `Developer ID Application: NAME (TEAMID)` |
| `APPLE_ID` | the Apple ID email used for notarization |
| `APPLE_PASSWORD` | an app-specific password for that Apple ID |
| `APPLE_TEAM_ID` | the 10-character Apple Team ID |

## Windows (SignPath)

The Windows Authenticode signing step is a **ready-to-activate template** in `.github/workflows/ci.yml` (commented out, immediately after *Locate built artefacts*). It is not live because the SignPath org/project/policy slugs come from the OSS account and every `uses:` in this repo must be SHA-pinned (OpenSSF Scorecard). Activation checklist is inline in the workflow. Required secrets once approved: `SIGNPATH_API_TOKEN`, `SIGNPATH_ORGANIZATION_ID`.

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
