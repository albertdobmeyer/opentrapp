use std::path::Path;

/// Policy/config files that must be bundled into the AppImage so the perimeter
/// can bind-mount verified copies at runtime (never from a writable host path).
/// Source of truth is the submodule; we stage a copy into `resources/perimeter/`
/// which `tauri.conf.json` bundle.resources packages. (Image tarballs +
/// image-digests.json are dropped into `resources/perimeter/images/` by CI.)
const STAGED_RESOURCES: &[&str] = &[
    "../../components/opencli-container/config/vault-seccomp.json",
    "../../components/opencli-container/config/vault-proxy-seccomp.json",
    "../../components/opencli-container/proxy/vault-proxy.py",
    "../../components/opencli-container/proxy/allowlist.txt",
    "../../components/opencli-container/egress/resolv.conf",
];

fn stage_perimeter_resources() {
    let dest_dir = Path::new("resources/perimeter");
    if std::fs::create_dir_all(dest_dir).is_err() {
        return;
    }
    for src in STAGED_RESOURCES {
        println!("cargo:rerun-if-changed={src}");
        let src_path = Path::new(src);
        if let Some(name) = src_path.file_name() {
            // Best-effort: a fresh checkout always has the submodule files; if a
            // file is missing (e.g. submodule not initialized in a dev tree), we
            // skip rather than fail the build — the bundle will simply lack it
            // and the orchestrator will surface a clear runtime error.
            let _ = std::fs::copy(src_path, dest_dir.join(name));
        }
    }
}

fn main() {
    stage_perimeter_resources();
    tauri_build::build()
}
