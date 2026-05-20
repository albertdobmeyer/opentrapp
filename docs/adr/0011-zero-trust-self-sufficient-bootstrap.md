# ADR-0011 — Zero-trust, self-sufficient first-launch bootstrap

**Status:** Accepted — implemented in v0.5.0
**Decision date:** 2026-05-20
**Supersedes (partially):** the bootstrap half of [ADR-0006](0006-four-container-topology.md) — the *topology* rationale stands; the *delivery + orchestration* mechanism it implied (host `podman compose` over a source tree) is replaced.
**Implemented by:**
- `app/src-tauri/src/orchestrator/perimeter.rs` — the signed perimeter spec (`resources/perimeter.yml`, compile-time embedded)
- `app/src-tauri/src/orchestrator/podman.rs` — native podman orchestrator + `BundleVerifier` + `fetch_perimeter_images`
- `app/src-tauri/src/bootstrap/mod.rs` — the 7-step pipeline (prepare-bundle → fetch → verify+load → up)
- `.github/workflows/ci.yml` — the `build-images` job (build, cosign-sign, push GHCR, export tarballs as release assets, emit signed `image-digests.json`)
**Verified by:** clean-box E2E (2026-05-20) — full five-container perimeter up from an AppImage in a directory with no source clone, zero on-host build, every image digest-verified, a tampered tarball refused.

---

## Context

Through v0.4.x the app brought the perimeter up by shelling out to `podman compose` against a checked-out source tree (`find_monorepo_root()` walked the filesystem for a `components/` directory). This worked on a developer's machine and **failed on every installed AppImage**: there is no source tree, no `compose.yml`, no Dockerfiles, and on a stock Ubuntu 24.04 box `podman compose` silently delegates to whatever external compose provider the host happens to have (here, `docker-compose-v2`), which the app neither ships nor controls.

The root problem, stated plainly: **the install path ran a developer build pipeline.** It depended on un-pinned host state — the host's source tree *and* the host's container-orchestration tooling. A security wrapper cannot delegate the construction of its own containment perimeter to software it cannot verify.

## Decision

First launch is **self-sufficient and zero-trust**. The app builds nothing on the user's machine; it loads pre-built, signed, digest-pinned images and orchestrates containers with code we own.

1. **Own the orchestration.** A native Rust orchestrator (`podman.rs`) reads a declarative perimeter spec and issues `podman run`/`stop`/`rm` directly. The only host dependency is `podman` itself, whose presence/version we check. No `compose`, no host-provided orchestration tool.

2. **The perimeter spec is signed by construction.** `perimeter.yml` is embedded into the binary via `include_str!`, so it is covered by the AppImage signature — the topology cannot change without a rebuild + re-sign. `compose.yml` is retained for dev-mode and audit; `tests/orchestrator-check.sh` keeps the two in agreement.

3. **Images are built, signed, and pinned in CI — never on the user's machine.** The `build-images` job builds each `vault-*` image, signs it with cosign keyless (Fulcio identity bound to our workflow), pushes it to GHCR by digest, and emits a signed `image-digests.json` overlay (pinned digests + signer identity + release coordinates).

4. **Delivery is decoupled from trust.** Image tarballs are attached as **release assets** (not bundled in the AppImage — bundling 485 MB made the AppImage 567 MB with a ~68 min build). Only the small overlay rides inside the signed AppImage as the trust anchor. At first launch the runtime downloads the tarballs from the release and **verifies each against the overlay digest before `podman load`** — a tampered or substituted image loads under a different digest and is refused. Upstream images (mitmproxy) are pulled by their pinned digest. **Runtime trust is digest-pinning against the signed overlay, not live cosign** — the user has no cosign, and keyless verification needs network; the cosign signatures on GHCR are the public/audit axis.

5. **Runtime state lives in `~/.opentrapp/`**, not a source tree (`runtime_data_dir`, replacing `monorepo_root`): `.env`, marker files, the verified policy-file resource dir, and the downloaded images. Security-policy files (seccomp profiles, the mitmproxy addon, allowlist, `resolv.conf`) ship inside the signed AppImage and are re-staged on every launch (self-healing against host-side tampering); they are never bind-mounted from a user-writable path.

6. **Component manifests are bundled, not discovered from a source tree** — so the UI renders dashboards on a clean machine before the perimeter is even up.

## Alternatives considered

- **Keep `podman compose` + bundle the source tree.** Rejected: still depends on an un-pinned host compose provider, and bundling Dockerfiles + build contexts means building on the user's machine (slow, unverifiable, the original failure).
- **Bundle `podman-compose` (the Python tool) in the AppImage.** Rejected: trades docker-compose-v2 roulette for an un-pinned host `python3` dependency.
- **Bundle image tarballs inside the AppImage (offline-first).** Implemented first, then reversed: measured 567 MB AppImage / ~68 min build (squashfs recompressing already-compressed layers). The release-asset model gives the same zero-trust guarantee with a ~90 MB AppImage and a minutes-long build; first launch needs network, which it already does for the agent's vendor API.

## Consequences

**Positive:** install works on any machine with `podman` and no source clone; nothing is built on the user's box; every image is cryptographically pinned to a signed anchor; ~90 MB AppImage; the perimeter definition is compile-time verified.

**Negative / residual:**
- First launch requires network to fetch images (subsequent launches are offline — images persist in podman storage).
- macOS/Windows have no `podman` by default; their runtime-install story is **deferred** (Linux/AppImage only so far).
- The app must spawn `podman` with a sanitized environment: an AppImage injects `LD_LIBRARY_PATH` pointing at its bundled glib, which otherwise poisons `conmon` (`undefined symbol: g_assertion_message_cmpint`). The orchestrator strips these vars when spawning podman.
