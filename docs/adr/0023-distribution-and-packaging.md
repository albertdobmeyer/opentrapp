# ADR-0023 — Distribution & packaging: OS-agnostic, vendor-neutral, Scorecard-legible

**Status:** Proposed — the distribution strategy is decided here; implementation is staged (the
recognized registry publish can land **early**, decoupled from de-Tauri; the prebuilt-installer track
lands with the de-Tauri cutover — see Sequencing). Tier 3 of the architecture mission
([ADR-0020](0020-product-identity-and-distribution.md)).
**Constraint (maintainer):** *as OS-agnostic as possible; never vendor- or OS-lock the project.*
**Cross-references:** [ADR-0020](0020-product-identity-and-distribution.md) (CLI/daemon + images are
the artifacts) · [ADR-0022](0022-daemon-control-surface.md) (the de-Tauri cutover changes the shipped
binary from a Tauri app to the daemon+CLI) · [ADR-0014](0014-monorepo-modular-distribution.md)
(modular distribution) · [`de-tauri-viewer-research.md`](../de-tauri-viewer-research.md) · CLAUDE.md

---

## Context

OpenTrApp's Scorecard **Packaging** check reads `?` ("no publishing workflow detected") because today
the app ships as native installers via GitHub Releases (`tauri-action`) and the perimeter images are
pushed to GHCR with a hand-rolled `podman push` — and **neither is a pattern Scorecard recognizes.**
ADR-0020 reframed the product as a **CLI/daemon + signed container images**; this ADR decides how those
are distributed under a hard constraint: maximum OS-agnosticism, no vendor or OS lock-in.

**Verified — what Scorecard actually recognizes** (`fileparser.IsPackagingWorkflow`, ossf/scorecard,
checked 2026-06-14). A workflow counts as "packaging" only if a step matches one of a fixed set; the
relevant ones:

- ✅ **Rust:** a `run:` step matching **`cargo.*publish`** (i.e. `cargo publish` to crates.io). *Note:
  the popular `katyo/publish-crates` action is **NOT** matched — it must be a literal `cargo publish`
  run step.*
- ✅ **Container:** a `run:` matching **`docker.*push`** **or** the **`docker/build-push-action`** action.
  *Our `podman push` does **not** match — this is the gap.*
- ✅ Go (`goreleaser/goreleaser-action`), npm, PyPI, Maven/Gradle, Ruby, NuGet, Scala, Ko, Elixir.
- ❌ **NOT recognized:** Homebrew (no pattern), generic GitHub Release (`softprops/action-gh-release`),
  `redhat-actions/push-to-registry`, `katyo/publish-crates`, `cargo-dist`'s release output.

This makes the decision concrete: the channel that *both* fits the no-lock constraint *and* flips the
check is **crates.io via a literal `cargo publish`**.

## Decision

A **two-track** distribution — a recognized registry publish (for legibility + developers/agents) and
OS-agnostic prebuilt installers (for the actual end-user UX) — with **no single-OS package manager or
proprietary vendor as a gate.**

### 1. Recognized registry publish (canonical, vendor-neutral, flips Scorecard) — `cargo publish` → crates.io
Publish **`opentrapp-core`** — the reusable, **Tauri-free** orchestration library (manifest/perimeter
control) — to **crates.io** via a literal `cargo publish` CI step.
- **OS-agnostic / no lock:** crates.io is the open, canonical Rust registry (Rust Foundation), not a
  proprietary vendor; source builds on any OS.
- **Flips Packaging** (the verified `cargo.*publish` pattern).
- **Genuinely honest, not gaming:** `opentrapp-core` is a real, reusable library — publishing it lets
  others build alternative viewers/tools on the same generic backend (the ADR-0020 "three projections"
  vision). *(The CLI/daemon **binary** is not a clean `cargo install` target — its `build.rs` bundles
  perimeter resources from the monorepo, which a crates.io source package lacks — so the binary ships
  as a prebuilt, track 2. The library is the source-registry artifact.)*

### 2. OS-agnostic end-user install (prebuilt, no compile, no lock) — `cargo-dist` (dist) → GitHub Releases
Use **`cargo-dist`** to build the CLI/daemon for **Linux (x86_64/aarch64), macOS (x86_64/aarch64),
Windows (x86_64)** from one config and publish to **GitHub Releases** with:
- a **universal shell installer** (`curl … | sh`, Linux/macOS) **and** a **PowerShell installer**
  (`irm … | iex`, Windows) — the *same* install UX on every OS (the rustup / uv / Bun model);
- checksums + the existing **cosign + SLSA provenance + minisign** signing;
- **inspect-before-run** documented (`curl -o install.sh; less install.sh; sh install.sh`) — a security
  tool must not normalize blind `curl|sh`; the non-piped alternatives (the crates.io library, the GHCR
  images) are always offered.
- **OS-agnostic / no lock:** self-hosted on GitHub Releases; no third-party package manager required;
  prebuilt so no toolchain needed.

### 3. Container images → GHCR via a recognized publish
Keep images on **GHCR** (already), but change the publish step to a Scorecard-recognized pattern — a
`docker push` `run:` step or `docker/build-push-action` (cosign signing retained). Secondary to track 1
for the score, but it makes the images count *and* GHCR/OCI is the vendor-neutral, OS-agnostic image
standard (any OCI runtime — podman, docker, containerd — can pull them).

**Landed (`ci.yml` `build-images`).** The perimeter-image push now uses `docker buildx build --push`
(matches `docker.*push`); since `build-images` lives in `ci.yml`, which has successful runs, Scorecard
recognizes the file as a publishing workflow (`checks/raw/github/packaging.go` requires a static matcher
hit **and** ≥1 successful run of that file). The conversion preserves the zero-trust digest invariant by
keeping every digest-sensitive step in **podman** — buildx only builds + pushes a single plain manifest
(`--provenance=false --sbom=false`); the offline bundle is `podman pull`-by-digest (which verifies the
content) → `podman save`, and a **fail-closed self-verify** (`podman load` → `image exists @digest`)
mirrors the runtime `BundleVerifier` exactly, so a subtly-wrong publish fails the release instead of
shipping a perimeter that silently refuses to start (§11). Recognition is verified statically; the
tag-only execution is confirmed on the next release tag (e.g. an `-rc`), not on the dev box.

### 4. Explicitly rejected as a *primary* channel (the no-lock test)
- **Homebrew / apt / AUR / winget / Chocolatey / Snap / Flatpak** — each is single-OS or single-vendor
  (Homebrew is macOS+Linux only, no Windows, *and* not Scorecard-recognized). **Allowed only as
  optional community conveniences, never the headline or a gate.** `cargo-dist` can emit a Homebrew
  formula for those who want it, but the documented default is the universal installer + crates.io.
- **`softprops/action-gh-release` alone** — not recognized (this is the current `?`).
- **A proprietary app store** — vendor lock; rejected outright.

## Consequences

**Positive**
- Maximum OS reach with one identity: every OS gets the same `install`-script UX (track 2) + the same
  `cargo`/OCI source (tracks 1, 3); no user is told "install Homebrew first" or "Linux only."
- Vendor-neutral end to end: crates.io + GitHub Releases + GHCR are the open, canonical, non-proprietary
  homes — the project can be forked and re-hosted without re-platforming.
- **Scorecard Packaging flips** via the recognized `cargo publish` (+ the GHCR fix) — legitimately, by
  publishing a real reusable library, not by gaming a heuristic.
- Publishing `opentrapp-core` advances the "generic backend / many viewers" vision (others can build on it).

**Negative / cost (honest)**
- `cargo install opentrapp-core` compiles from source (needs a Rust toolchain) — so it is the
  developer/recognition channel, **not** the end-user path; end users use track 2 (prebuilt). Both are
  OS-agnostic; the split must be documented so no one is told to `cargo install` the daemon.
- `curl|sh` on a security tool is a values tension — mitigated by signatures/checksums, inspect-before-run,
  and always offering non-piped alternatives (same posture as rustup).
- Publishing to crates.io requires the workspace path-deps to be versioned/published (today
  `publish = false` for the cargo-deny path-dep reason); `opentrapp-core` must become a publishable,
  versioned crate. Real but bounded implementation work.

## Alternatives considered

- **Homebrew-first.** Rejected: no Windows (OS-lock), and not Scorecard-recognized.
- **`cargo-dist` only (no crates.io).** Rejected for *recognition*: cargo-dist's GitHub-Releases output
  is not a recognized packaging pattern, so Packaging would stay `?`. (cargo-dist is kept as track 2.)
- **`cargo install` the daemon as the end-user path.** Rejected: the resource-bundling `build.rs` makes
  the binary not cleanly source-installable, and compiling-from-source is poor end-user UX.
- **An OS app store / vendor channel.** Rejected: vendor lock.

## Sequencing (what can land when)

- **Now / decoupled from de-Tauri (flips Packaging early):** make `opentrapp-core` a publishable crate +
  add the `cargo publish` CI step; switch the GHCR image push to a recognized `docker push` /
  `docker/build-push-action`. These do **not** depend on the de-Tauri migration and can move the
  Scorecard Packaging check off `?` ahead of the rest of the mission.
- **With the de-Tauri cutover ([ADR-0022](0022-daemon-control-surface.md)):** wire `cargo-dist` for the
  CLI/daemon binary (most natural once the shipped artifact is the daemon+CLI, not a Tauri bundle) and
  retire `tauri-action`'s native installers.

## What this ADR does NOT decide

- The exact `cargo-dist` config, the crates.io crate split/versioning mechanics, and the CI wiring →
  implementation, gated on this ADR + (for track 2) the de-Tauri cutover.
- macOS/Windows code-signing for the prebuilt binaries (SignPath/Apple) → the existing code-signing track.
