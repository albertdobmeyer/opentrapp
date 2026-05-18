# Contributing to OpenTrApp

Thank you for considering a contribution. This document covers how to clone, build, test, and submit changes. The architectural rules and the manifest contract are documented separately in [`CLAUDE.md`](CLAUDE.md); this document covers the contributor-facing workflow.

By participating in this project, you agree to follow our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## Cloning the repository

The project uses three git submodules (`openclaw-vault`, `clawhub-forge`, `moltbook-pioneer`). Clone with `--recurse-submodules` so they are populated on first clone:

```bash
git clone --recurse-submodules https://github.com/albertdobmeyer/opentrapp.git
```

If you cloned without `--recurse-submodules`, populate the submodules afterwards:

```bash
cd opentrapp
git submodule update --init --recursive
```

The default URLs are HTTPS. If you prefer SSH (and have an authenticated key configured for GitHub), edit the `.gitmodules` file or run:

```bash
git config --global url."git@github.com:".insteadOf "https://github.com/"
```

before the clone. The reverse mapping works equally well if you start with SSH URLs and need HTTPS.

## Submodule discipline

Each component exists in two places on a contributor's machine:

- **Standalone clone** at `~/<component>/` — for focused work on a single component
- **Submodule copy** at `~/opentrapp/components/<component>/` — for orchestrator integration

These are independent git checkouts. Changes in one do not propagate to the other automatically.

Sync workflow after a submodule change has been pushed to its component remote:

```bash
cd components/<component>
git pull
cd ../..
git add components/<component>
git commit -m "Update <component> submodule reference"
```

Please avoid modifying a component's `component.yml`, scripts, or library code directly inside the submodule directory of this repository without also pushing the change to the component's own remote. The submodule pointer is a record of *which commit* of the component this orchestrator integrates against; making changes only inside the submodule produces a divergent copy that is awkward for other contributors to work with.

Please also avoid force-pushing submodule references — other clones will need to re-fetch from a different history.

## Building from source

```bash
# Frontend (React + TypeScript)
cd app
npm install
npm run dev                       # Vite dev server

# Backend (Rust)
cd app/src-tauri
cargo build

# Full desktop bundle for the host platform
cd app
npm run tauri build
```

Tauri's per-platform prerequisites (WebKitGTK on Linux, Xcode CLI tools on macOS, the Microsoft C++ build tools on Windows) need to be installed for the desktop bundle to build. The full list is at [tauri.app](https://tauri.app/start/prerequisites/).

## Test gates

Every pull request keeps these checks green. CI runs them on every push to `main` and on every pull request:

```bash
# 1. Rust unit tests (currently 56)
cd app/src-tauri && cargo test --lib

# 2. Vitest frontend unit tests (currently 74)
cd app && npm test -- --run

# 3. TypeScript strict-mode checking
cd app && npx tsc --noEmit

# 4. End-to-end browser tests (currently 25)
cd app && npx playwright test

# 5. Manifest and orchestration validation (42 checks; expects 0 warnings)
bash tests/orchestrator-check.sh
```

A pull request that doesn't pass these checks won't be merged by CI. Running them locally before opening the pull request will save you time — debugging a failure in CI is significantly slower than debugging it on your machine.

## The 28 reserved-term list

The user-facing surface of the desktop application keeps developer concepts out of view. The mappings between developer terms and user-facing terms are in [`GLOSSARY.md`](GLOSSARY.md) §1.

The reserved-term list is enforced by [`app/e2e/user-facing.spec.ts`](app/e2e/user-facing.spec.ts) on every commit. As of this writing the array contains 28 terms. If you encounter a new developer-jargon term during your work that needs to be exposed to the user, please either:

1. Replace it with its user-facing mapping from [`GLOSSARY.md`](GLOSSARY.md), or
2. Add it to the `BANNED_TERMS` array in `app/e2e/user-facing.spec.ts` if it should remain out of user-visible text (and update the count documented in [`CLAUDE.md`](CLAUDE.md) §3).

The check fails if any reserved term appears in any user-mode page's visible text.

## Pull-request workflow

We follow an issue-first workflow:

1. **Open an issue** describing the change you want to make. For non-trivial changes (new features, schema modifications, security-impacting changes), please wait for maintainer feedback before starting implementation. For small fixes (typo, documentation correction, obvious bug), it is fine to open the pull request directly.
2. **Fork the repository** and create a topic branch. Suggested branch names: `fix/<short-description>`, `feat/<short-description>`, `docs/<short-description>`, `chore/<short-description>`.
3. **Make the change** and run the test gates locally.
4. **Open a pull request** using the template at [`.github/pull_request_template.md`](.github/pull_request_template.md). The template walks you through the questions a reviewer will ask anyway; filling it in up front speeds up review.
5. **Address review feedback** by pushing additional commits to the same branch. Please avoid force-pushing during review (it makes incremental review harder); the maintainer will squash on merge.
6. **The maintainer merges** when CI is green and the review is approved.

## Commit-message style

We follow a lightweight conventional-commits convention:

```
<type>: <short description>

<optional body explaining the why, in present tense>
```

Types in current use: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `perf`. Prefer the *why* over the *what* in the body — the diff already shows the *what*.

## Running the perimeter for manual verification

```bash
# Start the four-container perimeter
podman compose up -d

# Verify all containers are up
podman compose ps

# Stop the perimeter
podman compose down
```

Substitute `docker compose` for `podman compose` if you use Docker. The compose file is at the repository root ([`compose.yml`](compose.yml)).

For a full live-perimeter dogfood (the loop the maintainer runs before each release), see [`docs/specs/2026-04-28-dogfood-walkthrough-findings.md`](docs/specs/2026-04-28-dogfood-walkthrough-findings.md).

## Architectural rules to read before a non-trivial change

Before opening a non-trivial pull request, please read [`CLAUDE.md`](CLAUDE.md) end-to-end. The points most often missed by new contributors:

- **Generic-backend constraint** — the Tauri backend reads manifests and executes what they declare; component-specific logic lives in the components, not in the backend.
- **Manifest schema alignment** — the schema lives in three places (`schemas/component.schema.json`, `app/src-tauri/src/orchestrator/manifest.rs`, `app/src/lib/types.ts`) which change together.
- **Submodule discipline** — see above.
- **The 28 reserved-term list** — see above.

## Security-sensitive contributions

Please refrain from opening a public pull request for a vulnerability fix until the issue has been disclosed privately and a coordinated remediation timeline has been agreed. The full reporting process is in [`SECURITY.md`](SECURITY.md).

If your contribution touches the security-relevant path — the perimeter compose file, the proxy, the skill scanner, the workflow runner, the file-validation logic, or any of the verification scripts — please flag it explicitly in the pull-request body so a more careful review is applied.

## Releasing (maintainer-only)

A release is cut by tagging `vX.Y.Z` on `main`. CI builds release artefacts for the four supported platforms, attaches them to a draft release, and lets the maintainer publish. The release-notes template is in [`docs/release-notes-v0.3.0.md`](docs/release-notes-v0.3.0.md) (use the most recent prior release as a template).

## Asking for help

- **General questions** — open a GitHub Discussion on the [`opentrapp` repository](https://github.com/albertdobmeyer/opentrapp/discussions).
- **Bug reports** — open a GitHub Issue with reproduction steps.
- **Security reports** — see [`SECURITY.md`](SECURITY.md).
- **Direct contact with the maintainer** — `albertdobmeyer@proton.me` for security-sensitive matters; otherwise the public channels are preferred so other contributors can benefit from the answer.

This is a small project and the maintainer is solo; thank you for your patience.
