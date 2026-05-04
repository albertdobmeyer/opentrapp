# Lobster-TrApp v0.3.1 — Release Notes

**Tagged:** 2026-05-04
**Container baseline:** four-container perimeter as in v0.3.0; default Split Shell
**Target audience:** existing v0.3.0 users; non-technical end users

## Summary

A documentation, supply-chain, and brand release. No user-facing application functionality changes. Every shipped surface from v0.3.0 is preserved verbatim. The release is the first to exercise the new release-side attestation pipeline (cosign keyless + CycloneDX SBOM + SLSA Build Level 2 build-provenance) and the first to ship under the official LobsterTrApp brand assets.

## Changes since v0.3.0

### Brand

The application now ships under the official LobsterTrApp brand assets. Primary palette: brand-green `#009966` and brand-red `#CC3333`. Logo set:

- `app/public/logo-banner.png` — rectangular wordmark, used on the wizard `Welcome` screen, the README, and the landing-page hero
- `app/public/logo-square.png` — square symbol, used in the user-mode sidebar and as the master for the regenerated Tauri icon set (taskbar, `.ico`, `.icns`, Windows tiles, iOS, Android)
- `app/public/favicon.png` — claw silhouette, used as the browser tab icon

Components rebranded automatically through the existing `primary-*` token wiring: the wizard, the user-mode shell, the toast system, and the landing page. Pre-brand placeholder assets (`logo.svg`, `vite.svg`, the root-level `lobster-trapp-logo.png`, and a pair of stale screenshots) were removed.

### Publication-ready documentation set

Eight items from the post-launch enrichment roadmap landed in this version:

| Document | Purpose |
|---|---|
| [`docs/whitepaper.md`](whitepaper.md) | ~10-page paper-style treatment: threat model, system design, defense-in-depth, adaptive shells, CDR pipeline, implementation, evaluation, related work |
| [`docs/threat-model.md`](threat-model.md) | STRIDE-classified attacker-capability matrix (T1–T6) with residual-risk and empirical-evidence per row |
| [`docs/why-not-x.md`](why-not-x.md) | Differential against Firejail, gVisor, OS sandboxes, VM-only, scanner-only, allowlist-only, no-perimeter, capability-OS |
| [`docs/diagrams.md`](diagrams.md) | Five Mermaid diagrams: topology, trust tiers, network isolation, CDR pipeline, AssistantStatus state machine |
| [`docs/reproduce.md`](reproduce.md) + [`docs/reproduce.sh`](reproduce.sh) | Every numerical claim in the README mapped to an executable verification command |
| [`docs/adr/`](adr/) | Eight Architecture Decision Records covering the project's distinctive choices |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md), [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md), [`.github/pull_request_template.md`](../.github/pull_request_template.md) | Standard open-source hygiene |
| [`docs/demo/README.md`](demo/README.md) | Shooting script + ffmpeg recipe for the landing-page demo recording (the recording itself is queued for a future maintainer session) |

### Supply-chain attestation in CI

The `build-and-release` workflow gains a tag-only attestation block that runs after each platform build:

- A CycloneDX SBOM is generated per artefact via `anchore/sbom-action`
- Each artefact is signed with cosign keyless via Sigstore (the GitHub OIDC token is the trust anchor; no maintainer-held signing key)
- A SLSA Build Level 2 build-provenance attestation is produced via `actions/attest-build-provenance`
- All three are uploaded as draft-release assets alongside the platform installers

Verification commands for end users are documented in [`README.md`](../README.md) under *Test suite*.

### Security hardening of CI

GitHub Actions are now SHA-pinned (rather than `@v4`-style float references); job-level `permissions:` scoping defaults to `contents: read` with `build-and-release` opting up; the `pip install` of `pyyaml` is now `--require-hashes` against [`tests/requirements.txt`](../tests/requirements.txt). New workflows: CodeQL static analysis, OpenSSF Scorecard, supply-chain audit, Dependabot configuration, issue templates.

### Bug fixes

- **Submodule pointer drift.** The brand commit (`993a536`) accidentally rewound all three submodule pointers to older commits. The orchestrator-check suite (which validates that every cross-component workflow reference resolves at the parent's pointer) failed at 41/42 because pioneer's older `component.yml` did not yet have the `workflows:` section the orchestrator referenced. Fixed by bumping forge from `bd698a4` → `5bac4fb` (forward 49 commits), pioneer from `8b4c61c` → `52b3db2` (forward 46 commits), and vault from `76e0f0a` → `723d4a6` (forward 5 commits + a small gitignore addition that ignores the vault's runtime `.vault-audit-timestamp` and `.vault-config-hash` artefacts). Orchestrator-check is back to 42/42 with zero warnings.
- **Documentation drift on the Vitest count.** README and CLAUDE.md previously stated `Vitest (175)`. The actual count at v0.3.0 was 74 (matching the whitepaper's §8 ground truth). Corrected in five files (README, CLAUDE.md, CONTRIBUTING, reproduce.md, reproduce.sh).
- **GitHub-account migration footer in `.gitmodules`.** All three submodule URLs now resolve cleanly to `albertdobmeyer/<repo>.git` (previously several pointed at the old `gitgoodordietrying/` namespace through GitHub's redirect). `.gitmodules` and each submodule's working-tree git config are in sync.

### Test gates at release time

| Gate | Result |
|---|---|
| `cargo test --lib` | 56 passed |
| `npm test -- --run` | 74 passed |
| `npx tsc --noEmit` | clean |
| `npx playwright test` | 25 passed |
| `bash tests/orchestrator-check.sh` | 42 passed, 0 failed, 0 warnings |
| `bash docs/reproduce.sh` | 13/13 reproducible rows pass |

## Known issues

- The demo video for the landing page is queued — see [`docs/demo/README.md`](demo/README.md) for the shooting script. Until the recording lands, the landing-page hero shows the placeholder SVG.
- Five inline residual-risks tracked from [`docs/threat-model.md`](threat-model.md) (certificate pinning for upstream Anthropic / Telegram, fuzzing the CDR parser and generator, per-platform documentation of what persists after `compose down`, load-testing the proxy's rate-limiting threshold, friction-effect measurement on the per-action approval gate) are out of scope for this release.

## Upgrade path

There are no manifest-schema changes between v0.3.0 and v0.3.1. The auto-updater feed picks v0.3.1 up automatically. Users running v0.2.x should follow the v0.3.0 upgrade path first.

## Commit range

```
git log --oneline v0.3.0..v0.3.1
```

(populated post-tag; the principal commits since `v0.3.0` are: `993a536` brand work; `9ba4763` + `0592f5c` + `cc2c9d5` CI hardening; `a8ed9c1` whitepaper + first three ADRs + post-launch roadmap; `168642e` threat model + prior-art + reproduce.md + Mermaid diagrams + CONTRIBUTING + CoC + SLSA/SBOM CI; `50230dc` ADRs 0004–0008; `86c9742` + `da84495` submodule pointer fixes + orchestrator-workflows restoration; the v0.3.1 release-prep commit at the tag).

## Verifying this release

```bash
# 1. Reproducibility (deterministic counts and test gates)
bash docs/reproduce.sh

# 2. cosign verification of any release asset
cosign verify-blob \
  --certificate <asset>.pem \
  --signature <asset>.sig \
  --certificate-identity-regexp 'https://github.com/albertdobmeyer/lobster-trapp/.+' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  <asset>

# 3. SLSA build-provenance verification
gh attestation verify <asset> --owner albertdobmeyer

# 4. CycloneDX SBOM inspection
syft scan packages:<asset> -o cyclonedx-json | diff - sbom-<platform>.cyclonedx.json
```

These materials are produced unconditionally by CI on every tag push; their absence on a release is grounds for treating the release as suspect.
