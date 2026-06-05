# OpenSSF Best Practices Badge — Application Draft

**Status:** Draft answers ready for the maintainer to copy into [bestpractices.dev](https://www.bestpractices.dev/en/projects/new). Submitting requires a logged-in maintainer; this file pre-fills the answers based on the project's current state so the form-fill takes ~15 minutes instead of an hour of re-research.

**Why bother:** the OpenSSF Best Practices Badge is one of the few ways to close the `CII-Best-Practices` 0-score on OpenSSF Scorecard (the other Scorecard 0-scores are `Branch-Protection` (Scorecard tooling limitation), `Code-Review` (historical), `Maintained` (auto-resolves over 90 days), `Fuzzing` (closed by the Fuzz workflow shipping in v0.3.2), `Signed-Releases` (closed by v0.3.2's cosign + SLSA pipeline)). The Passing-tier badge is achievable today; Silver and Gold are stretch goals.

The full criteria are at [github.com/coreinfrastructure/best-practices-badge](https://github.com/coreinfrastructure/best-practices-badge/blob/main/doc/criteria.md). Below are the criteria most likely to need a project-specific answer; the rest (e.g. *"Project must use a public version-control system"*) auto-pass for any GitHub-hosted MIT-licensed Rust+TypeScript project.

---

## Project metadata

| Field | Value |
|---|---|
| Project name | OpenTrApp |
| Description | A desktop application that runs an autonomous CLI agent inside a five-container security perimeter (L7 + L3 egress policy split — see ADR-0009) on the user's own computer, with a local-AI judgment layer (Sentinel — ADR-0015) for the gray zone the static defences miss, and a Telegram interface for chat. |
| Project URL | https://github.com/albertdobmeyer/opentrapp |
| Project home page | https://opentrapp.com |
| License | MIT |

## Basic project information

- **`description_good`:** Yes. README contains a one-paragraph what-it-does at the top.
- **`interact`:** Yes. The GitHub issue tracker is the public discussion and feedback channel (GitHub Discussions is not enabled). SECURITY.md and CONTRIBUTING.md document the contact and contribution paths.
- **`contribution`:** Yes. CONTRIBUTING.md present, references the five test gates and the PR template.
- **`contribution_requirements`:** Yes. CONTRIBUTING.md explicitly lists code-of-conduct, the manifest contract rules, and the security-relevant-PR flagging.

## Change control

- **`repo_public`:** Yes. `git@github.com:albertdobmeyer/opentrapp.git`.
- **`repo_track`:** Yes. Git, every commit traceable.
- **`repo_distributed`:** Yes. Git is distributed.
- **`version_unique`:** Yes. Tags are immutable per `RELEASING.md` §"Yanked releases".
- **`version_tags`:** Yes. SemVer 2.0.0; tags are `vX.Y.Z`. Latest: `v0.6.0` (published, signed, all four platforms).
- **`release_notes`:** Yes. `docs/release-notes-v*.md` per release; CHANGELOG-style. Current: `docs/release-notes-v0.6.0.md`.
- **`release_notes_vulns`:** Yes. Release notes call out security-relevant fixes per release (e.g. v0.3.2 documented the 10 RustSec vulns resolved + the deny.toml ignore policy); `cargo deny check` runs in CI to keep the advisory posture current.

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
- **`test`:** Yes. CI enforces the full gate on every PR (as of v0.6.0): `cargo test` (109), Vitest (87), `tsc --noEmit` (strict), ESLint at `--max-warnings 0`, Playwright smoke (25), `tests/orchestrator-check.sh` (108 checks), `tests/integration-test.sh --ci` (cross-module contracts, 0 failures), and the `vault-proxy` policy unit tests. Plus the dogfood arc (`tests/dogfood/`), the per-boundary tests (`tests/e2e-telegram/`), and the workload security bash suites (Sentinel judge, CDR/disarm, semantic firewall, persona-guard, the AT-Protocol adapter, skill-verify-judge).
- **`test_invocation`:** Yes. CI runs the full gate on every PR and push (`.github/workflows/ci.yml`: frontend lint+tsc+vitest, rust check+test, orchestration, integration, Playwright smoke), plus CodeQL and the Fuzz workflow. `make verify-all` runs the core gates locally.
- **`test_most`:** Yes. New code is expected to land with tests. The CI gate on PRs prevents ratchet drift.
- **`test_policy`:** Yes. CONTRIBUTING.md documents the test gates; CLAUDE.md §7 lists the full CI-equivalent gate (including the lint and integration jobs that must be run for a local green to match CI).
- **`tests_are_added`:** Yes. New features land with new tests; see commit history.
- **`tests_documented_added`:** Yes. CONTRIBUTING.md.
- **`warnings`:** Yes. ESLint runs at `--max-warnings 0` as a CI gate (the `check-frontend` job) — any warning fails the build; `cargo check` and `cargo test` are CI gates on the Rust side.
- **`warnings_fixed`:** Yes. The lint ratchet (PR #27 → #33 → #34 → #36) drove the cap from 350 → 110 → 28 → 0 over four PRs; v0.6.0 cleared a further 36 accumulated frontend lint problems and added the lint + integration jobs to the documented gate so the ratchet can't silently regress again.
- **`warnings_strict`:** Yes. `npx tsc --noEmit` with strict mode; `cargo deny check` strict on advisories/bans/licenses/sources; ESLint at `strict-type-checked` with `--max-warnings 0`.

## Security

- **`know_secure_design`:** Yes. The project is itself an applied security-architecture paper. See `docs/whitepaper.md`, `docs/threat-model.md` (six attacker categories T1–T6, decomposed by STRIDE; the v0.6 host-mediated allowlist-loosening row under T1 and the approval-fatigue analysis under T5), `docs/why-not-x.md`, and the 17 architecture-decision records `docs/adr/0001`–`0017` — including ADR-0015 (the local-AI judgment layer / Sentinel), ADR-0016 (host-mediated allowlist loosening, structurally never agent-loosenable per ADR-0002), and ADR-0009 (the L7/L3 five-container split).
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
- **`vulnerabilities_fixed_60_days`:** Yes. Dependency advisories surfaced by `cargo deny check` / Dependabot are resolved well inside 60 days — the prior batch of alerts was resolved within 24 hours of opening.
- **`vulnerabilities_critical_fixed`:** Yes.

## Analysis

- **`static_analysis`:** Yes. CodeQL on every PR (`.github/workflows/codeql.yml`). Plus ESLint strict-type-checked.
- **`static_analysis_common_vulnerabilities`:** Yes. CodeQL covers the OWASP Top 10 patterns for the languages it analyses (JavaScript/TypeScript, Rust, Actions).
- **`static_analysis_fixed`:** Yes. No medium-or-higher exploitable vulnerability from static code analysis is outstanding. CodeQL reports only note-level lint (unused-variable false positives on inline format arguments, now dismissed); the other open items in the code-scanning view are OpenSSF Scorecard posture checks surfaced as SARIF (Branch-Protection, Code-Review, Vulnerabilities, CII-Best-Practices) plus dependency-pin advisories on a developer devcontainer script, none of which are code vulnerabilities in the delivered software.
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
