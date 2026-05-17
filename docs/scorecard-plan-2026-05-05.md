# Scorecard improvement plan

**Created:** 2026-05-05
**Trigger:** OpenSSF Scorecard report against commit `9fee9b8` returned a score of **5.6 / 10**. This document captures the per-check assessment, the practical path to the highest reachable score, and a cheat-sheet for the OpenSSF Best Practices self-attestation that satisfies one of the open items.

For the long-running posture record, see [`SCORECARD.md`](../SCORECARD.md). This file documents a single planning sweep on 2026-05-05 and the actions chosen from it.

## 1. Score arithmetic

Scorecard reports a weighted average. The published weights per severity are:

| Severity | Weight |
|---|---|
| Critical | 10 |
| High | 7.5 |
| Medium | 3 |
| Low | 1 |

Total available points across the 17 active checks (Packaging is reported as `?` and excluded by the API):

| Severity | Checks | Max points |
|---|---|---|
| Critical | 1 | 100 |
| High | 8 | 600 |
| Medium | 4 | 120 |
| Low | 4 | 40 |
| **Total** | **17** | **860** |

The `5.6` score corresponds to roughly 482 weighted points earned out of 860.

## 2. Per-check assessment (commit `9fee9b8`, run dated 2026-05-05T13:19Z)

| Check | Score | Severity | Status | Action plan |
|------|-------|----------|--------|-------------|
| Dangerous-Workflow | 10 | critical | Earned | hold |
| Binary-Artifacts | 10 | high | Earned | hold |
| Dependency-Update-Tool | 10 | high | Earned | hold |
| Token-Permissions | 10 | high | Earned | hold |
| Pinned-Dependencies | 10 | medium | Earned | hold |
| SAST | 10 | medium | Earned | hold |
| Security-Policy | 10 | medium | Earned | hold |
| CI-Tests | 10 | low | Earned | hold |
| License | 10 | low | Earned | hold |
| Branch-Protection | 4 | high | Partial | Tighten — see §3.4 |
| Contributors | 3 | low | Partial | Not engineerable in-session |
| Code-Review | 0 | high | Stale | Re-run Scorecard — eleven PRs merged through CI on 2026-05-04/05 are not yet reflected |
| Maintained | 0 | high | Time-gated | Auto-resolves at repository age 90 days |
| Signed-Releases | 0 | high | Pending | Cut v0.4.0 through the existing cosign + SLSA pipeline (`RELEASING.md`) |
| Vulnerabilities | 0 | high | Capped | See §3.5 — 25 open advisories, ~17 are unmaintained transitive crates with no first-party remediation |
| Fuzzing | 0 | medium | Pending | Add `cargo-fuzz` harnesses for the manifest parser and the orchestrator's command interpolator |
| CII-Best-Practices | 0 | low | Pending | Self-attest at [bestpractices.dev](https://www.bestpractices.dev/) — see §4 cheat-sheet |
| Packaging | ? | medium | Heuristic mismatch | GitHub Releases is not currently recognised by Scorecard's packaging heuristic; not actionable in-session |

## 3. Path to the highest reachable score

The actions below are ordered by impact-per-effort. Each row records the expected weighted-point gain on a fresh Scorecard run after the action lands.

### 3.1 Refresh the Scorecard run (5 minutes)

Two checks are already earned in code that Scorecard has not yet seen:

- **Code-Review** earns once Scorecard observes commits merged via PR with a passing CI run. Eleven such commits landed between 2026-05-04 evening and 2026-05-05 morning. A re-run is required to capture them.
- **Vulnerabilities** count may shift as some advisories age out of the active window or are superseded.

The Scorecard workflow currently has no `workflow_dispatch` trigger; the first prerequisite is to add one (delivered in PR #18, "ci: add workflow_dispatch trigger to OpenSSF Scorecard workflow"). Once merged, run:

```bash
gh workflow run scorecard.yml
```

**Expected gain:** Code-Review 0 → 8-10 (+60 to +75). Score reaches ~6.6-7.0.

### 3.2 Earn the OpenSSF Best Practices Passing badge (30-60 minutes, maintainer-only)

The questionnaire at [bestpractices.dev](https://www.bestpractices.dev/) self-attests against an open checklist that the project already largely satisfies. See §4 for the file-by-file mapping that fills the form.

**Expected gain:** CII-Best-Practices 0 → 5 (Passing). Score reaches ~7.0.

### 3.3 Cut v0.4.0 (30-60 minutes, maintainer-only)

The CI pipeline produces per-platform CycloneDX SBOMs, sigstore keyless signatures, and SLSA Build Level 2 attestations on every tag push. The v0.3.x releases predate the pipeline; the next tagged release is the first that Scorecard sees as signed. Procedure: `RELEASING.md` §"Release checklist".

**Expected gain:** Signed-Releases 0 → 10 (+75). Score reaches ~7.9.

### 3.4 Tighten Branch-Protection (30 minutes plus signing setup)

Three Scorecard sub-criteria within Branch-Protection are not yet satisfied:

1. **No bypass list** — the current ruleset includes `Claude` and `albertdobmeyer` on the bypass list. Scorecard penalises every bypass entry. Removing entries earns the sub-criterion at the cost of self-merge friction. Acceptable trade once the maintainer is comfortable with the PR cadence.
2. **Require signed commits** — requires the maintainer to configure GPG or SSH commit signing locally and register the key with GitHub. The ruleset rule is then enabled. Documented in [github.com/...settings/keys](https://github.com/settings/keys).
3. **Require status checks include CodeQL** — already covered by `Analyze (rust)`, `Analyze (javascript-typescript)`, `Analyze (actions)` in the required-status-checks list.

**Expected gain:** Branch-Protection 4 → ~8-9 (+30 to +37). Score reaches ~8.3-8.4.

### 3.5 Vulnerabilities — the realistic ceiling (3-5 / 10)

The Vulnerabilities check enumerates every open OSV/GitHub Advisory affecting the dependency graph. The current count of 25 decomposes:

- **6 npm advisories** (GHSA prefix only, no RUSTSEC tag): live in JavaScript dependencies that Dependabot's weekly cargo+npm sweep is already proposing patches for. Triaging and merging the next Dependabot wave closes most of them.
- **17 unmaintained Rust transitive crates**: GTK3 bindings (10 crates), Unicode dispatch chain (5 crates), `proc-macro-error`, `fxhash`. These are dependencies of `tauri/wry` on Linux and have no upstream remediation. They are documented in `app/src-tauri/deny.toml`'s `[advisories.ignore]` block with reasons.
- **2 GHSA-tagged Rust advisories** that overlap with the unmaintained set (`GHSA-wrw7-89jp-8q8g` / `RUSTSEC-2024-0429`, `GHSA-cq8v-f236-94qc` / `RUSTSEC-2026-0097`).

**Scorecard does not honour `deny.toml` ignores.** Every open advisory counts regardless of whether the project has accepted the risk with a stated reason. The ~17-19 unmaintained-but-unfixable advisories therefore cap this check at approximately 3-5 / 10 until the upstream Tauri ecosystem migrates to GTK4 (tracked at [tauri-apps/wry#1236](https://github.com/tauri-apps/wry/issues)) and the `idna`/`unicode-bidi` chain migrates to `icu4x`.

**Expected gain after npm cleanup:** Vulnerabilities 0 → ~3-5 (+22 to +37). Score reaches ~8.6-8.9.

### 3.6 Fuzzing harness (2-4 hours)

The Vault and the manifest parser are the two highest-leverage fuzz targets. `cargo-fuzz` over the manifest parser exercises the YAML→Rust struct decoder in `app/src-tauri/src/orchestrator/manifest.rs`; an orchestrator fuzz target exercises the `runner.rs` argument-interpolation code path that wraps user-supplied arguments. A minimal viable harness with two targets is achievable in one focused session.

**Expected gain:** Fuzzing 0 → 10 (+30). Score reaches ~9.0-9.2.

### 3.7 Maintained (auto-resolves)

Repository created 2026-04-21 (per the inferred history). Earns full credit on or after 2026-07-21.

**Expected gain:** Maintained 0 → 10 (+75). Score reaches ~9.8-9.9 (combined with all of the above).

### 3.8 Contributors (not in scope)

The check rewards commits authored by people from at least two distinct organisations. Single-maintainer projects therefore plateau at low scores on this check. Earning more than 3/10 requires either recruiting a contributor whose GitHub profile lists a different organisation, or accepting external pull requests at a meaningful cadence. Neither is engineerable in-session.

### 3.9 Realistic ceiling

Adding the gains in §3.1 through §3.7, the realistic ceiling is **~9.5 / 10**. True 10/10 is blocked by:

- Contributors (capped near 3 / 10 without recruited co-maintainers)
- Vulnerabilities (capped near 3-5 / 10 by upstream Tauri/Unicode-chain dependencies)
- Packaging (currently scored as `?`; would require publishing through a recognised package manager in addition to GitHub Releases)

The 9.5 / 10 ceiling assumes everything in §3.1–3.7 is executed and Maintained has aged in.

## 4. CII Best Practices cheat-sheet

The questionnaire at [bestpractices.dev](https://www.bestpractices.dev/) covers approximately 70 criteria organised into eight categories. Most are auto-detected by the form's repository scanner; the rest are confirmed with a one-line answer or a URL. The mapping below identifies the in-repository file or fact that satisfies each criterion the project already meets.

### Basics

| Criterion | Evidence |
|---|---|
| Project URL | https://github.com/albertdobmeyer/opentrapp |
| Open-source license | `LICENSE` (MIT) |
| FSF/OSI-approved license | yes (MIT) |
| Project page lists license | README.md badge row + License section |
| Documentation: how to install | README.md §"Requirements" + §"Installation" |
| Documentation: getting started | README.md §"Capabilities" + GitHub Releases page |
| Documentation: how to contribute | `CONTRIBUTING.md` |
| Project tracks bugs | https://github.com/albertdobmeyer/opentrapp/issues |
| Code of Conduct published | `CODE_OF_CONDUCT.md` |
| Welcoming community | `CODE_OF_CONDUCT.md` (adapted from Contributor Covenant 2.1) |

### Change control

| Criterion | Evidence |
|---|---|
| Public repository | https://github.com/albertdobmeyer/opentrapp |
| Version-control system | Git (GitHub) |
| Unique version IDs | tags follow SemVer (`v0.3.0`, `v0.3.1`); `RELEASING.md` §"Versioning" |
| Release notes published | `docs/release-notes-v0.3.0.md`, `docs/release-notes-v0.3.1.md` |
| Release notes report security fixes | when applicable, recorded in the release-notes file |

### Reporting

| Criterion | Evidence |
|---|---|
| Bug reporting process documented | `CONTRIBUTING.md` + GitHub issue templates (`.github/ISSUE_TEMPLATE/bug_report.yml`) |
| Acknowledgement within 14 days | `SECURITY.md` §"Response" — 48 hours acknowledged |
| Vulnerability reporting process | `SECURITY.md` §"Reporting a Vulnerability" |
| Private vulnerability reporting channel | email `gitgoodordietrying@proton.me` per `SECURITY.md` |

### Quality

| Criterion | Evidence |
|---|---|
| Working build system | `app/package.json` + `app/src-tauri/Cargo.toml` + `tauri.conf.json` |
| Standard build flow | `npm ci && npm run build`; `cargo build` (RELEASING.md §"Build and test") |
| Automated test suite | `.github/workflows/ci.yml` runs vitest, cargo test, Playwright |
| New functionality requires tests | `CONTRIBUTING.md` mentions test coverage in PR template |
| Test coverage measured | partially — Rust uses `cargo test`; frontend uses vitest |
| Compiles without warnings | yes — `tsc --noEmit` and `cargo check` are CI gates |
| Static analysis used | CodeQL via `.github/workflows/codeql.yml`, with `security-extended` and `security-and-quality` query packs |
| Hardened development | Branch protection ruleset + CodeQL + Dependabot + cargo audit + cargo deny |

### Security

| Criterion | Evidence |
|---|---|
| Cryptographic features documented | `docs/threat-model.md` + `app/src-tauri/src-tauri/capabilities/default.json` |
| Threat model published | `docs/threat-model.md` |
| Cryptographic libraries from approved set | `rustls` (default-tls feature on `reqwest`), `ring` (transitive); no hand-rolled crypto |
| Cryptographic agility | TLS via `rustls`; `reqwest` 0.12 used at the application layer |
| Vulnerability response timeline | `SECURITY.md` §"Response" |
| Use cryptographic hash for verification of releases | cosign signatures + SLSA provenance attestations on every tag (after v0.4.0) |
| Reproducible build | `docs/reproduce.md` + `docs/reproduce.sh` |

### Analysis

| Criterion | Evidence |
|---|---|
| Static analysis tool runs in CI | CodeQL workflow, supply-chain workflow (cargo audit, cargo deny, npm audit) |
| Dynamic analysis | partial — Playwright smoke tests + integration tests; full DAST is on the roadmap |
| Dependency analysis | Dependabot + cargo audit + npm audit |
| Software Composition Analysis (SCA) | CycloneDX SBOM produced per release |

### Project oversight

| Criterion | Evidence |
|---|---|
| Project sites use HTTPS | https://opentrapp.com (GitHub Pages) |
| Multi-factor authentication on the maintainer account | yes (GitHub enforces 2FA for maintainers of public repos with submitted attestation as of 2024-Q4) |

### Filling out the form

1. Sign in at https://www.bestpractices.dev/ with GitHub.
2. Click **Get badges** → **Submit a New Project**.
3. Repository URL: `https://github.com/albertdobmeyer/opentrapp`.
4. The form auto-detects most criteria from the repository contents. Confirm each, paste the relevant URL from the table above when asked for evidence, and use a one-line written justification when no URL applies (e.g. "MFA is enforced on the maintainer's GitHub account").
5. Save progress every few criteria. The form auto-saves but explicit saves are insurance.
6. On submit, the project is awarded the **Passing** badge if all required criteria are met.

The project is unlikely to clear **Silver** (which requires more rigorous static-analysis evidence and an explicit security-architecture document beyond `threat-model.md`) without follow-up work, and **Gold** (which requires reproducible-build verification by an independent third party) is a multi-month effort. Passing is the realistic target for this session.

## 5. Action queue for the current session

The autonomous in-session work, ordered, is:

1. **Land PR #18** — adds `workflow_dispatch` to the Scorecard workflow. Already opened on this branch. Once merged, trigger `gh workflow run scorecard.yml` to refresh the score against the eleven PR-merges from 2026-05-04/05. Expected jump: **+1.0 to +1.4 score points**.
2. **Open the v0.4.0 release-prep PR** — version bump in three files (`app/package.json`, `app/src-tauri/Cargo.toml`, `app/src-tauri/tauri.conf.json`) plus a release-notes file at `docs/release-notes-v0.4.0.md`. Maintainer reviews and tags. Expected jump on tag push: **+0.9 score points** (Signed-Releases earns).
3. **Add the fuzzing harness** — `cargo-fuzz` initialisation in `app/src-tauri/`, fuzz targets for `manifest::parse` and `runner::interpolate_args`, a CI workflow that runs the corpus for a bounded duration on each PR. Expected jump: **+0.4 score points**.

Out-of-session, maintainer-only:

4. **OpenSSF Best Practices form** — using §4 above. Expected jump: **+0.1 score points**.
5. **Tighten Branch-Protection** — remove Claude from the bypass list once the maintainer is comfortable with the PR cadence; configure commit signing and enable the rule. Expected jump: **+0.4 score points**.
6. **Wait for Maintained to age in** — automatic at day 90. Expected jump: **+0.9 score points**.

After (1)–(6) the projected score is **9.0–9.5**. Items requiring engineering investment beyond the current session (broader fuzzing coverage, eradicating remaining transitive npm advisories, recruiting a second-org contributor) remain as backlog.
