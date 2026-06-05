# OpenTrApp OpenSSF Best Practices Badge Answers

Working catalog of the answers entered into the OpenSSF Best Practices Badge form for bestpractices.dev project 12755. Every answer in this document is verified against the repository before it is written here. Copy each block into the matching field on the form. The text is plain and contains no em-dashes.

Badge entry: https://www.bestpractices.dev/en/projects/12755
Edit form: https://www.bestpractices.dev/en/projects/12755/edit
Target level: Passing. Starting point for this session: 18 percent.

The evidence behind these answers is recorded in docs/openssf-best-practices-application.md (criterion to evidence mapping) and docs/OpenSSF-Quiz.md (the earlier questionnaire record).

## Project metadata

### Project name
OpenTrApp

### Project home page URL
https://opentrapp.com

### Repository URL
https://github.com/albertdobmeyer/opentrapp

### Description
OpenTrApp is a free and open-source desktop application, released under the MIT license. It does not run AI agents itself. Instead, it acts as a security wrapper for the autonomous command-line AI agents that a user chooses to run on their own computer. The reference agent is OpenClaw, and the design is intended to support other command-line agents as well.

The application builds and manages a five-container security perimeter around the agent. The agent's runtime, its tools, and its add-on skills are isolated from the user's files and from the rest of the system, and they have no direct path to the internet. All of the agent's network traffic passes through a controlled egress chain that applies both an application-layer and a network-layer policy, and the agent's API credentials are held in a separate proxy container so the agent cannot read them directly. The application also provides supplementary security tools, including a scanner that inspects third-party agent skills for known risky patterns and an optional local AI component that reviews ambiguous cases. OpenTrApp does not claim to make running an autonomous agent completely safe. Its goal is to raise the cost of a compromise through layered defenses, and it documents the remaining risks in a public threat model.

### Programming languages
TypeScript, Rust, Python, Shell, JavaScript

### Description language
English (en)

### License
MIT

### CPE name (optional)
Leave blank. The project has no assigned Common Platform Enumeration name.

These General fields are auto-detected by the form and already populated, so they normally need no manual entry. They are listed here only for completeness.

## Basics (13 criteria; entry was at 7 of 13)

Note: the form auto-detects several of these from the GitHub repository (for example the FLOSS license and HTTPS), so some already show green and need no action. All 13 are written out below so the catalog is complete. For each one, set the status as shown and paste the justification. Every fact below was verified against the repository.

### description_good: the website succinctly describes what the software does
Status: Met
Justification: The repository README opens with a short paragraph that states what OpenTrApp does and the problem it solves, which is running autonomous command-line AI agents inside a contained security perimeter on the user's own computer.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/README.md

### interact: the website explains how to obtain the software, give feedback, and contribute
Status: Met
Justification: The README links to the GitHub releases page for obtaining the software and to the architecture and usage documentation. The repository issue tracker is the channel for feedback and bug reports, CONTRIBUTING.md explains how to contribute, and SECURITY.md describes the private channel for security reports.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### contribution: the contribution process is documented
Status: Met
Justification: CONTRIBUTING.md documents the contribution process. It explains how to clone and build the project, lists the test gates that continuous integration runs on every pull request and on every push to the main branch, and states that changes are proposed through GitHub pull requests.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### contribution_requirements: requirements for acceptable contributions are stated
Status: Met
Justification: CONTRIBUTING.md sets out the requirements for acceptable contributions, including agreement to the Code of Conduct, keeping all continuous integration test gates green, and following the project's repository layout and manifest conventions.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### floss_license: the software is released as FLOSS
Status: Met
Justification: OpenTrApp is released under the MIT license, which is a free and open-source software license.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/LICENSE

### floss_license_osi: the license is approved by the Open Source Initiative
Status: Met
Justification: The MIT license is approved by the Open Source Initiative.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/LICENSE

### license_location: the license is in a standard location
Status: Met
Justification: The license is stored in a file named LICENSE at the root of the repository, which is the standard location, and GitHub reports the repository license as MIT.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/LICENSE

### documentation_basics: basic documentation is provided
Status: Met
Justification: The README provides basic documentation, including the project's purpose, capabilities, requirements, installation steps, and limitations. The docs directory adds further documentation, including an architecture overview in docs/trifecta.md and a one-page perimeter explainer in docs/perimeter-explained.md.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/README.md

### documentation_interface: reference documentation describes the external interface
Status: Met
Justification: The external interface is the desktop application's graphical interface, together with the setup wizard and the Telegram chat interface used to talk to the agent. The README describes how the setup wizard guides the user through installation, credential entry, and pairing, and the capabilities section describes what the agent can and cannot do. The architecture document in docs/trifecta.md describes the internal component interfaces in more detail.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/README.md

### sites_https: the project sites support HTTPS
Status: Met
Justification: The project home page at https://opentrapp.com is served over HTTPS and returns a successful response over TLS. The source repository and the release download URLs are hosted on GitHub, which serves all content over HTTPS.
Evidence: https://opentrapp.com

### discussion: there is a searchable discussion mechanism
Status: Met
Justification: The project uses the GitHub issue tracker as its discussion mechanism for proposed changes and issues. It is searchable, every issue and comment can be referenced by its own URL, new people can join a discussion with a free GitHub account, and it does not require any proprietary client software.
Evidence: https://github.com/albertdobmeyer/opentrapp/issues

### english: the project can be used and discussed in English
Status: Met
Justification: All project documentation is written in English, and the project accepts bug reports and code comments in English.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/README.md

### maintained: the project is actively maintained
Status: Met
Justification: The project is actively maintained. The main branch receives frequent commits, the most recent release is version 0.6.0 from June 2026, and the maintainer triages issues and dependency updates on an ongoing basis.
Evidence: https://github.com/albertdobmeyer/opentrapp/commits/main

## Change Control (9 criteria; entry was at 3 of 9)

### repo_public: the source repository is public and has a URL
Status: Met
Justification: The source repository is publicly readable on GitHub and has a stable URL.
Evidence: https://github.com/albertdobmeyer/opentrapp

### repo_track: the repository tracks what changed, by whom, and when
Status: Met
Justification: The project uses Git, so every change is recorded in the commit history with the content of the change, the author, and the date and time it was made.
Evidence: https://github.com/albertdobmeyer/opentrapp/commits/main

### repo_interim: the repository includes interim versions for review
Status: Met
Justification: Development happens in the open on the main branch with frequent interim commits between releases. The repository contains the full development history, not only final release snapshots, so changes can be reviewed as they are made.
Evidence: https://github.com/albertdobmeyer/opentrapp/commits/main

### repo_distributed: a common distributed version control system is used
Status: Met
Justification: The project uses Git, which is a widely used distributed version control system.
Evidence: https://github.com/albertdobmeyer/opentrapp

### version_unique: each release has a unique version identifier
Status: Met
Justification: Each release has a unique version identifier in the form vX.Y.Z. The most recent release is version 0.6.0.
Evidence: https://github.com/albertdobmeyer/opentrapp/releases

### version_semver: releases use a recognized version numbering format
Status: Met
Justification: Releases follow Semantic Versioning 2.0.0, using the major, minor, and patch numbering format.
Evidence: https://github.com/albertdobmeyer/opentrapp/releases

### version_tags: each release is identified in version control
Status: Met
Justification: Each release is identified in version control by a Git tag of the form vX.Y.Z, for example v0.6.0.
Evidence: https://github.com/albertdobmeyer/opentrapp/tags

### release_notes: each release provides human-readable release notes
Status: Met
Justification: Each release includes human-readable release notes that summarize the major changes, both on the GitHub release page and as a release-notes file in the docs directory. For example, the version 0.6.0 release has a detailed notes body on its release page and a matching docs/release-notes-v0.6.0.md file.
Evidence: https://github.com/albertdobmeyer/opentrapp/releases/latest

### release_notes_vulns: release notes identify fixed publicly known vulnerabilities
Status: Met
Justification: When a release fixes publicly known vulnerabilities, the release notes identify them. For example, the version 0.3.2 release notes document the resolution of the RustSec advisories fixed in that release. Releases that fix no publicly known vulnerabilities do not list any.
Evidence: https://github.com/albertdobmeyer/opentrapp/releases/tag/v0.3.2

## Reporting (8 criteria; entry was at 1 of 8)

Note: three of these criteria are activity-based (response rates to reports). The repository issue tracker currently holds no human-submitted bug reports or enhancement requests, and no vulnerabilities have been reported through the security channel, so the honest answers below state that none have been received in the period. If you have in fact received and answered reports elsewhere, adjust those three accordingly.

### report_process: there is a process to submit bug reports
Status: Met
Justification: The project publishes a process for submitting reports. General bugs are reported through the GitHub issue tracker, and security issues are reported through the private process described in SECURITY.md.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/SECURITY.md

### report_tracker: an issue tracker is used for individual issues
Status: Met
Justification: The project uses the GitHub issue tracker to track individual issues.
Evidence: https://github.com/albertdobmeyer/opentrapp/issues

### report_responses: bug reports are acknowledged
Status: Met
Justification: The GitHub issue tracker is the channel for bug reports and is monitored by the maintainer. No external bug reports have been submitted in the recent period, so there are no unacknowledged reports outstanding. SECURITY.md commits to acknowledging reports within 48 hours.
Evidence: https://github.com/albertdobmeyer/opentrapp/issues

### enhancement_responses: enhancement requests are responded to
Status: Met
Justification: Enhancement requests are handled through the same GitHub issue tracker. No enhancement requests have been submitted in the recent period, so there are none outstanding.
Evidence: https://github.com/albertdobmeyer/opentrapp/issues

### report_archive: there is a public searchable archive of reports
Status: Met
Justification: The GitHub issue tracker provides a publicly available and searchable archive of reports and responses, in which each item has its own URL.
Evidence: https://github.com/albertdobmeyer/opentrapp/issues

### vulnerability_report_process: the vulnerability reporting process is published
Status: Met
Justification: The process for reporting vulnerabilities is published in SECURITY.md, which describes what information to send, where to send it, and the in-scope and out-of-scope categories. It links to the threat model for the full attacker-capability matrix.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/SECURITY.md

### vulnerability_report_private: private vulnerability reports are supported
Status: Met
Justification: Private vulnerability reports are supported through two channels. SECURITY.md provides a private email address for reports, and GitHub private vulnerability reporting is enabled on the repository.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/SECURITY.md

### vulnerability_report_response: vulnerability reports receive a timely initial response
Status: N/A (recommended). Mark Met instead if a vulnerability report was received and answered within 14 days.
Justification: No vulnerabilities have been reported through the security channel in the last six months, so this criterion is not applicable. The documented commitment in SECURITY.md is to acknowledge a report within 48 hours, which is well within the 14-day requirement.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/SECURITY.md

## Quality (13 criteria; entry was at 0 of 13)

### build: a working build system rebuilds the software from source
Status: Met
Justification: The software has a working build system that rebuilds it from source. The frontend builds with TypeScript and Vite, the Rust backend builds with Cargo, and the full desktop application is packaged with the Tauri build command. These steps are documented in CONTRIBUTING.md and run in continuous integration.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### build_common_tools: common build tools are used
Status: Met
Justification: The build uses widely adopted tools, including Cargo for Rust, npm and Vite for the frontend, and the Tauri build system for packaging.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### build_floss_tools: the software can be built using only FLOSS tools
Status: Met
Justification: All build tooling is free and open-source software, including the Rust toolchain and Cargo, Node.js, npm, Vite, and Tauri.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### test: there is an automated test suite released as FLOSS
Status: Met
Justification: The project includes automated test suites that are part of this open-source repository. They are Rust unit tests run with cargo test, frontend unit tests run with Vitest, end-to-end browser tests run with Playwright, a manifest and orchestration validation suite, and a cross-module integration suite.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### test_invocation: the test suite is invoked in a standard way
Status: Met
Justification: Each suite is invoked in the standard way for its language. The Rust tests run with cargo test, the frontend tests run with npm test using Vitest, the type checker runs with tsc, and the browser tests run with Playwright. A Makefile also groups these gates for convenience.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### test_most: the test suite covers most of the functionality
Status: Met
Justification: The test suite covers most of the project's functionality across both the Rust backend and the TypeScript frontend. It includes unit tests, strict type checking, end-to-end browser flows, manifest and orchestration validation, and cross-module contract tests, and security-relevant behavior has dedicated tests.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### test_continuous_integration: continuous integration runs the tests
Status: Met
Justification: Continuous integration runs on every pull request and on every push to the main branch using GitHub Actions. The workflow runs the Rust, frontend, type-checking, orchestration, integration, and browser test gates.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### test_policy: there is a policy to add tests for new functionality
Status: Met
Justification: The project's policy is that new functionality is added together with tests. This is stated in CONTRIBUTING.md, which lists the test gates that every change must keep green.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### tests_are_added: the test policy is followed in recent changes
Status: Met
Justification: The policy is followed in practice. Recent features were added together with their tests, and the test counts have grown release over release across the Rust, frontend, orchestration, and security suites.
Evidence: https://github.com/albertdobmeyer/opentrapp/commits/main

### tests_documented_added: the requirement to add tests is documented
Status: Met
Justification: CONTRIBUTING.md documents the expectation that changes land with tests and describes the test gates that a contributor must satisfy.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/CONTRIBUTING.md

### warnings: compiler warnings or linting tools are enabled
Status: Met
Justification: The project enables strict warning and linting tools. The frontend lint runs ESLint with a maximum of zero warnings, TypeScript is compiled in strict mode, and the Rust build and tests run in continuous integration. The cargo deny tool checks advisories, licenses, and bans.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### warnings_fixed: warnings are addressed
Status: Met
Justification: Warnings are treated as build failures rather than ignored. ESLint runs with a zero-warning threshold, so any warning fails continuous integration, and the frontend warning backlog was driven to zero and is kept there.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### warnings_strict: warning detection is maximized where practical
Status: Met
Justification: The project maximizes warning detection where practical. ESLint runs at the strict type-checked configuration with a zero-warning threshold, TypeScript uses strict mode, and cargo deny enforces strict advisory, license, ban, and source checks.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/app/src-tauri/deny.toml

## Security (16 criteria; entry was at 1 of 16)

Note: the project's primary purpose is not cryptography, and it implements no cryptographic algorithms of its own. Several cryptography criteria are therefore answered as not applicable, with an explanation. See the separate note in the Analysis section about the open code-scanning items.

### know_secure_design: a primary developer understands secure design principles
Status: Met
Justification: The project is an applied security architecture, and its design follows fail-safe defaults, least privilege, and separation of privilege. The agent runs with no default path to the internet, its API credentials are held away from it, and privilege is never loosened without an explicit human action. These principles are documented in the whitepaper, the threat model, and the architecture decision records.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/docs/whitepaper.md

### know_common_errors: a primary developer understands common implementation errors
Status: Met
Justification: The project documents the common error classes relevant to it and how they are countered, including prompt injection, command injection, path traversal, and server-side request forgery. The threat model maps attacker capabilities to mitigations, and the code applies specific countermeasures such as argument escaping, canonical path validation, and rejection of raw IP destinations at the egress boundary.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/docs/threat-model.md

### crypto_published: only published cryptographic protocols and algorithms are used
Status: Met
Justification: The project uses only standard, publicly documented cryptographic protocols and algorithms, all provided by established libraries. It implements no cryptographic algorithms of its own.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/docs/trifecta.md

### crypto_call: the software calls established crypto software rather than re-implementing it
Status: Met
Justification: The project does not re-implement cryptography. It relies on established cryptographic software, namely TLS libraries for transport, Sigstore cosign for release signatures, and the Tauri update framework for update signatures.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### crypto_floss: the cryptographic functionality is available as FLOSS
Status: Met
Justification: All cryptographic functionality the project depends on is available as free and open-source software, including the TLS stack, Sigstore cosign, and the Tauri update framework.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### crypto_keylength: default key lengths meet current minimums
Status: Met
Justification: The project relies on the default key lengths of the libraries it uses, which meet current minimum requirements, including modern TLS and Ed25519 signatures. It does not configure shorter keys.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### crypto_working: no broken cryptographic algorithms are used by default
Status: Met
Justification: The project does not depend on broken cryptographic algorithms. Release and update integrity rely on modern signature schemes, and transport uses current TLS.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### crypto_weaknesses: no algorithms with known serious weaknesses are depended on
Status: Met
Justification: The project does not depend on cryptographic algorithms or modes with known serious weaknesses, such as SHA-1 or CBC-mode SSH.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/docs/threat-model.md

### crypto_pfs: perfect forward secrecy for key agreement
Status: N/A
Justification: The project does not implement its own key-agreement protocol. Transport confidentiality is provided by standard TLS libraries, so there is no project-specific key agreement to which this criterion applies.

### crypto_password_storage: stored passwords use iterated salted hashing
Status: N/A
Justification: The software does not store passwords for authenticating external users. It has no user-account system.

### crypto_random: cryptographically secure random number generation
Status: N/A
Justification: The application does not itself generate cryptographic keys or security nonces. Signing material is managed by the release tooling and the update framework rather than generated by the application at runtime.

### delivery_mitm: delivery counters man-in-the-middle attacks
Status: Met
Justification: Release artifacts are delivered over HTTPS from GitHub and are cryptographically signed. Container images and release assets are signed with Sigstore cosign, SLSA build provenance is attached, and the desktop update framework verifies an update signature before applying an update. These measures counter man-in-the-middle tampering.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### delivery_unsigned: integrity is not based on a hash fetched over plain HTTP
Status: Met
Justification: The project does not rely on a cryptographic hash fetched over unencrypted HTTP. Integrity is established through signatures and HTTPS delivery rather than an unauthenticated hash.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/ci.yml

### vulnerabilities_fixed_60_days: known vulnerabilities are fixed within 60 days
Status: Met
Justification: There are no medium or higher severity vulnerabilities that have been publicly known for more than 60 days without a fix. Dependency advisories surfaced by cargo deny and Dependabot are addressed quickly. For example, a recent advisory in the tar crate was patched within a day.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/SECURITY.md

### vulnerabilities_critical_fixed: critical vulnerabilities are fixed rapidly
Status: Met
Justification: Critical vulnerabilities are prioritized and fixed rapidly. The project tracks advisories through cargo deny, Dependabot, and CodeQL and remediates them promptly.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/SECURITY.md

### no_leaked_credentials: no valid private credentials are exposed in the repository
Status: Met
Justification: The public repository does not contain valid private credentials. Credential files such as .env and .env.test are excluded by .gitignore, and GitHub secret scanning is enabled on the repository.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.gitignore

## Analysis (8 criteria; entry was at 0 of 8)

Note on the open code-scanning items: the remaining open alerts in the Security tab are OpenSSF Scorecard repository-posture checks surfaced as SARIF (Branch-Protection, Code-Review, Vulnerabilities, and the CII-Best-Practices badge itself, which clears once this badge is earned). They are not exploitable code vulnerabilities. The two CodeQL unused-variable warnings (false positives, since the variable is used in an inline format string) and the three dependency-pin advisories on a developer devcontainer script have been dismissed.

### static_analysis: a static analysis tool is applied
Status: Met
Justification: The project applies CodeQL static analysis. It runs in continuous integration over the JavaScript, TypeScript, Rust, and GitHub Actions code in the repository.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/codeql.yml

### static_analysis_common_vulnerabilities: the tool looks for common vulnerabilities
Status: Met
Justification: CodeQL uses query packs that look for common vulnerability patterns in the analyzed languages.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/codeql.yml

### static_analysis_fixed: medium and higher findings are remediated
Status: Met
Justification: No exploitable vulnerability identified by static code analysis is outstanding. The two note-severity CodeQL unused-variable warnings were false positives, since the variable is used in an inline format string, and have been dismissed. The three dependency-pin advisories on a developer devcontainer script have also been dismissed as developer tooling. The remaining open items in the code-scanning interface are OpenSSF Scorecard repository-posture checks surfaced as SARIF, none of which are exploitable vulnerabilities in the delivered software.
Evidence: https://github.com/albertdobmeyer/opentrapp/security/code-scanning

### static_analysis_often: static analysis runs often
Status: Met
Justification: Static analysis runs frequently. CodeQL runs on every push and every pull request and additionally on a weekly schedule.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/codeql.yml

### dynamic_analysis: a dynamic analysis tool is applied
Status: Met
Justification: The project applies fuzzing as dynamic analysis. A fuzzing workflow runs cargo fuzz against parsing and argument-handling surfaces.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/fuzz.yml

### dynamic_analysis_unsafe: memory-safety detection is used during dynamic analysis
Status: Met
Justification: The fuzzing harness builds with the address sanitizer and coverage instrumentation, which detect memory-safety problems. The application code is written in memory-safe Rust and TypeScript.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/fuzz.yml

### dynamic_analysis_enable_assertions: assertions are enabled during dynamic analysis
Status: Met
Justification: The fuzzing builds enable sanitizer checks and debug assertions during analysis.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/fuzz.yml

### dynamic_analysis_fixed: medium and higher dynamic-analysis findings are remediated
Status: Met
Justification: There are no outstanding medium or higher severity findings from dynamic analysis. Fuzzing findings are addressed when they arise.
Evidence: https://github.com/albertdobmeyer/opentrapp/blob/main/.github/workflows/fuzz.yml
