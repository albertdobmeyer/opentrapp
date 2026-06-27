# Release process

This document specifies the procedure for cutting and publishing a release of OpenTrApp. The procedure is auditable: every step is either codified in CI or recorded in a versioned artifact.

OpenTrApp is the headless `opentrapp-daemon` + the on-demand `viewer-server`, plus the five signed perimeter container images (ADR-0019 / ADR-0022 / ADR-0023). The de-Tauri cutover (PR #184) removed the old desktop-app build; this document describes the **current** lane.

## Prerequisites

Releases are cut by a maintainer with push access to `main` and the ability to create release tags. No signing keys live on the maintainer's machine: host-binary provenance and image signatures are produced in CI with ephemeral, GitHub-issued OIDC tokens (GitHub artifact attestations + cosign keyless via Sigstore). The only repository secret required is `CARGO_REGISTRY_TOKEN` (for the crates.io publish of `opentrapp-core`); it is optional — without it that one job no-ops.

## Versioning

OpenTrApp follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

- **MAJOR** — incompatible manifest contract change (schema enum removal, breaking field rename), or end-user data migration required
- **MINOR** — new feature, additive manifest schema change, or substantial UX rework that does not break existing manifests
- **PATCH** — bug fix, documentation, or dependency update with no behavioral change

Pre-release identifiers (`-rc.1`, `-beta.2`) mark binaries exposed to community testers but not yet recommended for general use. All distributable crates are versioned **in lockstep** (one product version, one tag): `opentrapp-core`, `opentrapp-daemon`, and `viewer-server` carry the same version, so a single `git tag vX.Y.Z` releases the whole product.

## Release checklist

The checklist below is the canonical procedure. Each step is reproducible from the repository state at the moment the tag is pushed.

1. **Verify `main` is green.** All workflows on the latest `main` commit must report success: `CI`, `CodeQL`, `OpenSSF Scorecard`, supply-chain. A red signal blocks the release.
2. **Bump the version (lockstep).** Set the identical new version in all four manifests:
   - `app/package.json` (`version`)
   - `app/src-tauri/crates/core/Cargo.toml` (`[package].version`)
   - `app/src-tauri/crates/daemon/Cargo.toml` (`[package].version`)
   - `app/src-tauri/crates/viewer-server/Cargo.toml` (`[package].version`)

   Then `cargo check` (updates `Cargo.lock`) and `tests/orchestrator-check.sh` (validates `package.json` ↔ daemon `Cargo.toml` agree). Confirm the unified announcement with `dist plan` — it must show **one** `announcing vX.Y.Z` covering both `opentrapp-daemon` and `viewer-server`.
3. **Write the release notes.** Create `docs/release-notes-vX.Y.Z.md` listing breaking changes, new features, bug fixes, known issues, and the full commit range (`git log --oneline vPREV..vNEW`). Scope every claim to what is verified (CLAUDE.md §11): do not assert a property the release does not actually carry.
4. **Open a release-prep PR** with the version bump + release notes; land it on `main` after review.
5. **Tag the release commit** (must point at the merged release-prep commit):
   ```bash
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin vX.Y.Z
   ```
6. **The tag push fans out to three workflows:**
   - **`release.yml` (cargo-dist).** Builds `opentrapp-daemon` + `viewer-server` for Linux (x86_64/aarch64), macOS (x86_64/aarch64), and Windows (x86_64); generates the shell (`curl … | sh`) and PowerShell installers + `sha256` checksums; attaches a **SLSA build-provenance attestation** to every artifact (GitHub-native, keyless); creates the GitHub Release and uploads all of it.
   - **`ci.yml` → `build-images`.** Builds the five perimeter images, pushes each to GHCR **by digest**, **cosign-signs** it keyless, exports the OCI tarballs, and writes the signed `image-digests.json` overlay. These are the artifacts the daemon's `fetch_perimeter_images()` consumes on first run (digest-pinned by `BundleVerifier`); they must be uploaded as **durable release assets** (issue #76).
   - **`publish-crate.yml`.** Publishes `opentrapp-core` to crates.io (only if `CARGO_REGISTRY_TOKEN` is set; the Scorecard-recognized Packaging signal).
7. **Verify the draft release (consumption end, CLAUDE.md §11).** Confirm:
   - installers + per-target archives for all five targets, each with its `.sha256`;
   - the `attestations` tab shows a build-provenance attestation for every host artifact;
   - the signed `image-digests.json` overlay + the five perimeter image tarballs are present as release assets (#76);
   - a clean-box `opentrapp-daemon vault up` against the published release loads the images, digest-pins them, and `vault verify` returns `pass=7 fail=0` (the post-release BundleVerifier T0). Win/macOS browser-runtime is maintainer-hardware-gated (#104).
8. **Publish the release.** Paste `docs/release-notes-vX.Y.Z.md` into the description and publish.

## The cargo-dist workflow is generated, then hardened — keep it that way

`.github/workflows/release.yml` is produced by `dist generate`, then **hand-hardened** to SHA-pin every third-party action (this repo's posture; the workflow runs privileged — `attestations: write` + `id-token: write`). `dist-workspace.toml` carries `allow-dirty = ["ci"]` so `dist` does not fight those edits. **After any `dist init` / version upgrade, re-apply the SHA pins** before committing (replace each `actions/<name>@vN` with its pinned commit SHA `# vN.N.N`), and re-run `dist plan` to confirm the unified announcement. The `dist` tool itself is installed in CI from its own version-pinned release over HTTPS — coherent with choosing it as the tool (ADR-0023).

## Verifying a published release

End users can verify a download without installing anything (also in `SECURITY.md`):

- **Host binaries:** `gh attestation verify <file> --repo albertdobmeyer/opentrapp` confirms the artifact was built by this repository's release workflow (SLSA provenance). Each archive also ships a `.sha256`.
- **Perimeter images:** `cosign verify ghcr.io/albertdobmeyer/opentrapp/<image>@sha256:<digest> --certificate-identity-regexp 'https://github.com/albertdobmeyer/opentrapp/.+' --certificate-oidc-issuer https://token.actions.githubusercontent.com` confirms the image was signed by this project's CI. At runtime the daemon does **not** depend on this; trust is the digest pin against the signed `image-digests.json` overlay (ADR-0011) — the cosign signature is the public audit axis.

These materials are produced unconditionally on every tag push; they cannot be omitted by a release manager. If they are missing, the release is suspect.

## Backporting and patch releases

Patch releases (`vX.Y.Z+1`) are cut from `main` when no MINOR-level work has landed since the previous patch. If MINOR-level work has landed, the patch is cut from a release branch (`release/vX.Y`). The release-prep PR procedure above applies in either case.

## Yanked releases

A release found to be defective after publication is yanked, not deleted:

1. Edit the GitHub release; mark it as a pre-release.
2. Append a notice explaining the defect and pointing to the replacement release.
3. If the defect is in a perimeter image, the digest pin makes already-installed daemons safe by construction (they only load the digest they were shipped); cut a replacement release with corrected images.

The yanked release's artifacts remain available so users who already downloaded them can verify what they have. Re-using a yanked tag for a different commit is not permitted.
