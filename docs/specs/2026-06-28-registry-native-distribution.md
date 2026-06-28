# Spec 2 — Registry-native distribution (cargo-dist + crates.io)

**Date:** 2026-06-28 · **Author:** session prep · **Status:** DRAFT for owner review (lane already built; this spec is a verification harness + owner runbook, not a build).
**Decision frame:** [ADR-0023](../adr/0023-distribution-and-packaging.md) (two-track: registry publish + OS-agnostic installers, no single-vendor lock), [ADR-0020](../adr/0020-product-identity-and-distribution.md) tenet 2.
**Roadmap home (status):** [`ROADMAP.md`](../../ROADMAP.md) §5. **Harmonized sequence:** [`2026-06-28-command-surfaces-harmonization.md`](2026-06-28-command-surfaces-harmonization.md).

> **The bar.** "Done" for a release is verified at the **consumption end** (§11): a user actually `cargo install`s / runs the installer, `cargo add opentrapp-core` resolves, the images pull and cosign-verify. A green CI build is necessary, never sufficient.

---

## 1. The headline: the lane is built. This spec verifies it and hands the owner the cut.

Recon confirmed the entire release machinery is **built, SHA-pinned, and ready**. There is **no build work** here. The two missing pieces are both **owner-gated** (a secret + a tag). So this spec delivers: (a) a **release-readiness verification harness** that proves the lane will succeed *before* the owner cuts, (b) a few **agent-preparable polish** items, and (c) the **owner runbook**.

This honors CLAUDE.md §11 *"gate the claim, not the workstream"*: v0.9.0 is shippable now; the unified CLI (Spec 1) and MCP (Spec 3) do not block it.

## 2. Verified current state (ground truth)

| Component | State | Evidence |
|---|---|---|
| cargo-dist: 5 targets (linux x86_64/aarch64, macOS x86_64/aarch64, windows x86_64), `shell`+`powershell` installers, SLSA keyless attestations, version pinned `0.32.0` | **READY** | `dist-workspace.toml:7,11,13,17` |
| `release.yml`: triggers on SemVer tag (+ PR dry-run), jobs plan→build-local→build-global→host→announce, all actions SHA-pinned, creates a **DRAFT** release | **READY** | `.github/workflows/release.yml:41–45,59,291` |
| `ci.yml::build-images`: builds 5 perimeter images, **cosign keyless-signs**, OCI-exports, writes signed `image-digests.json`, attaches to the draft (waits ≤30 min) | **READY** | `.github/workflows/ci.yml:195–318` |
| `publish-crate.yml`: publishes **`opentrapp-core`** to crates.io on `v*`; **gated on `CARGO_REGISTRY_TOKEN` presence** (skips cleanly if absent) | **READY (token-gated)** | `.github/workflows/publish-crate.yml:15–18,28–53` |
| `opentrapp-core` publishability: has `description`/`license`/`repository`; **no path-only deps**; **not** `publish = false`; embeds its data files (`src/embedded/`) | **READY** | `crates/core/Cargo.toml:9,11,20,21,28–36` |
| `opentrapp-daemon` + `viewer-server`: `publish = false` (workspace-internal) but `[package.metadata.dist] dist = true` → shipped as binaries, not crates | **CORRECT** | `crates/daemon/Cargo.toml:12,22–23`; `viewer-server/Cargo.toml:23–24` |
| Versions: workspace `0.9.0` lockstep (core/daemon/viewer-server/frontend); last git tag `v0.8.0` (pre-cutover Tauri app) | **GAP = next cut is v0.9.0** | `crates/*/Cargo.toml`; `git tag` |
| Image signing keys | **None on box (by design)** — GitHub OIDC / Sigstore keyless for both artifacts (SLSA) and images (cosign) | `dist-workspace.toml:17`; `ci.yml:220,243` |

**Missing for a real cut — both owner-gated:**
1. `CARGO_REGISTRY_TOKEN` repository secret (without it, crates.io publish skips and OpenSSF Scorecard *Packaging* stays unlit).
2. The `v0.9.0` git tag (fires all three workflows).

## 3. The refactor / rebuild / new-code verdict

| Area | Verdict | Why |
|---|---|---|
| cargo-dist config, release.yml, build-images, publish-crate.yml | **KEEP (zero change)** | Built, hardened, SHA-pinned. |
| Release-readiness verification (dry-run publish, dist plan, metadata completeness) | **NEW (small, agent-preparable)** | Proves the lane *will* succeed before the irreversible tag. None exists today. |
| crates.io metadata polish (keywords/categories/readme/rust-version) | **REFACTOR (tiny)** | Improves discoverability + may be required for a clean publish; see §5. |
| v0.9.0 CHANGELOG / release notes | **NEW (agent-draftable, owner-approved)** | The de-Tauri + goproxy + alpine story, scoped to what's verified. |
| `RELEASING.md` accuracy pass | **REFACTOR (tiny)** | Ensure the documented steps match the built lane + the Spec-1 binary rename. |
| Homebrew tap / MSI | **DEFER (YAGNI)** | ADR-0023 marks Homebrew "later optional"; MSI not requested. Installers are shell+powershell. |

## 4. The release-readiness verification harness (the agent-preparable core)

A non-gating, on-demand check (a `make release-dryrun` target and/or a `workflow_dispatch` CI job) that proves the cut will succeed. It runs the **consumption-end** checks short of the irreversible publish:

1. **`cargo publish -p opentrapp-core --dry-run --locked`** — the single most important goalpost. It packages + compiles exactly what crates.io will receive. Exit 0 ⇒ the publish will work. **This is runnable today** and tells us the true red/green (e.g. a missing `readme`, an excluded embedded file, or a dev-dep leak would surface here).
2. **`dist plan`** — asserts the announced version == workspace `0.9.0` and all 5 targets are present (the installers will build).
3. **Metadata completeness** — assert `opentrapp-core` Cargo.toml carries the crates.io-recommended fields (see §5).
4. **No-unpublishable-deps invariant** — assert core has no `{ path = … }` deps and is not `publish = false` (pins the property so a future edit can't silently break the publish).

## 5. crates.io metadata polish (small, may be release-blocking)

`opentrapp-core` has the *required* trio (`description`/`license`/`repository`). For a clean, discoverable publish add the recommended fields if absent: `readme` (a crate-level README so crates.io renders a page), `keywords`, `categories`, `rust-version` (MSRV). The dry-run (§4.1) reveals whether any of these is hard-blocking vs merely advisory. Agent-preparable from the existing crate docs; owner approves the keyword/category choices.

## 6. Red-first goalposts (concrete; define "release-ready")

| # | Goalpost | Asserts | Likely state now |
|---|---|---|---|
| R1 | `cargo publish -p opentrapp-core --dry-run --locked` | exit 0 | **unknown — run it first** (heavy compile; box-capable or CI). The truth-source. |
| R2 | `core_manifest_has_crates_io_recommended_fields` (Rust test or orchestrator-check section) | core Cargo.toml has `readme`+`keywords`+`categories` | likely **red** (only the required trio confirmed) |
| R3 | `core_has_no_path_deps_and_is_publishable` | no `{path=…}` deps; `publish` ≠ false | green now — pin so it stays green |
| R4 | `dist plan` announces `0.9.0` with all 5 targets | parse `dist plan` JSON | green now — pin |
| R5 | `release_notes_exist_for_the_target_version` | a `CHANGELOG`/notes entry for `0.9.0` exists | **red** (to author) |

R2/R5 are genuinely red-first deliverables; R3/R4 are regression pins; R1 is the load-bearing consumption-end proof to run first.

## 7. Owner runbook (the gated cut — outward-facing, do not automate)

> **Release hard-gate ([ROADMAP.md](../../ROADMAP.md):126, owner 2026-06-22):** no version ships until all code-scan alerts are closed. Status: #46 cleared by the de-Tauri cutover, #80–82 done, goproxy advisories cleared (#198); **only #43 + #1 (co-maintainer) remain open — honestly open, never dismissed.** Confirm the owner's go/no-go against this gate before tagging.

1. Add `CARGO_REGISTRY_TOKEN` to repo secrets (from crates.io).
2. Land the release-prep commit: version is already `0.9.0`; add the CHANGELOG/notes (R5). Run the harness (§4) — all green.
3. `git tag v0.9.0 && git push origin v0.9.0` → fires `release.yml` (draft), `ci.yml::build-images` (signed images onto the draft), `publish-crate.yml` (crates.io). **Never push a `v*` tag without the owner's explicit go/no-go** (outward-facing, irreversible).
4. **Verify the draft at the consumption end before publishing:** download + run the shell + PowerShell installers; `cargo add opentrapp-core` / `cargo install` resolves; `cosign verify` + `gh attestation verify` pass on the images. Then publish the draft.
5. **Post-publish:** exercise the **BundleVerifier digest-staging T0** on a clean box (the post-release boundary self-test — ROADMAP §1:51) so the *shipped* path, not just from-source, is proven.

## 8. Verification at the consumption end (the bar, §11)

- **Pre-tag:** the harness (§4) green, esp. R1 (`--dry-run` exit 0).
- **Post-tag, pre-publish:** installers actually install + run; the crate actually resolves; images actually pull + verify. (§7.4.)
- **Post-publish:** BundleVerifier T0 on a clean box (§7.5). Until then the shipped-path containment claim is **unverified, not done** (§11).

## 9. Out of scope (YAGNI)

Homebrew tap, MSI/WiX, SignPath code-signing reapply (ROADMAP §5:101, after visibility), any second registry. All explicitly deferred by ADR-0023 / the roadmap.

## 10. Parallelizable (zero-decision) chunks

- R2/R3/R4 checks + the `make release-dryrun` target — pure coding from §4–§6.
- The metadata field additions (§5) once the owner picks keywords/categories.

Stays with owner: the secret, the tag, the go/no-go, the keyword/category choices, the draft verification.
