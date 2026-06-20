# Known advisories & Scorecard posture

This document explains the security advisories OpenTrApp **knowingly accepts**
(with rationale) and how to read the project's [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/albertdobmeyer/opentrapp).
It exists so that a "low" Scorecard *Vulnerabilities* number is not mistaken for
negligence — most of it is unfixable upstream noise, and we say so plainly.

The machine-readable source of truth is
[`app/src-tauri/deny.toml`](../app/src-tauri/deny.toml) (`[advisories].ignore`),
enforced by `cargo deny check` in [`supply-chain.yml`](../.github/workflows/supply-chain.yml).

---

## The 23 in the Scorecard *Vulnerabilities* count (2026-06-13)

Scorecard's external OSV scan reports **23**. They split cleanly:

- **4 genuinely fixable → fixed this session** (2 npm dev-tooling, 2 Python test deps). See *Resolved* below.
- **19 accepted** — `unmaintained` / `unsound` *warnings* on **transitive** Rust
  crates we do not control. **None are exploitable vulnerabilities.** Scorecard's
  OSV scan **cannot read `deny.toml`**, so the count stays at 19 regardless; our
  local/CI audit is clean and the acceptance is auditable.

**Re-verified 2026-06-16 (Scorecard 7.7/10, commit `63d4426`; count now 21 — the
`rand` advisory is tallied at both 0.7.3 and 0.8.5).** Two findings this pass:

1. **No dependency bump at our layer fixes any of them — verified, not assumed.** Each
   advisory was traced to its root: the GTK3 set and `glib` come via `tauri`/`wry`;
   `rand`, `fxhash` and the `unic-*` chain come via `tauri-utils`' build-time codegen
   (`html5ever`→`rand`, `selectors`→`fxhash`, `urlpattern`→`unic-*`); `idna` is already
   on the clean 1.x / `icu4x` line. `cargo update` moves **0** packages. The only
   resolution is removing the Tauri tree — the **Phase 3 de-Tauri cutover** — or an
   upstream GTK4 migration in `tauri-apps/wry`.
2. **All 21 are confined to the optional desktop-GUI binary; the perimeter spine is
   advisory-clean.** `cargo tree` shows the `opentrapp` GUI crate pulling the full Tauri
   stack, while the crates.io-published **`opentrapp-core`** and the headless
   **`opentrapp-daemon`** — the code that actually runs the containment — contain **zero**
   GTK / WebKit / `wry` / `tauri-utils` / `unic-*` / `rand 0.7–0.8` crates (their only
   `tauri` string is the `src-tauri/` directory *path*). So the Scorecard *Vulnerabilities*
   count is entirely the optional GUI's deprecated GTK3 bindings; the daemon + core have none.

## Accepted Rust advisories (all *warnings*, not vulnerabilities)

The machine-readable acceptance for the `unmaintained` set is in `deny.toml`
(`[advisories].ignore`); `cargo deny check` is clean against it. The two `unsound`
entries (glib, rand) are **not** in that list — cargo-deny's advisory-DB view does
not match our transitive version constraints, so an `ignore` there would emit
spurious "advisory-not-detected" warnings on every CI run. They are still detected
by `cargo audit` / OSV and accounted for here.

| Source | Crates | IDs | Why accepted |
|--------|--------|-----|--------------|
| **Tauri 2 GTK3 webview stack** | `gtk`, `gdk`, `atk`, `gdkx11`, `gdk-sys`, `gtk-sys`, `gtk3-macros`, … | RUSTSEC-2024-0411…0420 (10) | gtk-rs GTK3 bindings are unmaintained. Pulled by `tauri`/`wry` on **Linux only**. No remediation at our layer — clears when the Tauri ecosystem migrates to GTK4 (tracked upstream in `tauri-apps/wry`). |
| **Transitive unmaintained crates** | `proc-macro-error`, `fxhash`, `unic-*` | RUSTSEC-2024-0370, RUSTSEC-2025-0057, RUSTSEC-2025-0075/0080/0081/0098/0100 (7) | Build-time / deep transitive deps with no first-party remediation; await upstream migration (e.g. `proc-macro-error2`, `selectors`→`ahash`, `unic-*`→`icu4x`). |
| **Transitive *unsound* warnings** | `glib`, `rand` | RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g, RUSTSEC-2026-0097 / GHSA-cq8v-f236-94qc (2) | Unsoundness on code paths we never enter: glib's `VariantStrIter` (we call no such API; transitive via `tauri`) and rand's `rng()`-with-custom-global-logger (we configure no custom logger). Not exploitable in our usage; await upstream crate fixes. |

That is **10 + 7 + 2 = 19** — exactly the Rust IDs in the Scorecard list. They
cannot be removed without an upstream change and are re-evaluated on every
dependency bump.

## Resolved this session (2026-06-13)

The four fixable OSV findings were eliminated — all in **dev / test** tooling, none
shipped in the desktop app:

| ID | Package | Ecosystem | Fix |
|----|---------|-----------|-----|
| `GHSA-58qx-3vcg-4xpx` | `ws` (via `jsdom`) | npm dev | `overrides: { "ws": "^8.20.1" }` |
| `GHSA-jxxr-4gwj-5jf2` | `brace-expansion` 5.x (via `nyc`→`glob`→`minimatch`) | npm dev | `overrides: { "brace-expansion@5": "^5.0.6" }` — version-scoped so the unrelated `brace-expansion@1.x` under eslint's `minimatch@3` is left untouched |
| `GHSA-6w46-j5rx-g56g` | `pytest` | PyPI (e2e tests) | floor raised `>=8.0` → `>=9.0.3` in `tests/e2e-telegram/requirements.txt` |
| `GHSA-mf9w-mj56-hr94` | `python-dotenv` | PyPI (e2e tests) | floor raised `>=1.0` → `>=1.2.2` (also `pytest-asyncio>=1.4`, which supports pytest 9) |

After the npm overrides, `npm audit` reports **0 vulnerabilities** (prod *and* dev).

## npm

Production `npm audit --omit=dev` is **clean (0)**. The prior real advisory,
`GHSA-2j2x-hqr9-3h42` (react-router open-redirect), was fixed by bumping
`react-router-dom` to `^6.30.4`. The two dev-only advisories above are now fixed
via `overrides`; remaining dev advisories are tracked by Dependabot.

---

## How to read the Scorecard

A literal 10/10 is **not achievable for a solo desktop app**, and chasing some
checks would be counterproductive. Honest interpretation:

| Check | Why it scores low | Reality |
|-------|-------------------|---------|
| **Vulnerabilities** | Scorecard's external OSV scan counts the 19 GTK3/unmaintained/unsound advisories above and **cannot read `deny.toml`** | Not exploitable; unfixable upstream. Every *fixable* finding (2 npm, 2 Python) is resolved — see *Resolved this session*. |
| **Code-Review** | Counts *approved* changesets by a second person | Solo project — no second approver exists. Only improves with a co-maintainer. |
| **Contributors** | Wants commits from ≥2 companies/orgs | Solo project. Improves organically. |
| **Branch-Protection** | Some settings disabled | The safe settings are enabled; *required approvals* are intentionally off because they would block the sole maintainer's own merges. |
| **Packaging** | Looks for a package-registry publish workflow | A desktop app has no registry target; releases ship as signed GitHub Release assets (SBOM + cosign + SLSA). Accepted at the heuristic's expense. |

The checks that reflect real engineering hygiene — Dangerous-Workflow,
Token-Permissions, SAST, Security-Policy, License, Fuzzing, Maintained, CI-Tests —
are at maximum. See [`threat-model.md`](threat-model.md) for the application's
actual security posture.

### Improvable-check work (2026-06-13) — one reached max, one is capped

| Check | Was | Gap (from Scorecard's own detail) | Outcome |
|-------|-----|-----------------------------------|---------|
| **Signed-Releases** | 8 | All recent releases lacked a *provenance* asset — `attest-build-provenance` wrote the attestation to GitHub's store, but no provenance **file** was attached to the release | ✅ **→10 on the next tagged release.** The release workflow now copies the attestation bundle to `provenance-<platform>.intoto.jsonl` and uploads it as a release asset (Scorecard matches the `.intoto.jsonl` suffix). The score climbs as provenance-bearing releases enter the 5-release window; existing releases are not retro-fixed. |
| **Pinned-Dependencies** | 9 | 3 unpinned commands in `workloads/skills/.devcontainer/setup.sh` (2 npm global installs, 1 pip) | 🔶 **Stays at 9 — npm portion not honestly fixable.** The pip line is now hash-pinned (`pip install --require-hashes -r requirements.txt`, cp312 wheel hashes verified via `pip download`) ✅. The **two `npm install -g` lines cannot reach Scorecard's bar**: per `isNpmUnpinnedDownload` in `ossf/scorecard`, npm is "pinned" **only** for `npm ci` (lockfile-verified) or a git URL anchored to a commit hash — a semantic-version pin (`npm@11.17.0`) is *not examined* and counts as unpinned. `npm ci` is not possible here (the devcontainer's `package.json` has no deps / lockfile and `molthub` is the workbench's own CLI, not a registry package), and a git+hash URL would require a real `molthub` repo. Neither is achievable without fabrication, so we accept 9. The version pins are kept for reproducibility, not score. |

> **Correction (2026-06-14):** PR #86 originally claimed Pinned-Dependencies
> would reach 10. That was wrong — Scorecard does not credit npm version-pins
> (verified against its source). The score stays at 9; only the pip line and
> Signed-Releases improved. Recorded here rather than silently left to surface
> on the next rescan.

The remaining non-maximal checks (Vulnerabilities, Code-Review, Contributors,
Branch-Protection, Pinned-Dependencies' npm lines, CII Gold) are the structural /
upstream / people caps described above.
