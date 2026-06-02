use std::path::Path;

/// Policy/config files that must be bundled into the AppImage so the perimeter
/// can bind-mount verified copies at runtime (never from a writable host path).
/// Source of truth is the workload/infra directory; we stage a copy into
/// `resources/perimeter/` which `tauri.conf.json` bundle.resources packages.
/// (Image tarballs + image-digests.json are dropped into
/// `resources/perimeter/images/` by CI.)
const STAGED_RESOURCES: &[&str] = &[
    "../../workloads/agent/config/vault-seccomp.json",
    "../../workloads/agent/config/vault-proxy-seccomp.json",
    "../../infra/proxy/vault-proxy.py",
    "../../infra/proxy/allowlist.txt",
    "../../infra/egress/resolv.conf",
];

/// Which workload manifests a given install profile bundles. The GUI discovers
/// dashboards only for the manifests present, so a `containment` build shows
/// only the containment dashboard (Pillar B, modular distribution). Mirrors
/// `distribution.yml` at the repo root — keep the two in sync.
/// `agent` is the containment shield's manifest; `proxy`/`egress` are infra
/// with no component.yml.
fn profile_manifests(profile: &str) -> &'static [&'static str] {
    match profile {
        "containment" => &["agent"],
        "containment+skills" => &["agent", "skills"],
        "containment+social" => &["agent", "social"],
        // "all" and any unknown value default to the full set (no regression).
        _ => &["agent", "skills", "social"],
    }
}

fn stage_manifests() {
    // Profile is chosen at build/install time via OPENTRAPP_PROFILE; default
    // `all` preserves today's behaviour.
    let profile = std::env::var("OPENTRAPP_PROFILE").unwrap_or_else(|_| "all".into());
    println!("cargo:rerun-if-env-changed=OPENTRAPP_PROFILE");
    let base = Path::new("resources/perimeter/manifests");
    for workload in profile_manifests(&profile) {
        let src = format!("../../workloads/{workload}/component.yml");
        println!("cargo:rerun-if-changed={src}");
        let dest_dir = base.join(workload);
        if std::fs::create_dir_all(&dest_dir).is_err() {
            continue;
        }
        let _ = std::fs::copy(Path::new(&src), dest_dir.join("component.yml"));
    }
}

fn stage_perimeter_resources() {
    let dest_dir = Path::new("resources/perimeter");
    if std::fs::create_dir_all(dest_dir).is_err() {
        return;
    }
    for src in STAGED_RESOURCES {
        println!("cargo:rerun-if-changed={src}");
        let src_path = Path::new(src);
        if let Some(name) = src_path.file_name() {
            // Best-effort: a fresh checkout always has the workload files; if a
            // file is missing in a dev tree, we skip rather than fail the build
            // — the bundle will simply lack it and the orchestrator will
            // surface a clear runtime error.
            let _ = std::fs::copy(src_path, dest_dir.join(name));
        }
    }
}

/// Stage the shared Sentinel lib (`sentinel/`) into the perimeter resource dir
/// so both consumers find it in a packaged build: the host bridge
/// (`commands/sentinel.rs` → `resource_dir()/perimeter/sentinel`) and the
/// in-container shields (perimeter.yml `kind: resource` mount → `/opt/sentinel`).
/// Sentinel is a policy/lib artifact, so it rides the same verified-staged-`:ro`
/// path as vault-proxy.py / allowlist.txt (ADR-0009/0011, spec 08 §5). It is
/// shared across every install profile, so it is always staged regardless of
/// `OPENTRAPP_PROFILE`. Test files (`*.test.sh`) are skipped — runtime only.
fn stage_sentinel() {
    println!("cargo:rerun-if-changed=../../sentinel");
    let src_root = Path::new("../../sentinel");
    let dest_root = Path::new("resources/perimeter/sentinel");
    if !src_root.is_dir() {
        // A non-source build (e.g. CI staging artefacts only) may not have the
        // tree here; skip rather than fail — a missing lib surfaces as a clear
        // runtime error in the resolver.
        return;
    }
    let _ = copy_dir_recursive(src_root, dest_root);
}

/// Recursively copy `src` → `dest`, skipping `*.test.sh`. `std::fs::copy`
/// preserves Unix mode bits, so `judge.sh`/`embed.sh` stay executable.
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            copy_dir_recursive(&path, &dest.join(&name))?;
        } else if path.is_file() {
            if name.to_string_lossy().ends_with(".test.sh") {
                continue;
            }
            std::fs::copy(&path, dest.join(&name))?;
        }
    }
    Ok(())
}

/// Derive the set of container names for a given profile by reading
/// `distribution.yml` (the single source of truth). Returns all containers
/// from all shields the profile includes. Falls back to the empty set on any
/// parse error so that a missing/malformed distribution.yml never breaks the
/// default build — the unfiltered overlay is preserved in that case.
fn profile_containers_from_dist(profile: &str) -> Option<Vec<String>> {
    // Locate distribution.yml relative to the Cargo.toml (two levels up from
    // app/src-tauri/ → repo root).
    let dist_path = Path::new("../../distribution.yml");
    let text = std::fs::read_to_string(dist_path).ok()?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&text).ok()?;

    let shields = doc.get("shields")?;
    let profiles = doc.get("profiles")?;
    let default_profile = doc
        .get("default_profile")
        .and_then(|v| v.as_str())
        .unwrap_or("all");

    // Resolve the profile name: unknown → default_profile.
    let resolved = if profiles.get(profile).is_some() {
        profile
    } else {
        default_profile
    };

    let members = profiles.get(resolved)?.as_sequence()?;
    let mut containers: Vec<String> = Vec::new();
    for shield_name in members {
        let name = shield_name.as_str()?;
        let shield = shields.get(name)?;
        let ctrs = shield.get("containers")?.as_sequence()?;
        for c in ctrs {
            if let Some(s) = c.as_str() {
                containers.push(s.to_string());
            }
        }
    }
    Some(containers)
}

/// Stage a profile-filtered `image-digests.json` into
/// `resources/perimeter/images/`. The source overlay is written there by CI
/// and covers all five containers. This function rewrites it to contain only
/// the entries whose container is part of the active profile — so an AppImage
/// built with `OPENTRAPP_PROFILE=containment` only pulls 3 images at first
/// launch instead of all 5 (spec 05 §4f, modular distribution Pillar B).
///
/// Rules:
/// - Entries with `source: "external"` are always kept — they are upstream
///   pinned images (mitmproxy for vault-proxy) and are part of every profile
///   that includes the containment core.
/// - Entries with `source: "built"` are kept when the last path segment of
///   their GHCR repo key matches a container name in the profile's set.
/// - Default profile = `all` (unset `OPENTRAPP_PROFILE`): all entries kept
///   — no behaviour change for existing builds.
/// - No-op when the source overlay doesn't exist (dev builds without CI
///   artefacts; `DevVerifier` path in the runtime handles that case).
fn stage_images() {
    let profile = std::env::var("OPENTRAPP_PROFILE").unwrap_or_else(|_| "all".into());
    println!("cargo:rerun-if-env-changed=OPENTRAPP_PROFILE");
    println!("cargo:rerun-if-changed=../../distribution.yml");

    let images_dir = Path::new("resources/perimeter/images");
    let overlay_path = images_dir.join("image-digests.json");

    // No overlay yet (dev checkout without CI artefacts) — nothing to filter.
    if !overlay_path.exists() {
        return;
    }

    let json_text = match std::fs::read_to_string(&overlay_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cargo:warning=stage_images: could not read {}: {e}", overlay_path.display());
            return;
        }
    };

    // Parse as a generic JSON object so we preserve unknown top-level fields
    // (version, tag, repo, signer_identity_regexp, oidc_issuer) verbatim.
    let mut overlay: serde_json::Map<String, serde_json::Value> =
        match serde_json::from_str(&json_text) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("cargo:warning=stage_images: overlay parse error: {e}");
                return;
            }
        };

    // For profile=all (or unresolvable), keep everything as-is.
    if profile == "all" {
        return;
    }

    let allowed: std::collections::HashSet<String> =
        match profile_containers_from_dist(&profile) {
            Some(v) => v.into_iter().collect(),
            None => {
                eprintln!(
                    "cargo:warning=stage_images: could not resolve profile '{profile}' \
                     from distribution.yml — keeping full overlay"
                );
                return;
            }
        };

    let images = match overlay.get_mut("images").and_then(|v| v.as_object_mut()) {
        Some(m) => m,
        None => return,
    };

    // Filter in-place: keep external entries (always needed for vault-proxy /
    // containment core) + built entries whose service name is allowed.
    let keys_to_remove: Vec<String> = images
        .iter()
        .filter_map(|(key, entry)| {
            let source = entry
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("built");
            if source == "external" {
                return None; // always keep
            }
            // Built image: key is a GHCR repo like
            // `ghcr.io/albertdobmeyer/opentrapp/vault-agent`.
            // The service name is the last path segment.
            let service = key.rsplit('/').next().unwrap_or(key.as_str());
            if allowed.contains(service) { None } else { Some(key.clone()) }
        })
        .collect();

    for k in &keys_to_remove {
        images.remove(k);
    }

    let filtered_json = match serde_json::to_string_pretty(&overlay) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cargo:warning=stage_images: serialise error: {e}");
            return;
        }
    };

    // Write only if something actually changed — avoids spurious rebuilds.
    if filtered_json.trim() != json_text.trim() {
        if let Err(e) = std::fs::write(&overlay_path, &filtered_json) {
            eprintln!("cargo:warning=stage_images: write error: {e}");
        } else {
            eprintln!(
                "cargo:warning=stage_images: profile '{profile}' — removed {} image(s): {}",
                keys_to_remove.len(),
                keys_to_remove.join(", ")
            );
        }
    }
}

fn main() {
    stage_perimeter_resources();
    stage_sentinel();
    stage_manifests();
    stage_images();
    tauri_build::build()
}
