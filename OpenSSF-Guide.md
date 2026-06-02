> **EXISTING ENTRY — edit, do not re-add.** This project is already registered at
> bestpractices.dev as **project #12755** (created 2026-05-05 under the project's
> former name, *Lobster-TrApp*, before the OpenTrApp rebrand). A name/domain change
> does **not** require a new application — the entry is keyed to the GitHub repo and is
> editable in place. Re-adding would create a duplicate and orphan the ~18% already recorded.
>
> **Edit it here:** https://www.bestpractices.dev/en/projects/12755/edit (sign in with
> the GitHub account that owns the repo). Update these four fields first:
> 1. Project name: `Lobster-TrApp` → `OpenTrApp`
> 2. Home page URL: `https://lobster-trapp.com` → `https://opentrapp.com`
> 3. **Repository URL:** `…/lobster-trapp` → `https://github.com/albertdobmeyer/opentrapp`
>    — this is the field Scorecard's `CII-Best-Practices` check keys on; until it points
>    at `opentrapp`, the badge cannot credit this repo.
> 4. Description: replace the stale "four-container / OpenClaw" text with the current
>    five-container, agent-agnostic description (see `docs/OpenSSF-Quiz.md` §1).
>
> Then walk the remaining criteria using `docs/openssf-best-practices-application.md`
> (refreshed to v0.6.0) to push 18% → Passing. The checklist below is the original
> 2026-05 plan, kept for the criteria mapping — its "Add a project" framing is
> superseded by the edit-in-place guidance above.

---

 Self-attest at OpenSSF Best Practices. This is the last big Scorecard item that's purely yours to do.

  1. Go to https://www.bestpractices.dev/
  2. Sign in with GitHub
  3. Open the **existing** project #12755 at https://www.bestpractices.dev/en/projects/12755/edit — do NOT click "Add a project" (see the banner above)
  4. Fill out the questionnaire. Most criteria you already meet:
    - ✅ Free/libre/open-source license (MIT)
    - ✅ Source repository public + version controlled
    - ✅ Bug-tracking system (GitHub Issues)
    - ✅ Code of Conduct
    - ✅ Security policy (SECURITY.md)
    - ✅ Vulnerability reporting process
    - ✅ Tests in CI
    - ✅ Static analysis (CodeQL)
    - ✅ Cryptographic signatures on releases (cosign + SLSA — once v0.4.0 cuts)
    - ✅ HTTPS for project sites

    - ✅ Security policy (SECURITY.md)
    - ✅ Vulnerability reporting process
    - ✅ Tests in CI
    - ✅ Static analysis (CodeQL)
    - ✅ Cryptographic signatures on releases (cosign + SLSA — once v0.4.0 cuts)
    - ✅ HTTPS for project sites

  For each criterion, paste the relevant URL or filename as evidence. The form auto-grants "Passing" badge level as soon as you meet the bar.

  5. Once issued, copy the badge markdown they give you and append it to the README badge row, e.g.:
  [![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12755/badge)](https://www.bestpractices.dev/projects/12755)

  Earning the Passing badge takes Scorecard's CII-Best-Practices from 0 → 5. Silver = 7. Gold = 10. Most projects start at Passing and graduate over time.

  Sometime soon (auto-resolves a check)

  Cut v0.4.0. The cosign + SLSA pipeline is wired in ci.yml but has never produced a tagged release. The next git tag v0.4.0 && git push origin v0.4.0 will:
  - Build installers for all four platforms
  - Generate per-platform CycloneDX SBOMs
  - Sign each artifact with cosign (sigstore keyless)
  - Produce a SLSA Build Level 2 build-provenance attestation
  - Attach all of that to the GitHub Release

  That immediately earns Signed-Releases 0 → 10 on the next Scorecard pass. Follow RELEASING.md step-by-step.

  If you don't want to cut a real release yet, you can validate the pipeline by tagging a release-candidate (v0.4.0-rc.1) — same machinery runs.

  Background work (no action from you)

  These keep ticking on their own:
  - Dependabot PRs are accumulating. Triage them weekly — most are minor bumps that merge cleanly. The vite 5 → 8 and jsdom 25 → 29 ones currently in your queue are major-version bumps with breaking changes; close them and let Dependabot propose      
  smaller increments.
  - Maintained check earns automatically once the repo is 90 days old (currently flagged because it's < 90 days).
  - Code-Review check accrues credit as you merge PRs through the new ruleset.

  What I'd do in the next hour, in order

  1. (2 min) gh workflow run scorecard.yml and watch it complete
  2. (1 min) Refresh the public Scorecard page to confirm Branch-Protection improved
  3. (1 min) Test that direct push to main is rejected
  4. (45 min) OpenSSF Best Practices questionnaire
  5. (5 min) Add the Best Practices badge to README

  After that you're at the practical ceiling without cutting a release or waiting for the 90-day Maintained threshold.

  Once we hit the ceiling

  When you're ready, ping me to:
  - Help with v0.4.0 release notes drafting
  - Set up commit signing (for the deferred "Require signed commits" rule, which earns Branch-Protection's last point)
  - Triage the open Dependabot PRs to chip away at Vulnerabilities
  - Sketch a fuzz harness for the manifest parser if you decide to invest in Fuzzing