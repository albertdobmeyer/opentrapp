# Handoff — Code-quality and code-hygiene workstream

**Created:** 2026-05-05
**Workstream:** Drive the strict code-scanning gate to zero open issues and zero warnings; identify and rewrite especially poor code where doing so produces a more elegant and maintainable result.

This handoff is scoped to the code-quality workstream. The general session-state document remains [`docs/handoff.md`](handoff.md); do not overwrite it without coordinating with the parallel-agent workstream.

## 1. Repository state at the start of this workstream

- **Branch:** `main` clean and aligned with `origin/main` at `9fee9b8` ("fix(ci): replace github/codeql-action tag-object SHA with commit SHA (#17)").
- **Submodules:** synchronised against the recorded pointers (`openskill-forge`, `openagent-social`, `opencli-container`).
- **Open pull requests:** zero.
- **Preserved local-only branch:** `feat/brand-assets-blue-gradient` carries an unmerged commit (`596e8d2`) introducing the gradient FontLogo, the OpenTrApp-Blue accent token, and supporting render scripts. Either land it as a PR or delete it deliberately; do not let it drift further out of date.

## 2. The constraint a code-quality session must work within

Direct pushes to `main` are blocked by a branch ruleset. Every change goes through a pull request. The ruleset enforces, in addition to the standard PR flow:

- All eight CI checks must pass on the PR's HEAD commit (`Frontend (tsc + vitest)`, `Rust (check + test)`, `Orchestration (42 checks)`, `Integration tests (cross-module contracts)`, `Playwright smoke tests`, plus the three CodeQL `Analyze` jobs)
- `strict_required_status_checks_policy = true` — the PR branch must be on top of the latest `main` when merged
- A code-scanning gate keyed to CodeQL with `errors_and_warnings + all alerts` thresholds — any open CodeQL finding on the PR will block the merge
- `Claude` and `albertdobmeyer` are on the bypass list; `--admin` is available for cases where the rebase cascade is genuinely impractical, but should not be used to bypass real CodeQL findings (that would defeat the purpose of this workstream)

## 3. The actual surface area to improve

A snapshot of open code-scanning alerts taken at the start of this workstream:

```
6 open alerts, all from the "Scorecard" tool:
  Scorecard / VulnerabilitiesID    severity=error  security=high    (transitive Rust deps awaiting upstream patches)
  Scorecard / MaintainedID         severity=error  security=high    (repo age — auto-resolves at 90 days)
  Scorecard / CodeReviewID         severity=error  security=high    (earns as merged-via-PR commits accumulate)
  Scorecard / BranchProtectionID   severity=error  security=high    (the bypass list takes a few points off)
  Scorecard / FuzzingID            severity=error  security=medium  (no fuzz harness yet)
  Scorecard / CIIBestPracticesID   severity=error  security=low     (self-attestation pending at bestpractices.dev)

CodeQL alerts: 0
```

**The headline:** CodeQL itself currently reports the codebase as clean. The "strict scanner finds zero issues" goal is *already met* for CodeQL. The Scorecard findings above are meta-process items, not code defects, and the ruleset's code-scanning gate is keyed to CodeQL — they do not block PR merges.

This means a code-quality session should not chase the alert count down (it is already at zero for the relevant tool). It should instead expand the surface area of analysis: run additional linters that produce more information than CodeQL's default packs, and act on the findings.

## 4. Where to look for actual code issues

The CI does not currently run the most aggressive linters. A code-quality session should run them locally first, fix what they find, then consider adding them to CI as required checks:

### Rust (in `app/src-tauri/`)

```bash
cd app/src-tauri
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --all-features -- -W clippy::pedantic -W clippy::nursery 2>&1 | tee clippy-pedantic.log
cargo fmt --check
cargo machete                                     # finds unused dependencies
cargo +nightly udeps                              # cross-checks unused dependencies (nightly only)
```

The `pedantic` and `nursery` lint groups will produce many findings; treat them as a triage pool, not a hard gate. Fix the ones that genuinely improve readability or correctness; ignore the stylistic ones that fight idiomatic code.

### TypeScript / React (in `app/`)

```bash
cd app
npx tsc --noEmit                                  # already clean per CI
npx eslint . --ext .ts,.tsx --max-warnings 0      # current lint config
npx eslint . --ext .ts,.tsx --rule 'complexity: [warn, 10]' --rule 'max-depth: [warn, 4]'
npx eslint-plugin-security                        # if/once added — surfaces security-relevant patterns
npx knip                                          # finds unused exports, files, dependencies
```

### Both stacks

```bash
npx markdownlint-cli2 "**/*.md" "#node_modules"   # markdown hygiene across docs
git ls-files | xargs -I{} grep -l "TODO\|FIXME\|HACK\|XXX" {} 2>/dev/null  # tag-debt inventory
```

### Optional: run CodeQL locally with broader query packs

CI runs CodeQL with `security-extended,security-and-quality`. To dig deeper, run it locally with additional packs:

```bash
# Install the CodeQL CLI from https://github.com/github/codeql-cli-binaries
codeql database create db --language=javascript-typescript --source-root=app/src
codeql database analyze db codeql/javascript-queries:codeql-suites/javascript-code-scanning.qls --format=sarif-latest --output=results.sarif
codeql database analyze db codeql/javascript-queries:codeql-suites/javascript-security-extended.qls --format=sarif-latest --output=results-extended.sarif
```

Repeat for `--language=rust`. Compare against CI's findings to identify queries CI is not running.

## 5. Triage workflow for findings

For each finding the linters produce, classify into one of three buckets:

1. **Fix** — straightforward correctness or clarity improvement. Land in a small focused PR.
2. **Refactor candidate** — symptom of a structurally bad section (high cyclomatic complexity, awkward control flow, nested conditionals, long parameter lists, repeated patterns). Capture in a roadmap item; rewrite as a separate PR.
3. **Justified suppression** — finding does not apply to the code's actual context (test fixture, generated code, false positive). Suppress with the linter's `allow`/`disable-next-line`/`expect` mechanism, **always with a comment giving the reason**. Never suppress silently.

The third bucket is unavoidable but should remain small. If the suppression list grows, it is a sign that the configured rule set is too aggressive for the codebase.

## 6. When to rewrite a section

The user's brief permits rewriting "especially bad" sections. The qualifying criteria, in order of weight:

- **Cyclomatic complexity ≥ 15** in a single function (run `npx eslint --rule 'complexity: [error, 15]'` to enumerate)
- **Cognitive complexity** noticeably higher than cyclomatic (nested conditionals, mixed control structures)
- **Parameter lists ≥ 6** without a builder pattern or a typed config object
- **Functions exceeding ~80 lines** in TS/TSX or ~100 lines in Rust without internal section markers or extractable sub-functions
- **Recurring duplication** of the same pattern in three or more places without an obvious shared abstraction

When rewriting, preserve external behaviour exactly. The PR description must include the before/after metric (complexity, line count, parameter count) and a one-paragraph rationale. If existing tests do not cover the section, add tests *first* in a separate commit, then refactor.

## 7. PR mechanics under the strict ruleset

The serial-merge pain observed earlier in this project is intrinsic to `strict_required_status_checks_policy = true`: every merge moves `main`, every other open PR becomes "behind", every "behind" PR must be rebased, and rebasing triggers fresh CI on the slowest gate (CodeQL Rust, ~7 minutes per run). Multi-PR sessions therefore have an unavoidable serial component.

Two practical patterns to minimise it:

- **Land code-quality work in small, sequential PRs.** Open one, wait for it to merge, then open the next. Avoid having more than one or two PRs in flight at a time.
- **Group changes by file or module.** A PR that touches twenty files in five modules will take longer to review and land than five PRs that each touch four files in one module. Smaller PRs also keep the blast radius of a regression contained.

The standard daily flow:

```bash
git checkout main
git pull
git checkout -b refactor/<short-subject>
# work
git push -u origin refactor/<short-subject>
gh pr create --fill
gh pr merge --squash --auto --delete-branch        # auto-merge fires when CI is green
```

If the rebase cascade becomes a real obstacle (e.g. five queued PRs after a Dependabot wave), the maintainer can temporarily relax `strict_required_status_checks_policy` for a session and re-tighten it after the queue is drained. This is documented in [`SCORECARD.md`](../SCORECARD.md) §"Posture summary".

## 8. References for the next session

- **Project conventions:** [`CLAUDE.md`](../CLAUDE.md) — non-negotiable architectural rules, the manifest contract, the user-vs-developer terminology rule
- **Threat model:** [`docs/threat-model.md`](threat-model.md) — relevant when judging whether a refactor changes any security-affecting surface
- **Release process:** [`RELEASING.md`](../RELEASING.md) — version bumps, tag procedure, release-note conventions
- **Scorecard posture:** [`SCORECARD.md`](../SCORECARD.md) — what is earned, what is pending, what auto-resolves
- **Saved memories:** brand colours (OpenTrApp-Green / -Red / -Blue), documentation tone (academic, professional, objective), parallel-agent scope (do not touch project-professionalisation files without checking first), brand-asset locations
- **The orphan branch:** `feat/brand-assets-blue-gradient` (`596e8d2`) — decide whether to PR it or delete it before starting code-quality work; do not let it interact with refactors that touch the same files

## 9. First commands when the session begins

```bash
# Verify state
cd /b/A5DS-HQ/REPOS/opentrapp
git fetch origin
git status                                         # should show: clean, on main, up to date with origin/main
gh pr list --state open                            # should show: empty, or only the brand-asset PR if turned into one

# Take a baseline reading of code-quality signals
cd app && npx tsc --noEmit
cd app && npx eslint . --ext .ts,.tsx --format unix > ../baseline-eslint.log 2>&1
cd app/src-tauri && cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee ../baseline-clippy.log
cd app/src-tauri && cargo clippy --all-targets --all-features -- -W clippy::pedantic 2>&1 | tee ../baseline-clippy-pedantic.log

# Inventory the technical-debt surface
git ls-files '*.ts' '*.tsx' '*.rs' | xargs grep -nE 'TODO|FIXME|HACK|XXX' 2>/dev/null > ../debt-inventory.log
```

These baselines anchor the work — every subsequent PR can cite the count it reduces.
