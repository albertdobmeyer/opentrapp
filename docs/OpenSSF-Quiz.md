# OpenSSF Best Practices — Questionnaire Record

**Project:** OpenTrApp
**Repository:** https://github.com/albertdobmeyer/opentrapp
**Project home:** https://opentrapp.com
**Badge series:** Metal (Passing → Silver → Gold)
**Current target:** Passing
**bestpractices.dev project:** [#12755](https://www.bestpractices.dev/en/projects/12755) — edit at https://www.bestpractices.dev/en/projects/12755/edit
**First submitted:** 2026-05-05 (under the project's former name, *Lobster-TrApp* — before the 2026-05-17 rebrand)
**Last edited:** 2026-05-05

> ⚠ **The live entry still carries the old branding.** Project #12755 was filed
> before the rebrand and still shows name `Lobster-TrApp`, home `lobster-trapp.com`,
> repo URL `…/lobster-trapp`, and a stale "four-container / OpenClaw" description (~18%
> complete). The answers in *this* file are current (OpenTrApp, five-container, v0.6.0);
> the live form is not. **Do not create a new project — edit #12755 in place** (a
> name/domain change never needs a re-application). Update the name, home-page URL,
> repository URL (the field Scorecard's `CII-Best-Practices` check keys on), and
> description, then continue from the ~18% already recorded toward Passing.

This document records every answer submitted to the [OpenSSF Best Practices](https://www.bestpractices.dev/) questionnaire. It pairs with the criterion-to-evidence mapping at [`scorecard-plan-2026-05-05.md`](scorecard-plan-2026-05-05.md) §4 — that document shows which repository file satisfies each criterion; this document records what was actually written into each form field at submission time.

**Re-attestation procedure.** When the badge is re-attested (annually, or after a major criterion change), copy the answers below into the form, edit only the fields whose answers have materially changed, and update the **Last edited** stamp at the top. Each new edit pass adds a row to the §"Update log" at the bottom of this file.

**Section numbering.** The headings below mirror the section order shown in the bestpractices.dev questionnaire so a maintainer scrolling through the form can scroll through this file in lock-step.

---

## Section 1 — General

### Project name

> OpenTrApp

### Brief description (markdown)

> **OpenTrApp** is a desktop application that lets you safely use an autonomous AI helper on your own computer.
>
> AI helpers like the *agent* can read files, run programs, and install community-made plugins. Most of the time these features are useful, but they also mean a poorly written or malicious plugin could damage your computer or leak your data. OpenTrApp keeps the AI helper inside a sealed-off space — a *sandbox* — so it can do its job without touching your real files, passwords, or other programs. It also checks every plugin for known dangers before letting the helper use it.
>
> You don't need to be a programmer to use it. Everything is controlled through a friendly desktop window, and you chat with the helper through regular [Telegram](https://telegram.org/) messages. Free and open source under the MIT license.

### Description language

> English (en)

### Project URL

> https://opentrapp.com

### Repository URL

> https://github.com/albertdobmeyer/opentrapp

### Licence(s)

> MIT

### Programming languages

> TypeScript, Rust, Python, Shell, JavaScript

### CPE name

> _(none — the project does not have a registered Common Platform Enumeration name at the time of submission. The repository will register a CPE if a downstream consumer requires one for compliance reporting.)_

### Other general comments

> _(empty)_

---

## Section 2 — Basic project website content

### `description_good` — Project website MUST succinctly describe what the software does

**Status:** Met
**Justification:**

> The project's README opening paragraph and the landing page at https://opentrapp.com both state in their opening sentence that OpenTrApp is a desktop application running the agent inside a five-container security perimeter (L7 + L3 egress policy split per ADR-0009) on the user's own computer. The README §"Purpose" expands this with the threat-model rationale (the ClawHavoc 2026-Q1 study finding 11.9% of ClawHub skills were malicious) so a visitor learns the function and the differentiator inside the first scroll.

**Evidence URL:** https://github.com/albertdobmeyer/opentrapp/blob/main/README.md

### `interact` — Project website MUST provide information on how to obtain, give feedback, and contribute

**Status:** Met
**Justification:**

> README §"Requirements" and the landing page link to GitHub Releases for download. Bug reports and feature requests are accepted via GitHub Issues with structured templates at `.github/ISSUE_TEMPLATE/bug_report.yml` and `.github/ISSUE_TEMPLATE/feature_request.yml`. Security vulnerabilities follow the private process documented in `SECURITY.md`. The contribution workflow is documented in `CONTRIBUTING.md`. General discussion is open via GitHub Discussions.

### `contribution` — Contribution process MUST be explained (URL required)

**Status:** Met
**Justification:**

> Non-trivial contribution file in the repository describing the build/test/submission workflow, branch ruleset, and review expectations.

**Evidence URL:** https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### `contribution_requirements` — Acceptable-contribution requirements SHOULD be documented (URL required)

**Status:** Met
**Justification:**

> CONTRIBUTING.md documents the contribution process. The pull-request template at `.github/pull_request_template.md` enumerates the expected fields including a test plan. Architectural rules and the manifest contract that contributions must respect are in `CLAUDE.md`. Code-quality gates (TypeScript strict, `cargo check`, `cargo test`, vitest, Playwright, CodeQL) run automatically on every PR via `.github/workflows/ci.yml` and must pass before merge per the branch ruleset.

**Evidence URL:** https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

---

## Section 3 — FLOSS license

### `floss_license` — Software MUST be released as FLOSS

**Status:** Met
**Justification:**

> The MIT license is approved by the Open Source Initiative (OSI).

### `floss_license_osi` — Required licence(s) SHOULD be OSI-approved

**Status:** Met
**Justification:**

> The MIT license is approved by the Open Source Initiative (OSI).

### `license_location` — Project MUST post licence(s) in a standard location (URL required)

**Status:** Met
**Justification:**

> Non-trivial license location file in repository.

**Evidence URL:** https://github.com/albertdobmeyer/opentrapp/blob/main/LICENSE

---

## Section 4 — Documentation

### `documentation_basics` — Project MUST provide basic documentation

**Status:** Met
**Justification:**

> README.md covers requirements, installation, capabilities, limitations, build instructions, and verification commands. RELEASING.md documents the release procedure. CONTRIBUTING.md, CODE_OF_CONDUCT.md, and SECURITY.md cover the contributor and security paths. The technical architecture is in `docs/trifecta.md` and `docs/threat-model.md`; ADRs are in `docs/adr/`; release notes per version are in `docs/release-notes-vX.Y.Z.md`.

**Evidence URL:** https://github.com/albertdobmeyer/opentrapp/blob/main/README.md

### `documentation_interface` — Project MUST provide reference documentation describing the external interface

**Status:** Met
**Justification:**

> OpenTrApp is a desktop application rather than a library; its "external interface" is the manifest contract that third-party components must conform to. The contract is specified by JSON Schema at `schemas/component.schema.json` (the source of truth) and described in `CLAUDE.md` §"The manifest contract". Each section (identity, status, commands, configs, health, workflows) has a normative description. The user-facing GUI surfaces are documented in the README and demonstrated in the demo-recording scaffold at `docs/demo/`.

**Evidence URL:** https://github.com/albertdobmeyer/opentrapp/blob/main/schemas/component.schema.json

---

## Section 5 — Other basic criteria

### `sites_https` — Project sites MUST support HTTPS using TLS

**Status:** Met
**Justification:**

> Given only `https:` URLs.

### `discussion` — Project MUST have searchable, URL-addressable, open discussion mechanism(s)

**Status:** Met
**Justification:**

> GitHub supports discussions on issues and pull requests. GitHub Discussions are also enabled on the repository.

### `english` — Project SHOULD provide documentation in English and accept bug reports and code comments in English

**Status:** Met
**Justification:**

> All project documentation, code comments, commit messages, issue and PR conversations, and release notes are in English. Bug reports and code-review comments are accepted in English.

### `maintained` — Project MUST be maintained

**Status:** Met
**Justification:**

> Actively maintained. The default branch receives multiple commits per week. Two tagged releases in the past 30 days (v0.3.0 on 2026-05-02 and v0.3.1 on 2026-05-04). A v0.3.2 patch release is queued at the time of this attestation. The repository has an enforced branch ruleset that requires all CI checks to pass before merge, and the OpenSSF Scorecard workflow runs weekly. Maintainer: @albertdobmeyer.

**Evidence URL:** https://github.com/albertdobmeyer/opentrapp/commits/main

---

## Section 6 — Change control

_To be filled in as the user reaches this section of the questionnaire. Expected criteria include `repo_public`, `repo_track`, `repo_distributed`, `version_unique`, `version_semver`, `version_tags`, `release_notes`, `release_notes_vulns`. The cheat-sheet at [`scorecard-plan-2026-05-05.md`](scorecard-plan-2026-05-05.md) §4 maps each criterion to its evidence file._

---

## Section 7 — Reporting

_To be filled in. Expected criteria include `report_process`, `report_tracker`, `report_responses`, `enhancement_responses`, `report_archive`, `vulnerability_report_process`, `vulnerability_report_private`, `vulnerability_report_response`._

---

## Section 8 — Quality

_To be filled in. Expected criteria include `build`, `build_common_tools`, `build_floss_tools`, `test`, `test_invocation`, `test_most`, `test_policy`, `tests_are_added`, `tests_documented_added`, `warnings`, `warnings_fixed`, `warnings_strict`._

---

## Section 9 — Security

_To be filled in. Expected criteria include `know_secure_design`, `know_common_errors`, `crypto_published`, `crypto_call`, `crypto_floss`, `crypto_keylength`, `crypto_working`, `crypto_pfs`, `crypto_password_storage`, `crypto_random`, `crypto_weaknesses`, `crypto_alternatives`, `crypto_used_network`, `crypto_tls12`, `crypto_certificate_verification`, `crypto_verification_private`, `hardening`, `assurance_case`._

---

## Section 10 — Analysis

_To be filled in. Expected criteria include `static_analysis`, `static_analysis_common_vulnerabilities`, `static_analysis_fixed`, `static_analysis_often`, `dynamic_analysis`, `dynamic_analysis_unsafe`, `dynamic_analysis_enable_assertions`, `dynamic_analysis_fixed`._

---

## Section 11 — Project oversight

_To be filled in. Expected criteria include `roles_responsibilities`, `access_continuity`, `bus_factor`._

---

## Update log

| Date | Editor | What changed |
|------|--------|--------------|
| 2026-05-05 | albertdobmeyer | Initial submission for the **Passing** tier; sections 1-5 fully populated; sections 6-11 scaffolded with criterion lists, to be filled in as the user reaches them in the form. |
