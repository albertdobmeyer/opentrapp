# Release process

This document specifies the procedure for cutting and publishing a release of OpenTrApp. The procedure is auditable: every step is either codified in CI or recorded in a versioned artifact.

## Prerequisites

Releases are cut by a maintainer with push access to the `main` branch and the ability to create release tags. No external credentials are required at the maintainer's machine; signing is performed in CI using ephemeral, GitHub-issued OIDC tokens (cosign keyless via Sigstore).

## Versioning

OpenTrApp follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

- **MAJOR** — incompatible manifest contract change (schema enum removal, breaking field rename), or end-user data migration required
- **MINOR** — new feature, additive manifest schema change, or substantial UX rework that does not break existing manifests
- **PATCH** — bug fix, documentation, or dependency update with no behavioral change

Pre-release identifiers (`-rc.1`, `-beta.2`) are used for binaries that are exposed to community testers but not yet recommended for general use.

## Release checklist

The checklist below is the canonical procedure. Each step is reproducible from the repository state at the moment the tag is pushed.

1. **Verify `main` is green.** All workflows on the latest `main` commit must report success: `CI`, `CodeQL`, `OpenSSF Scorecard`, `Supply-chain audit`. A red signal blocks the release.
2. **Update the version number in three locations.** They are tracked in:
   - `app/package.json` (`version` field)
   - `app/src-tauri/Cargo.toml` (`[package].version`)
   - `app/src-tauri/tauri.conf.json` (`version` field)

   The orchestration check (`tests/orchestrator-check.sh`) validates that all three are equal.
3. **Write the release notes.** Create `docs/release-notes-vX.Y.Z.md` following the structure of the most recent prior release notes file. The release notes must, at minimum, list:
   - Breaking changes (MAJOR/MINOR only)
   - New features
   - Bug fixes
   - Known issues at the time of release
   - The full commit range (`git log --oneline vPREV..vNEW`)
4. **Open a release-prep pull request** containing the version bump and the release notes. Land it on `main` after the standard review.
5. **Tag the release commit.**
   ```bash
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin vX.Y.Z
   ```
   The tag must point to the merge commit that landed the release-prep PR.
6. **Wait for `build-and-release` to complete.** The tag push triggers the `build-and-release` job in `ci.yml`, which builds installers for Linux, macOS (Apple Silicon and Intel), and Windows, generates a CycloneDX SBOM per platform, signs every artifact with cosign, and produces a SLSA Build Level 2 provenance attestation. Outcomes are uploaded as draft release assets.
7. **Verify the draft release.** Open the GitHub release page and confirm:
   - All four platforms produced installers
   - An `sbom-*.cyclonedx.json` is present for each platform
   - Each installer is accompanied by a `.sig` (cosign signature) and `.pem` (signing certificate)
   - The `attestations` tab shows a build-provenance attestation for every artifact
8. **Publish the release.** Edit the draft release: paste the contents of `docs/release-notes-vX.Y.Z.md` into the description field, then click "Publish release". The release is now visible to end users and the auto-updater pipeline.

## Verifying a published release

End users who wish to verify a downloaded binary can do so without installing the application. The procedure is documented in `SECURITY.md`. In summary:

- Each artifact has an associated `*.sig` (signature) and `*.pem` (certificate) in the same release.
- `cosign verify-blob --certificate <file>.pem --signature <file>.sig --certificate-identity-regexp 'https://github.com/albertdobmeyer/opentrapp/.+' --certificate-oidc-issuer https://token.actions.githubusercontent.com <file>` confirms that the artifact was produced by the project's GitHub Actions workflow.
- The SLSA build-provenance attestation is verifiable via `gh attestation verify <file> --owner albertdobmeyer`.
- The CycloneDX SBOM enumerates every dependency that was linked into the build.

These materials are produced unconditionally on every tag push; they cannot be omitted by a release manager. If they are missing from a release, the release is suspect.

## Backporting and patch releases

Patch releases (`vX.Y.Z+1`) are cut from `main` when no MINOR-level work has landed since the previous patch release. If MINOR-level work has landed, the patch is cut from a release branch (`release/vX.Y`). The release-prep PR procedure in §Release checklist applies in either case.

## Yanked releases

A release that is found to be defective after publication is yanked, not deleted. The procedure is:

1. Edit the GitHub release; check "Mark as a pre-release".
2. Append a notice to the release description explaining the defect and pointing to the replacement release.
3. The auto-updater feed is regenerated; existing installations stop receiving the yanked release as an update candidate.

The yanked release's artifacts remain available so that users who previously downloaded them can verify what they have. Re-using a yanked tag for a different commit is not permitted.
