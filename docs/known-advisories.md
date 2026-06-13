# Known advisories & Scorecard posture

This document explains the security advisories OpenTrApp **knowingly accepts**
(with rationale) and how to read the project's [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/albertdobmeyer/opentrapp).
It exists so that a "low" Scorecard *Vulnerabilities* number is not mistaken for
negligence — most of it is unfixable upstream noise, and we say so plainly.

The machine-readable source of truth is
[`app/src-tauri/deny.toml`](../app/src-tauri/deny.toml) (`[advisories].ignore`),
enforced by `cargo deny check` in [`supply-chain.yml`](../.github/workflows/supply-chain.yml).

---

## Accepted Rust advisories (all *warnings*, not vulnerabilities)

`cargo audit` reports ~20 RUSTSEC advisories in the dependency tree. **None are
exploitable vulnerabilities** — they are `unmaintained` / `unsound` warnings on
**transitive** crates we do not control. `cargo audit` exits 0 on them; they are
listed in `deny.toml` so `cargo deny check` is clean and the acceptance is
auditable.

| Source | Crates | Why accepted |
|--------|--------|--------------|
| **Tauri 2 GTK3 webview stack** (RUSTSEC-2024-0411…0420) | `gtk`, `gdk`, `atk`, `gdkx11`, `gdk-sys`, `gtk-sys`, `gtk3-macros`, … | gtk-rs GTK3 bindings are unmaintained. Pulled by `tauri`/`wry` on **Linux only**. No remediation at our layer — clears when the Tauri ecosystem migrates to GTK4 (tracked upstream in `tauri-apps/wry`). |
| **Transitive unmaintained crates** | `proc-macro-error`, `fxhash`, `unic-*` | Build-time / deep transitive deps with no first-party remediation; await upstream migration (e.g. `proc-macro-error2`, `selectors`→`ahash`, `unic-*`→`icu4x`). |

These cannot be removed without an upstream change. They are re-evaluated on
every dependency bump (each `deny.toml` entry carries a reason).

> **Resolved & removed:** `RUSTSEC-2024-0429` (glib `VariantStrIter` unsoundness)
> was dropped from the ignore list on 2026-06-13 — glib was bumped past the
> affected version, so it is no longer detected.

## npm

Production `npm audit --omit=dev` is **clean (0)**. The one prior real advisory,
`GHSA-2j2x-hqr9-3h42` (react-router open-redirect), was fixed by bumping
`react-router-dom` to `^6.30.4`. Dev-only advisories are tracked by Dependabot.

---

## How to read the Scorecard

A literal 10/10 is **not achievable for a solo desktop app**, and chasing some
checks would be counterproductive. Honest interpretation:

| Check | Why it scores low | Reality |
|-------|-------------------|---------|
| **Vulnerabilities** | Scorecard's external OSV scan counts the GTK3/unmaintained advisories above and **cannot read `deny.toml`** | Not exploitable; unfixable upstream. The one real npm vuln is fixed. |
| **Code-Review** | Counts *approved* changesets by a second person | Solo project — no second approver exists. Only improves with a co-maintainer. |
| **Contributors** | Wants commits from ≥2 companies/orgs | Solo project. Improves organically. |
| **Branch-Protection** | Some settings disabled | The safe settings are enabled; *required approvals* are intentionally off because they would block the sole maintainer's own merges. |
| **Packaging** | Looks for a package-registry publish workflow | A desktop app has no registry target; releases ship as signed GitHub Release assets (SBOM + cosign + SLSA). Accepted at the heuristic's expense. |

The checks that reflect real engineering hygiene — Dangerous-Workflow,
Token-Permissions, SAST, Pinned-Dependencies, Signed-Releases, Security-Policy,
License, Fuzzing — are at or near maximum. See [`threat-model.md`](threat-model.md)
for the application's actual security posture.
