# OpenSSF Scorecard

This document records the project's posture against the [OpenSSF Scorecard](https://github.com/ossf/scorecard) checklist. Scorecard runs weekly via the workflow at [`.github/workflows/scorecard.yml`](.github/workflows/scorecard.yml) and publishes results to the public Scorecard registry. The current score is visible from the badge in [`README.md`](README.md) and at [scorecard.dev](https://scorecard.dev/viewer/?uri=github.com/albertdobmeyer/opentrapp).

## Posture summary

The project's posture against each Scorecard check is summarised below. Three categories are used:

- **Earned** — automated controls in the repository satisfy the check
- **Pending action** — satisfaction requires a human action documented under the check
- **Resolves with time** — satisfaction is a function of repository age, release cadence, or accumulated history

### Earned

| Check | Mechanism |
|------|-----------|
| Binary-Artifacts | No checked-in binaries; build artifacts are produced in CI and published to GitHub Releases |
| Dangerous-Workflow | No `pull_request_target` with `actions/checkout` of the head ref, no untrusted-input expressions in shell scripts |
| Dependency-Update-Tool | Dependabot, configured in [`.github/dependabot.yml`](.github/dependabot.yml) |
| License | MIT, with [`LICENSE`](LICENSE) at the repository root |
| Pinned-Dependencies | Every GitHub Action is pinned by full commit SHA with a trailing version comment; pip dependencies use `--require-hashes` against [`tests/requirements.txt`](tests/requirements.txt) |
| SAST | CodeQL with security-extended and security-and-quality query packs, configured in [`.github/workflows/codeql.yml`](.github/workflows/codeql.yml) |
| Security-Policy | [`SECURITY.md`](SECURITY.md) at the repository root, with disclosure procedure, response timelines, and scope |
| Token-Permissions | All workflows declare a top-level `permissions` block; write scopes are job-scoped to the minimum required |

### Pending action

These checks require either a maintainer-only configuration change in the GitHub UI or an external attestation that cannot be granted from inside the repository.

| Check | Required action |
|-------|-----------------|
| **Branch-Protection** | Enable branch protection on `main` in **Settings → Branches**. Recommended ruleset: require pull request before merging, require at least one approving review, require status checks to pass (suggested: `Frontend (tsc + vitest)`, `Rust (check + test)`, `Orchestration`, `Integration tests`, `Playwright smoke tests`, `CodeQL`), require linear history, restrict force-pushes. |
| **CII-Best-Practices** | Self-attest the project at [bestpractices.dev](https://www.bestpractices.dev/). The questionnaire takes 30–60 minutes; the project already satisfies most criteria (license, security policy, version control, code review, automated tests, static analysis). The earned badge URL is added to the README badge row once issued. |
| **Code-Review** | Merge work through pull requests with at least one approving review, rather than direct pushes to `main`. Branch-Protection (above) enforces this once enabled. |

### Resolves with time

These checks have correct mechanics in place and will earn points without further intervention as the project accumulates history.

| Check | Resolution path |
|-------|-----------------|
| **Maintained** | Earns full credit once the repository is more than 90 days old and continues to receive commits at a normal cadence |
| **Signed-Releases** | The release pipeline produces cosign keyless signatures and SLSA build-provenance attestations on every tag push; the next tagged release after this commit will be the first that Scorecard sees as signed |
| **CI-Tests** | Earns credit as pull requests merge into `main` — Dependabot already opens these on a weekly schedule |

### Long-running work

These checks are intentionally not satisfied at present, with the rationale recorded for the next maintainer who reads this file.

| Check | Rationale |
|-------|-----------|
| **Vulnerabilities** | The advisory database flags transitive Rust dependencies (primarily through `reqwest` and `tokio`) for which no upstream patched release is available at the time of writing. Dependabot's weekly cargo updates will close these as patches are published. The supply-chain workflow (`cargo audit`, `cargo deny`) makes the open advisories visible in CI. |
| **Fuzzing** | A fuzzing harness over the manifest parser and the orchestrator's command interpolation is on the roadmap. Until then, the property tests in `app/src-tauri/src/orchestrator/tests.rs` exercise the same surfaces deterministically. |
| **Packaging** | OpenTrApp is a desktop application distributed via GitHub Releases, not a published package. Scorecard's check looks for a recognised packaging workflow (npm publish, container registry push, etc.); the GitHub Release flow does not match its current heuristics. |
| **Contributors** | The repository has a single maintaining organisation. The check reflects this accurately and is not a target for active improvement. |

## Verification

To re-run Scorecard locally against the current `main`:

```bash
docker run --rm \
  -e GITHUB_AUTH_TOKEN=$(gh auth token) \
  gcr.io/openssf/scorecard:stable \
  --repo=github.com/albertdobmeyer/opentrapp
```

The published machine-readable record for the most recent successful run is available at:

```
https://api.securityscorecards.dev/projects/github.com/albertdobmeyer/opentrapp
```
