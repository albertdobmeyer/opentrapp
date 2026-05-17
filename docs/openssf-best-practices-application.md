# OpenSSF Best Practices Badge — Application Draft

**Status:** Draft answers ready for the maintainer to copy into [bestpractices.dev](https://www.bestpractices.dev/en/projects/new). Submitting requires a logged-in maintainer; this file pre-fills the answers based on the project's current state so the form-fill takes ~15 minutes instead of an hour of re-research.

**Why bother:** the OpenSSF Best Practices Badge is one of the few ways to close the `CII-Best-Practices` 0-score on OpenSSF Scorecard (the other Scorecard 0-scores are `Branch-Protection` (Scorecard tooling limitation), `Code-Review` (historical), `Maintained` (auto-resolves over 90 days), `Fuzzing` (closed by the Fuzz workflow shipping in v0.3.2), `Signed-Releases` (closed by v0.3.2's cosign + SLSA pipeline)). The Passing-tier badge is achievable today; Silver and Gold are stretch goals.

The full criteria are at [github.com/coreinfrastructure/best-practices-badge](https://github.com/coreinfrastructure/best-practices-badge/blob/main/doc/criteria.md). Below are the criteria most likely to need a project-specific answer; the rest (e.g. *"Project must use a public version-control system"*) auto-pass for any GitHub-hosted MIT-licensed Rust+TypeScript project.

---

## Project metadata

| Field | Value |
|---|---|
| Project name | OpenTrApp |
| Description | A desktop application that runs the OpenClaw Clawbot inside a four-container security perimeter on the user's own computer, with a Telegram interface for chat. |
| Project URL | https://github.com/albertdobmeyer/opentrapp |
| Project home page | https://opentrapp.com |
| License | MIT |

## Basic project information

- **`description_good`:** Yes. README contains a one-paragraph what-it-does at the top.
- **`interact`:** Yes. GitHub Issues + Discussions enabled. SECURITY.md and CONTRIBUTING.md document the contact paths.
- **`contribution`:** Yes. CONTRIBUTING.md present, references the five test gates and the PR template.
- **`contribution_requirements`:** Yes. CONTRIBUTING.md explicitly lists code-of-conduct, the manifest contract rules, and the security-relevant-PR flagging.

## Change control

- **`repo_public`:** Yes. `git@github.com:albertdobmeyer/opentrapp.git`.
- **`repo_track`:** Yes. Git, every commit traceable.
- **`repo_distributed`:** Yes. Git is distributed.
- **`version_unique`:** Yes. Tags are immutable per `RELEASING.md` §"Yanked releases".
- **`version_tags`:** Yes. SemVer 2.0.0; tags are `vX.Y.Z`. Latest: `v0.3.2`.
- **`release_notes`:** Yes. `docs/release-notes-v*.md` per release; CHANGELOG-style.
- **`release_notes_vulns`:** Yes. v0.3.2 release notes call out the 10 RustSec vulns resolved + the deny.toml ignore policy.

## Reporting

- **`report_process`:** Yes. SECURITY.md documents the private-disclosure path and the in-scope/out-of-scope categories.
- **`report_tracker`:** Yes. GitHub Issues for non-security; security via `albertdobmeyer@proton.me` per SECURITY.md.
- **`report_responses`:** Yes. SECURITY.md commits to a 48-hour acknowledgment window.
- **`vulnerability_report_process`:** Yes. SECURITY.md.
- **`vulnerability_report_private`:** Yes. Email + GitHub private vulnerability reporting both available.
- **`vulnerability_report_response`:** Yes. SECURITY.md documents the 48-hour SLA.

## Quality

- **`build`:** Yes. `cargo build --release` produces the binary; `npm run tauri build` produces installers for all four target platforms (Linux, macOS ARM, macOS Intel, Windows).
- **`build_common_tools`:** Yes. Cargo + npm — both ubiquitous.
- **`build_floss_tools`:** Yes. All build tooling is open-source.
- **`test`:** Yes. Five test gates: `cargo test --lib` (56), Vitest (74), `tsc --noEmit` (strict), Playwright (25), `tests/orchestrator-check.sh` (42 checks). Plus the dogfood arc (`tests/dogfood/`) and the per-boundary tests (`tests/e2e-telegram/`).
- **`test_invocation`:** Yes. `make verify-all` runs all five gates; CI runs each on every PR.
- **`test_most`:** Yes. New code is expected to land with tests. The test gate on PRs prevents ratchet drift.
- **`test_policy`:** Yes. CONTRIBUTING.md documents the five gates.
- **`tests_are_added`:** Yes. New features land with new tests; see commit history.
- **`tests_documented_added`:** Yes. CONTRIBUTING.md.
- **`warnings`:** Yes. ESLint complexity gate is at 0 warnings as of PR #36 (locked in). Rust builds with two known warnings on `WorkflowStatus` derive (unintentional dead-code analysis interaction, documented).
- **`warnings_fixed`:** Yes. The lint ratchet (PR #27 → #33 → #34 → #36) drove the cap from 350 → 110 → 28 → 0 over four PRs.
- **`warnings_strict`:** Yes. `npx tsc --noEmit` with strict mode; `cargo deny check` strict on advisories/bans/licenses/sources; ESLint at `strict-type-checked`.

## Security

- **`know_secure_design`:** Yes. The project is itself an applied security-architecture paper. See `docs/whitepaper.md`, `docs/threat-model.md`, `docs/why-not-x.md`, `docs/adr/0001`–`0008`.
- **`know_common_errors`:** Yes. Whitepaper §10 (related work) cites Simon Willison's "lethal trifecta", OWASP LLM Top 10, Trail of Bits secure-ML guidance.
- **`crypto_published`:** N/A — no custom cryptography; we use cosign (Sigstore), SLSA, and Tauri's auto-updater Ed25519 signing, all standard primitives.
- **`crypto_call`:** Yes. Use of FOSS crypto libraries (sigstore, ring via rustls, ed25519-dalek via Tauri).
- **`crypto_floss`:** Yes.
- **`crypto_keylength`:** Yes. SLSA + cosign use industry-standard key lengths; we don't roll our own.
- **`crypto_working`:** Yes. Cryptographic protocols are not the project's focus; we depend on widely-deployed libraries.
- **`crypto_pfs`:** N/A.
- **`crypto_password_storage`:** N/A — no user accounts.
- **`crypto_random`:** Yes. The user's API credential (`ANTHROPIC_API_KEY`) is held in `vault-proxy`'s environment, never persisted by us.
- **`delivery_mitm`:** Yes. Release artefacts are downloaded over HTTPS from GitHub. Signed with cosign + SLSA build provenance.
- **`delivery_unsigned_email`:** Yes. We don't deliver via unsigned email.
- **`vulnerabilities_fixed_60_days`:** Yes. The 6 alerts in this session were resolved within 24 hours of opening.
- **`vulnerabilities_critical_fixed`:** Yes.

## Analysis

- **`static_analysis`:** Yes. CodeQL on every PR (`.github/workflows/codeql.yml`). Plus ESLint strict-type-checked.
- **`static_analysis_common_vulnerabilities`:** Yes. CodeQL covers the OWASP Top 10 patterns for the languages it analyses (JavaScript/TypeScript, Rust, Actions).
- **`static_analysis_fixed`:** Yes. CodeQL findings on `main` are at zero as of v0.3.2.
- **`static_analysis_often`:** Yes. Every PR + every push to main + a weekly scheduled run.
- **`dynamic_analysis`:** Yes. Fuzz workflow (`.github/workflows/fuzz.yml`) runs `cargo fuzz` against the manifest parser and the command-argument interpolator on every PR touching those surfaces.
- **`dynamic_analysis_unsafe`:** Yes. Address sanitizer enabled in fuzz builds.
- **`dynamic_analysis_enable_assertions`:** Yes. Debug-assertions on in fuzz builds.
- **`dynamic_analysis_fixed`:** Yes. No outstanding fuzz findings.

## Stretch (Silver/Gold) — items that need work

- **Reproducible builds:** SLSA Build Level 2 attestations are produced (cosign + provenance) but not yet *byte-for-byte reproducible*. SLSA L3 (a tamper-evident build platform) and reproducibility are stretch goals.
- **DCO sign-off on commits:** not currently enforced; would need a CI check.
- **Crypto algorithm review:** N/A as above; we don't ship custom crypto.
- **`document_architecture`:** Yes — but the *Silver* version of this asks for an architectural-pattern overview. We have one in `docs/trifecta.md`, `docs/whitepaper.md`, `docs/diagrams.md`. Solid.

## Submission steps

1. Sign in to [bestpractices.dev](https://www.bestpractices.dev) with the GitHub account that owns the repo.
2. New project → paste the project URL.
3. Walk through the criteria; copy the answers above into the relevant boxes.
4. Submit for the **Passing** tier. Silver is achievable later with the stretch items.
5. Once awarded: add the badge to README.md (URL provided by the badge service).
6. The Scorecard `CII-Best-Practices` check should flip to a non-zero score on the next nightly run.

Estimated time at the form: 15–20 min if using these answers.
