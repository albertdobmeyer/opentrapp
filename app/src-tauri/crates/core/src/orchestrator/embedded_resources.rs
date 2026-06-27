//! Perimeter policy resources embedded into `opentrapp-core` and extracted to
//! the runtime resource dir on every launch.
//!
//! These ~17 files (~150 KB) are the security policy the perimeter bind-mounts:
//! the seccomp profiles, the L7 proxy addon + allowlist, the egress resolv.conf,
//! the workload manifests, and the shared Sentinel lib. They are `include_bytes!`'d
//! here — exactly like [`super::perimeter`] embeds `perimeter.yml` via
//! `include_str!` — so they ride inside the signed binary (covered by its
//! signature) and the daemon needs no external bundle dir. This is what makes
//! `opentrapp-core` self-sufficient post de-Tauri (ADR-0022): no `handle.path()
//! .resource_dir()` from Tauri, no build.rs staging.
//!
//! Canonical sources live in the workload/infra/sentinel dirs; the in-crate
//! copies under `src/embedded/perimeter-resources/` are refreshed by
//! `make sync-core-embedded` and guarded against drift by
//! `tests/orchestrator-check.sh` (`cmp -s`, the same pattern as `perimeter.yml`).
//! `include_bytes!` does NOT carry Unix mode, so the executable flag is declared
//! explicitly in the table below and re-applied on extraction.
//!
//! The exact contract (every file present, byte-identical, exec bits intact,
//! covers every `kind: resource`/`seccomp` mount in `perimeter.yml`) is pinned by
//! the `pin_*` tests in [`super::podman`].

use std::path::Path;

/// `(dest path relative to the resource dir, file bytes, needs-exec)`.
/// Mirrors build.rs STAGED_RESOURCES + stage_manifests + stage_sentinel and the
/// `make sync-core-embedded` target — keep all three in lockstep.
const EMBEDDED: &[(&str, &[u8], bool)] = &[
    // ── containment: seccomp syscall filters ──────────────────────────────
    (
        "vault-seccomp.json",
        include_bytes!("../embedded/perimeter-resources/vault-seccomp.json"),
        false,
    ),
    (
        "vault-proxy-seccomp.json",
        include_bytes!("../embedded/perimeter-resources/vault-proxy-seccomp.json"),
        false,
    ),
    // ── L7 egress policy (the goproxy compiles its matcher in; only the
    //    allowlist data is provisioned as a resource — ADR-0026) ───────────
    (
        "allowlist.txt",
        include_bytes!("../embedded/perimeter-resources/allowlist.txt"),
        false,
    ),
    // ── L3 egress policy ──────────────────────────────────────────────────
    (
        "resolv.conf",
        include_bytes!("../embedded/perimeter-resources/resolv.conf"),
        false,
    ),
    // ── workload manifests (GUI/dashboard discovery) ──────────────────────
    (
        "manifests/agent/component.yml",
        include_bytes!("../embedded/perimeter-resources/manifests/agent/component.yml"),
        false,
    ),
    (
        "manifests/skills/component.yml",
        include_bytes!("../embedded/perimeter-resources/manifests/skills/component.yml"),
        false,
    ),
    (
        "manifests/social/component.yml",
        include_bytes!("../embedded/perimeter-resources/manifests/social/component.yml"),
        false,
    ),
    // ── shared Sentinel lib (mounted :ro at /opt/sentinel on the shields) ──
    (
        "sentinel/config.sh",
        include_bytes!("../embedded/perimeter-resources/sentinel/config.sh"),
        true,
    ),
    (
        "sentinel/egress-advisor.sh",
        include_bytes!("../embedded/perimeter-resources/sentinel/egress-advisor.sh"),
        true,
    ),
    (
        "sentinel/judge.sh",
        include_bytes!("../embedded/perimeter-resources/sentinel/judge.sh"),
        true,
    ),
    (
        "sentinel/embed.sh",
        include_bytes!("../embedded/perimeter-resources/sentinel/embed.sh"),
        true,
    ),
    (
        "sentinel/corpus/build.sh",
        include_bytes!("../embedded/perimeter-resources/sentinel/corpus/build.sh"),
        true,
    ),
    (
        "sentinel/corpus/known-bad.json",
        include_bytes!("../embedded/perimeter-resources/sentinel/corpus/known-bad.json"),
        false,
    ),
    (
        "sentinel/lib/sentinel_embed.py",
        include_bytes!("../embedded/perimeter-resources/sentinel/lib/sentinel_embed.py"),
        false,
    ),
    (
        "sentinel/README.md",
        include_bytes!("../embedded/perimeter-resources/sentinel/README.md"),
        false,
    ),
    (
        "sentinel/verdict-schema.json",
        include_bytes!("../embedded/perimeter-resources/sentinel/verdict-schema.json"),
        false,
    ),
];

/// Extract every embedded policy file into `dst` (the runtime resource dir),
/// overwriting any prior/tampered copy. Re-applies the executable bit to the
/// Sentinel scripts (lost by `include_bytes!`). Called on every perimeter
/// bring-up so the runtime copy is always the signed source of truth.
pub fn extract_embedded_resources(dst: &Path) -> std::io::Result<()> {
    for (rel, bytes, exec) in EMBEDDED {
        let path = dst.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        #[cfg(unix)]
        if *exec {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

/// The count of embedded files — used by the drift check to confirm the table
/// stays in lockstep with `make sync-core-embedded`.
pub const EMBEDDED_FILE_COUNT: usize = EMBEDDED.len();
