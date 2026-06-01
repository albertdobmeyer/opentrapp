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

fn main() {
    stage_perimeter_resources();
    stage_manifests();
    tauri_build::build()
}
