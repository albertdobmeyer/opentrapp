# Known advisories & Scorecard posture

This document explains the security advisories OpenTrApp **knowingly accepts**
(with rationale) and how to read the project's [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/albertdobmeyer/opentrapp).
It exists so that a "low" Scorecard *Vulnerabilities* number is not mistaken for
negligence — most of it is unfixable upstream noise, and we say so plainly.

The machine-readable source of truth is
[`app/src-tauri/deny.toml`](../app/src-tauri/deny.toml) (`[advisories].ignore`),
enforced by `cargo deny check` in [`supply-chain.yml`](../.github/workflows/supply-chain.yml).

---

## Trust-tier triage: what gates a release vs what is a contained residual

OpenTrApp is a containment system, so its code lives in three trust tiers
([`GLOSSARY.md`](../GLOSSARY.md) section 4, [`trifecta.md`](trifecta.md) section 2),
and a security-scan alert is triaged by **which tier it lands in**:

- **Tier 1, trusted (host):** the user, the host CLI coordinator (e.g. Claude Code,
  the *external* operator who orchestrates the app with the user's authority), the GUI.
  Runs on the host with the user's privileges.
- **Tier 2, infrastructure (the perimeter):** the orchestrator plus the `vault-*`
  containers' policy code (proxy key-injection and allowlist, the L3 egress filter,
  the skill scanner). This is the boundary-*enforcing* code.
- **Tier 3, contained (untrusted by design):** the agent runtime inside `vault-agent`
  (the open-source CLI agent, OpenClaw, plus its dependency tree), loaded skills, and
  fetched content. The architecture **assumes this is hostile** and is engineered for
  its compromise ([`ADR-0006`](adr/0006-four-container-topology.md): "it runs
  untrusted-by-design code"; whitepaper section 3.1: "Tier 3 is *expected* to fail").

**The gating rule.**

- **Tiers 1 and 2 (trusted / boundary-enforcing): a vulnerability there is
  release-gating.** This code runs with the user's privileges (Tier 1) or *is* the cage
  (Tier 2), so a flaw is a real breach path. These alerts block a release. Example: the
  GTK3/Tauri advisories below are Tier-1 GUI code, genuinely gating, cleared only by the
  de-Tauri cutover.
- **Tier 3 (the contained agent and its dependencies): out of scope by policy, tracked
  as a *contained residual*, not release-gating.** [`SECURITY.md`](../SECURITY.md)
  already states it: "Vulnerabilities in upstream dependencies (... npm packages ...)
  ... are out of scope." The release gate for Tier 3 is **containment verified**: the
  T0 boundary self-test passes (network isolation, the credential air-gap B3, the L3
  egress filter, read-only skills), **not** "the inside agent's code is vuln-free,"
  which is unreachable and is not the security claim. The whole USP-1 is that **even a
  fully compromised inside agent cannot reach the credentials, the host, or the network
  except through the proxy allowlist** (verified by `verify.sh` check 7 and boundary
  self-test B3).

Hash-pinning the inside agent's dependencies is therefore the wrong move: it adds no
integrity beyond the downstream image-digest pin, and it would convert Tier-3
out-of-scope upstream vulns into release-blocking alerts, contradicting both the policy
and the architecture.

### Tier-3 contained residual: the OpenClaw runtime (`vault-agent`)

`workloads/agent/recipes/openclaw/install.sh` installs `openclaw@2026.2.26` with
`--ignore-scripts`. OpenClaw's npm dependency tree (about 733 packages) carries upstream
advisories; a 2026-06-22 resolution check counted 15 (13 high, 2 critical): the `axios`
SSRF / prototype-pollution cluster, `@whiskeysockets/baileys` message spoofing
(critical), `@mariozechner/pi-coding-agent` local privilege escalation,
`@hono/node-server`, `tar`, `ws`, and the Discord / Lark adapters. **All are upstream in
OpenClaw, none fixable at our layer**, and all are already present in the shipped agent
image (the perimeter runs OpenClaw today).

These are Tier-3 contained residuals, not defects in OpenTrApp's posture; they are the
reason the cage exists. Each is neutralized by the perimeter:

- `axios` SSRF: the agent has no network except the `vault-proxy` allowlist, so the
  request cannot reach an attacker-controlled host. Contained.
- `baileys` WhatsApp spoofing (critical): WhatsApp is not enabled in the hardened
  config. Dead code.
- `pi-coding-agent` LPE "on shared Linux hosts": the agent runs non-root in a
  single-user container with no other users. Not applicable.

**Why no lockfile.** Scorecard's Pinned-Dependencies check wants `npm ci` (a committed
lockfile) rather than `npm install -g pkg@version`. We deliberately keep the
version-pinned install because (a) the integrity that matters for the agent image is the
**downstream image-digest pin** (the signed bundle pins the entire built image by
digest, strictly stronger than npm hash-pins, [`ADR-0023`](adr/0023-distribution-and-packaging.md)),
reinforced by the version pin and `--ignore-scripts` (no dependency build-scripts run at
install); (b) a tampered dependency in Tier 3 is *contained* (it would run in the cage
with no keys and no egress except the proxy), unlike a Tier-1 dependency where a tamper
is host compromise; and (c) committing the lockfile would surface those 15 out-of-scope
upstream vulns into GitHub's dependency graph as standing, unfixable, release-blocking
alerts. Build-time pinning of the *trusted* tiers is already in place
(`/app/package-lock.json`, the `cargo` lockfiles, Dependabot scoped to `/app`). The
Scorecard Pinned-Dependencies finding on the OpenClaw line is accepted as a Tier-3
residual under the rule above.

---

## The 23 in the Scorecard *Vulnerabilities* count (2026-06-13)

> **Update (2026-06-27) — the 19 Rust advisories in this section are now RESOLVED by removal.** The
> de-Tauri cutover ([ADR-0022](adr/0022-daemon-control-surface.md); #184, 2026-06-24) **deleted the GUI
> crate and the entire Tauri / wry / GTK3 tree.** Verified at the consumption end: `app/src-tauri/Cargo.lock`
> has **0** `tauri`/`wry`/`webkit`/`gtk` entries and the `deny.toml` `[advisories].ignore` list is now
> **empty** (`cargo deny check advisories` is clean — nothing present-but-ignored). The external Scorecard
> *Vulnerabilities* count may lag until its next OSV re-scan, but at the lockfile level these are
> **eliminated, not merely accepted.** The detail below is the pre-cutover posture, kept as an audit
> trail. The only release-gating advisory surface today is the goproxy Go toolchain (RESOLVED 2026-06-27,
> at the end of this doc).

Scorecard's external OSV scan reported **23** (pre-cutover snapshot). They split cleanly:

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

## Accepted Rust advisories — now RESOLVED by the de-Tauri cutover (historical record)

> All 19 below were *accepted warnings* only while the Tauri GUI crate still pulled them. The cutover
> (#184, 2026-06-24) **removed the entire Tauri/wry/GTK3 tree**, so none appear in
> `app/src-tauri/Cargo.lock` any more and the `deny.toml` `[advisories].ignore` list is now **empty**
> (`cargo deny check advisories` is clean — nothing present-but-ignored). The table is the pre-cutover
> acceptance record.

The machine-readable acceptance for the `unmaintained` set **previously** lived in `deny.toml`
(`[advisories].ignore`) and is now empty. The two `unsound`
entries (glib, rand) were **not** in that list — cargo-deny's advisory-DB view did
not match our transitive version constraints, so an `ignore` there would have emitted
spurious "advisory-not-detected" warnings on every CI run. They were detected
by `cargo audit` / OSV and accounted for here at the time (pre-cutover).

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

## goproxy (`vault-proxy`, Tier-2) — Go toolchain advisories: RESOLVED 2026-06-27

`vault-proxy` is the L7 chokepoint and is **Tier-2 infrastructure (release-gating)** —
a flaw here is a real breach path, so it must stay advisory-clean. `govulncheck ./...`
on `infra/proxy/goproxy/` found **24 reachable** standard-library advisories (the repo
had earlier *guessed* "9"); every one was a patched-toolchain fix, because the module
built on **go 1.23.0**. Fixed at the root rather than per-CVE:

| Change | From → To | Clears |
|--------|-----------|--------|
| `go.mod` `go` directive | `1.23.0` → **`1.25.11`** | all 24 reachable stdlib advisories (1.25.11 is the max "Fixed in", e.g. GO-2026-5039/5037 net/textproto + crypto/x509) |
| `golang.org/x/net` | `v0.43.0` → **`v0.56.0`** | the one reachable *module* vuln, **GO-2026-4918** (`Fixed in v0.53.0`); `x/text` → v0.38.0 alongside |
| Containerfile builder | `golang:1.23-alpine` (floating) → **`golang:1.25.11-alpine@sha256:523c3eff…`** (digest) | reproducibility + the unpinned-builder gap (the runtime base was already digest-pinned) |

The 10 imported-package + 23 required-module advisories govulncheck also listed are
**not called** (no reachable path) and are swept up by the `x/*` bumps regardless.
**Verified at the consumption end:** `govulncheck` clean, `go test ./...` green, and the
rebuilt 15.6 MB image re-passed the live boundary self-test through the product daemon
(`pass=7` cold **and** resumed, B5 CA fingerprint unchanged). Re-evaluated on every Go
dependency/toolchain bump; the CI `goproxy vault-proxy (go test)` job guards regressions.

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
| **Pinned-Dependencies** | 9 | 3 unpinned commands in `workloads/skills/.devcontainer/setup.sh` (2 npm global installs, 1 pip) | 🔶 **Stays at 9 — npm portion not honestly fixable.** The pip line is now hash-pinned (`pip install --require-hashes -r requirements.txt`, cp312 wheel hashes verified via `pip download`) ✅. The **two `npm install -g` lines cannot reach Scorecard's bar**: per `isNpmUnpinnedDownload` in `ossf/scorecard`, npm is "pinned" **only** for `npm ci` (lockfile-verified) or a git URL anchored to a commit hash — a semantic-version pin (`npm@11.17.0`) is *not examined* and counts as unpinned. `npm ci` is not possible here (the devcontainer's `package.json` has no deps / lockfile and `molthub` is the workbench's own CLI, not a registry package), and a git+hash URL would require a real `molthub` repo. Neither is achievable without fabrication, so we accept 9. The version pins are kept for reproducibility, not score. The newer OpenClaw install line (`workloads/agent/recipes/openclaw/install.sh`) is a **Tier-3 contained residual** (see *Trust-tier triage* above): accepted for the same reason and explicitly not release-gating, because the inside agent is untrusted by design and is contained, not verified-clean. |

> **Correction (2026-06-14):** PR #86 originally claimed Pinned-Dependencies
> would reach 10. That was wrong — Scorecard does not credit npm version-pins
> (verified against its source). The score stays at 9; only the pip line and
> Signed-Releases improved. Recorded here rather than silently left to surface
> on the next rescan.

The remaining non-maximal checks (Vulnerabilities, Code-Review, Contributors,
Branch-Protection, Pinned-Dependencies' npm lines, CII Gold) are the structural /
upstream / people caps described above.
