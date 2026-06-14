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

## Silver / Gold gap analysis (2026-06-13)

The Passing badge is held (project #12755). Pursuing **Silver** (and where cheap,
**Gold**) is the only `CII-Best-Practices` Scorecard lever left. Most Silver
criteria we *already satisfy* from existing work; the table below maps the
notable ones and flags the genuine gaps.

**Already satisfied (answer "Met" with the cited evidence):**

| Silver/Gold criterion | Evidence |
|-----------------------|----------|
| `signed_releases` (Gold) | cosign keyless + SLSA provenance per release |
| `installation_common` / `build_reproducible` (partial) | `docs/reproduce.md` + `reproduce.sh`; SBOM per platform |
| `static_analysis` + `_often` | CodeQL on every commit; `cargo clippy`/`eslint --max-warnings 0` as CI gates |
| `dynamic_analysis` | fuzz workflow (`fuzz.yml`) |
| `vulnerabilities_fixed_60_days` / `vulnerability_report_process` | `SECURITY.md` + this remediation; `docs/known-advisories.md` for accepted upstream advisories |
| `architecture_documented` / `documentation_architecture` | `docs/trifecta.md`, `whitepaper.md`, `diagrams.md`, ADRs |
| `crypto_*` | Met-N/A — no custom crypto; relies on rustls/TLS, cosign, OS keystores |
| `test_policy` / `tests_documented_added` | `CONTRIBUTING.md` "Test gates"; CI requires green |
| `warnings` / `warnings_fixed` | `eslint --max-warnings 0`, `cargo deny check` green |
| `maintenance_or_update` | Dependabot (npm + cargo + actions) |
| `documentation_roadmap` | [`docs/roadmap.md`](roadmap.md) — public, status-marked, with explicit out-of-scope |
| `documentation_architecture` | [`docs/trifecta.md`](trifecta.md), [`whitepaper.md`](whitepaper.md), [`diagrams.md`](diagrams.md), ADRs |
| `documentation_security` / `documentation_quick_start` | [`SECURITY.md`](../SECURITY.md) + [`threat-model.md`](threat-model.md); README Installation/Quick-start |
| `governance` (project decision-making) | [`docs/governance.md`](governance.md) — model, decision process (ADRs), maintainer path, honest bus-factor |
| `code_review_standards` | [`CONTRIBUTING.md` §Review standards](../CONTRIBUTING.md) — the 8-point reviewer checklist |
| `assurance_case` | [`docs/assurance-case.md`](assurance-case.md) — claims C0–C5 mapped to argument + evidence + consumption-end check |
| `require_2FA` (committers use 2FA) | Documented in [`governance.md`](governance.md) + [`CONTRIBUTING.md` onboarding] as a merge-rights requirement |

**Genuine gaps for Silver:**

- **`dco` — sign-off on commits.** ✅ **Done** (2026-06-13). Enforced by a
  self-contained [`dco.yml`](../.github/workflows/dco.yml) check on every PR;
  contributor guidance in `CONTRIBUTING.md`. Use `git commit -s`.
- **`test_statement_coverage` (Silver 80% / Gold 90%).** 🔶 **Climbing, measured
  honestly; not yet at 80%.** Two efforts landed: (1) priority-first unit tests for
  the security-critical paths (IPC contract, credential flow, routing gate, install
  conductor, status surfaces) took frontend unit coverage **13% → ~53%**; (2) the
  E2E suite is now **instrumented** (`vite-plugin-istanbul`) and **merged** with the
  unit coverage (`scripts/merge-coverage.mjs`, both istanbul so they union cleanly),
  giving a **combined frontend ≈ 58% statements** — this counts the coverage the
  Playwright suite already provides instead of re-unit-testing the UI. Reported on
  every push/PR via [`coverage.yml`](../.github/workflows/coverage.yml). Do NOT
  claim the 80% row until the combined number actually reaches it; the gap now is
  the remaining presentational components, lower-risk and partly E2E-covered.
- **`build_reproducible` (full).** SLSA L2 + SBOM exist; byte-for-byte reproducibility
  is not yet verified end-to-end (Scorecard Tier-3B work in `road-to-recommendable.md`).
- **`two_person_review` (Gold).** Requires a second reviewer — same solo-maintainer
  cap as the Scorecard `Code-Review` check. Out of reach until a co-maintainer exists.

**Documentation criteria — closed (2026-06-13).** The Silver documentation/process
rows that were previously unanswered now have authored evidence: `documentation_roadmap`
([roadmap.md](roadmap.md)), `governance` ([governance.md](governance.md)),
`assurance_case` ([assurance-case.md](assurance-case.md)), and `code_review_standards`
([CONTRIBUTING.md §Review standards](../CONTRIBUTING.md)). Claim these "Met" with the
cited links.

**Honest split of what remains for Silver:**

- **`test_statement_coverage80` — ✅ Met (2026-06-13).** Frontend statement coverage
  is **80.11%** (1269/1584), measured by `npm run test:coverage` (istanbul), over 302
  passing unit tests. The climb from ≈53% was priority-first (security-critical IPC
  and credential flows first, then the install conductor, status surfaces, dev
  projection, and the failure cascade). Claim this row with the measured number.
- **People-blocked (cannot be satisfied solo, same root cause as the Scorecard
  `Code-Review`/`Contributors` 0-scores):** `two_person_review`,
  `contributors_unassociated`, `bus_factor`. These need a *second maintainer*. The
  governance and review-standards docs are written so that the moment a co-maintainer
  is active, these flip without further code work (enable required approvals; their
  commits supply the second affiliation).
- **`build_reproducible` (full).** SLSA L2 + per-platform SBOM exist; byte-for-byte
  reproducibility is not yet verified end-to-end (roadmap "Later").

**Bottom line:** the Silver *percentage* rises with every "Met" row above, but the
**Silver badge itself stays out of reach until a second maintainer exists** — and
the Scorecard `CII-Best-Practices` number only moves when the badge tier changes, so
this documentation work raises Silver readiness, not (yet) the Scorecard score. That
is the honest, intended outcome: make Silver instant once the people-gate clears.

## Submission steps

1. Sign in to [bestpractices.dev](https://www.bestpractices.dev) with the GitHub account that owns the repo.
2. New project → paste the project URL.
3. Walk through the criteria; copy the answers above into the relevant boxes.
4. Submit for the **Passing** tier. Silver is achievable later with the stretch items.
5. Once awarded: add the badge to README.md (URL provided by the badge service).
6. The Scorecard `CII-Best-Practices` check should flip to a non-zero score on the next nightly run.

Estimated time at the form: 15–20 min if using these answers.
