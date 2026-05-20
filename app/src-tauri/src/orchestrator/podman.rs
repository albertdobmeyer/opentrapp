//! Native podman orchestrator.
//!
//! Replaces the `podman compose` / `docker compose` shell-out (which depended
//! on whatever un-pinned compose provider the host happened to have — the
//! v0.4.1 first-launch failure) with direct `podman` invocations driven by the
//! signed [`PerimeterSpec`](super::perimeter::PerimeterSpec).
//!
//! The only host dependency is `podman` itself, whose version we can check.
//! Image trust is delegated to an [`ImageVerifier`] (cosign + digest pin in
//! production — see step 5); this module is verification-agnostic.
//!
//! Container DNS: containers are named exactly by service (`vault-proxy`, …)
//! so podman's resolver maps the service name to its address on every shared
//! network with no alias configuration. Grouping/reaping is by label.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Output};
use std::time::Duration;

use serde::Deserialize;

use super::error::OrchestratorError;
use super::perimeter::{
    DependencyCondition, EnvKind, ImageRef, ImageSource, MountKind, PerimeterSpec, Service,
};
use crate::lifecycle::redact_secrets;

/// Compose-project-style prefix for networks + labels. Single perimeter per host.
pub const PROJECT: &str = "opentrapp";

/// Resolves + verifies an image reference to a runnable, pinned ref string.
///
/// Production impl (step 5) checks a signed digest overlay + `cosign verify`
/// against our CI identity before returning `repo@sha256:…`. The orchestrator
/// never runs an image this trait hasn't blessed.
pub trait ImageVerifier {
    fn verify_and_resolve(&self, image: &ImageRef) -> Result<String, OrchestratorError>;
}

/// Everything the arg-builders need that isn't in the spec: where verified
/// resource files live, the resolved runtime env (secrets already looked up),
/// and the image verifier.
pub struct RunContext<'a> {
    /// Verified, non-agent-writable dir holding policy files (seccomp profiles,
    /// vault-proxy.py, allowlist.txt, resolv.conf). See ADR-0009.
    pub resource_dir: &'a Path,
    /// Runtime environment (from the user's `.env`) for secret resolution.
    pub env: &'a BTreeMap<String, String>,
    pub verifier: &'a dyn ImageVerifier,
}

fn net_name(net: &str) -> String {
    format!("{PROJECT}_{net}")
}

// ─── Pure arg builders (unit-tested; no process spawned) ────────────────

/// `podman network create …` args for one network, or `None` if it should be
/// skipped (we never skip today, but keeps the call site total).
pub fn network_create_args(name: &str, internal: bool, subnet: Option<&str>) -> Vec<String> {
    let mut a = vec!["network".into(), "create".into()];
    if internal {
        a.push("--internal".into());
    }
    if let Some(s) = subnet {
        a.push("--subnet".into());
        a.push(s.to_string());
    }
    a.push(net_name(name));
    a
}

/// Build the full `podman run` argument vector for one service.
///
/// `resolved_image` is the verifier's output (a pinned ref); passing it in
/// keeps this function pure and testable. Returns an error only when a required
/// secret env var is absent.
pub fn container_run_args(
    service_name: &str,
    svc: &Service,
    resolved_image: &str,
    ctx: &RunContext,
) -> Result<Vec<String>, OrchestratorError> {
    let mut a: Vec<String> = vec!["run".into(), "--detach".into()];

    a.push("--name".into());
    a.push(service_name.to_string());

    // Grouping/reaping labels. The compose-compat label keeps the existing
    // watchdog/diagnostics readers working until step 3 migrates them.
    for (k, v) in [
        ("com.docker.compose.service", service_name),
        ("com.docker.compose.project", PROJECT),
        ("io.opentrapp.service", service_name),
        ("io.opentrapp.perimeter", "1"),
    ] {
        a.push("--label".into());
        a.push(format!("{k}={v}"));
    }

    a.push("--restart".into());
    a.push(svc.restart.clone());

    if svc.read_only {
        a.push("--read-only".into());
    }
    if svc.cap_drop_all {
        a.push("--cap-drop".into());
        a.push("ALL".into());
    }
    for cap in &svc.cap_add {
        a.push("--cap-add".into());
        a.push(cap.clone());
    }
    if svc.no_new_privileges {
        a.push("--security-opt".into());
        a.push("no-new-privileges".into());
    }
    if let Some(seccomp) = &svc.seccomp {
        // Resolve to the verified resource path — never an absolute dev path.
        let p = ctx.resource_dir.join(seccomp);
        a.push("--security-opt".into());
        a.push(format!("seccomp={}", p.display()));
    }
    if let Some(n) = svc.pids_limit {
        a.push("--pids-limit".into());
        a.push(n.to_string());
    }
    if let Some(m) = &svc.mem_limit {
        a.push("--memory".into());
        a.push(m.clone());
    }
    if let Some(c) = svc.cpus {
        a.push("--cpus".into());
        a.push(format!("{c}"));
    }
    for t in &svc.tmpfs {
        a.push("--tmpfs".into());
        match &t.options {
            Some(o) => a.push(format!("{}:{}", t.path, o)),
            None => a.push(t.path.clone()),
        }
    }
    for s in &svc.sysctls {
        a.push("--sysctl".into());
        a.push(format!("{}={}", s.key, s.value));
    }
    for net in &svc.networks {
        a.push("--network".into());
        a.push(net_name(net));
    }
    for v in &svc.volumes {
        let source = match v.kind {
            MountKind::Named => v.source.clone(),
            // Resource mounts resolve to the verified, read-only resource dir.
            MountKind::Resource => ctx.resource_dir.join(&v.source).display().to_string(),
        };
        let mut spec = format!("{}:{}", source, v.target);
        if v.read_only {
            spec.push_str(":ro");
        }
        a.push("-v".into());
        a.push(spec);
    }
    for e in &svc.env {
        match e.kind {
            EnvKind::Literal => {
                let val = e.value.clone().unwrap_or_default();
                a.push("-e".into());
                a.push(format!("{}={}", e.name, val));
            }
            EnvKind::Secret => {
                let var = e.var.as_deref().unwrap_or(&e.name);
                match ctx.env.get(var) {
                    Some(val) => {
                        a.push("-e".into());
                        a.push(format!("{}={}", e.name, val));
                    }
                    None => {
                        if let Some(def) = &e.default {
                            a.push("-e".into());
                            a.push(format!("{}={}", e.name, def));
                        } else if e.optional {
                            // tolerated absence (e.g. OPENAI_API_KEY)
                        } else {
                            return Err(OrchestratorError::ExecutionError(format!(
                                "required secret '{var}' for {service_name} is not set"
                            )));
                        }
                    }
                }
            }
        }
    }
    if let Some(hc) = &svc.healthcheck {
        a.push("--health-cmd".into());
        a.push(hc.test.clone());
        if let Some(i) = &hc.interval {
            a.push("--health-interval".into());
            a.push(i.clone());
        }
        if let Some(t) = &hc.timeout {
            a.push("--health-timeout".into());
            a.push(t.clone());
        }
        if let Some(r) = hc.retries {
            a.push("--health-retries".into());
            a.push(r.to_string());
        }
        if let Some(sp) = &hc.start_period {
            a.push("--health-start-period".into());
            a.push(sp.clone());
        }
    }
    if svc.stdin_open {
        a.push("-i".into());
    }
    if svc.tty {
        a.push("-t".into());
    }

    a.push(resolved_image.to_string());
    if let Some(cmd) = &svc.command {
        a.extend(cmd.iter().cloned());
    }
    Ok(a)
}

// ─── Process helpers ────────────────────────────────────────────────────

/// Env vars an AppImage injects to point at its OWN bundled libraries. When the
/// app shells out to system `podman`/`conmon`, these poison the child: conmon
/// loads the AppImage's glib and dies with `undefined symbol:
/// g_assertion_message_cmpint`. We strip them so child processes use system
/// libs. (Confirmed: system conmon works with a clean env, fails with the
/// AppImage LD_LIBRARY_PATH.)
const APPIMAGE_LIB_ENV: &[&str] = &[
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "GTK_PATH",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GIO_MODULE_DIR",
    "GSETTINGS_SCHEMA_DIR",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
];

fn system_command(program: &str) -> StdCommand {
    let mut cmd = StdCommand::new(program);
    for var in APPIMAGE_LIB_ENV {
        cmd.env_remove(var);
    }
    cmd
}

/// Run `podman <args>` with a timeout wrapper (falls back to a direct call if
/// `timeout(1)` is absent). Spawned with a sanitized env so system podman/conmon
/// use system libraries, not the AppImage's bundled ones. Stderr is redacted.
fn podman(args: &[String], timeout: Duration) -> Result<Output, OrchestratorError> {
    let secs = timeout.as_secs().max(1).to_string();
    let wrapped = system_command("timeout")
        .args(["--signal=TERM", "--kill-after=5s", &secs, "podman"])
        .args(args)
        .output();
    let out = match wrapped {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            system_command("podman").args(args).output()
        }
        other => other,
    }
    .map_err(OrchestratorError::IoError)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        eprintln!(
            "[orchestrator] podman {} → {}: {}",
            args.first().map(String::as_str).unwrap_or(""),
            out.status,
            redact_secrets(stderr.trim())
        );
    }
    Ok(out)
}

fn ok(out: &Output) -> bool {
    out.status.success()
}

// ─── Orchestration ──────────────────────────────────────────────────────

/// Create any missing perimeter networks. Idempotent.
pub fn ensure_networks(spec: &PerimeterSpec) -> Result<(), OrchestratorError> {
    for (name, net) in &spec.networks {
        let exists = podman(
            &["network".into(), "exists".into(), net_name(name)],
            Duration::from_secs(10),
        )?;
        if ok(&exists) {
            continue;
        }
        let created = podman(
            &network_create_args(name, net.internal, net.subnet.as_deref()),
            Duration::from_secs(30),
        )?;
        if !ok(&created) {
            return Err(OrchestratorError::ExecutionError(format!(
                "failed to create network {}",
                net_name(name)
            )));
        }
    }
    Ok(())
}

/// Remove a single perimeter container by name. Idempotent (`--ignore`).
pub fn rm_service(service_name: &str) -> Result<(), OrchestratorError> {
    podman(
        &[
            "rm".into(),
            "--force".into(),
            "--ignore".into(),
            service_name.to_string(),
        ],
        Duration::from_secs(30),
    )?;
    Ok(())
}

/// Bring the whole perimeter up, in dependency order, waiting on `healthy`
/// dependencies before starting dependents.
pub fn up(spec: &PerimeterSpec, ctx: &RunContext) -> Result<(), OrchestratorError> {
    ensure_networks(spec)?;
    for service_name in spec.start_order() {
        let svc = &spec.services[&service_name];

        // Wait for any health-gated dependency to actually be healthy.
        for dep in &svc.depends_on {
            if matches!(dep.condition, DependencyCondition::Healthy) {
                wait_healthy(&dep.service, Duration::from_secs(60))?;
            }
        }

        let image = ctx.verifier.verify_and_resolve(&svc.image)?;
        // Clear any orphan with the same name before (re)creating.
        rm_service(&service_name)?;
        let args = container_run_args(&service_name, svc, &image, ctx)?;
        let out = podman(&args, Duration::from_secs(120))?;
        if !ok(&out) {
            return Err(OrchestratorError::ExecutionError(format!(
                "failed to start {service_name}"
            )));
        }
    }
    Ok(())
}

/// Tear the whole perimeter down (reverse start order).
pub fn down(spec: &PerimeterSpec) -> Result<(), OrchestratorError> {
    let mut order = spec.start_order();
    order.reverse();
    for service_name in order {
        rm_service(&service_name)?;
    }
    Ok(())
}

/// Poll a container's health until `healthy` or timeout.
fn wait_healthy(service_name: &str, timeout: Duration) -> Result<(), OrchestratorError> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let out = podman(
            &[
                "inspect".into(),
                "--format".into(),
                "{{.State.Health.Status}}".into(),
                service_name.to_string(),
            ],
            Duration::from_secs(10),
        )?;
        if ok(&out) {
            let status = String::from_utf8_lossy(&out.stdout);
            match status.trim() {
                "healthy" => return Ok(()),
                "unhealthy" => {
                    return Err(OrchestratorError::ExecutionError(format!(
                        "{service_name} reported unhealthy"
                    )))
                }
                _ => {}
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(OrchestratorError::Timeout(timeout.as_secs()));
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Resolve `built` images to `repo:latest` and pass `external` refs through.
/// DEV-ONLY: performs no signature/digest verification. Used only when no
/// signed image-digests overlay is bundled (i.e. a `cargo tauri dev` run).
/// Loud by design.
pub struct DevVerifier;

impl ImageVerifier for DevVerifier {
    fn verify_and_resolve(&self, image: &ImageRef) -> Result<String, OrchestratorError> {
        match image.source {
            ImageSource::External => image
                .r#ref
                .clone()
                .ok_or_else(|| OrchestratorError::ExecutionError("external image missing ref".into())),
            ImageSource::Built => {
                let repo = image.repo.clone().ok_or_else(|| {
                    OrchestratorError::ExecutionError("built image missing repo".into())
                })?;
                eprintln!(
                    "[orchestrator] WARNING DevVerifier: running {repo}:latest WITHOUT \
                     signature/digest verification (dev only)"
                );
                Ok(format!("{repo}:latest"))
            }
        }
    }
}

// ─── Production verifier: digest-pinned, bundle-backed ──────────────────
//
// Trust model (offline-first, zero-trust): the image tarballs + this overlay
// are bundled INSIDE the cosign-signed AppImage, so they inherit the AppImage
// signature. At runtime we do not re-run cosign (Karen has no cosign, and
// keyless verify needs network) — instead we pin every image to the digest in
// the signed overlay and refuse anything that doesn't match. The cosign keyless
// signatures on GHCR (step 4) are the public/audit axis, verified by CI and
// security researchers, not on Karen's machine.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ImageEntry {
    pub digest: String,
    pub source: String,
    /// Tarball filename (relative to the images dir) to `podman load`.
    #[serde(default)]
    pub tar: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageDigestOverlay {
    pub version: u32,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub signer_identity_regexp: String,
    #[serde(default)]
    pub oidc_issuer: String,
    /// repo (without tag/digest) → pinned digest + source + tarball name.
    pub images: BTreeMap<String, ImageEntry>,
}

impl ImageDigestOverlay {
    pub fn parse(json: &str) -> Result<Self, OrchestratorError> {
        serde_json::from_str(json)
            .map_err(|e| OrchestratorError::ExecutionError(format!("image-digests overlay parse: {e}")))
    }
}

/// Strip any `:tag` or `@digest` suffix to get the bare repo key used in the
/// overlay. Handles registry hosts with ports (`host:5000/repo`) by only
/// treating a colon in the final path segment as a tag separator.
fn repo_key(reference: &str) -> String {
    let at = reference.split('@').next().unwrap_or(reference);
    match at.rsplit_once('/') {
        Some((prefix, last)) => match last.rsplit_once(':') {
            Some((name, _tag)) => format!("{prefix}/{name}"),
            None => at.to_string(),
        },
        None => match at.rsplit_once(':') {
            Some((name, _tag)) => name.to_string(),
            None => at.to_string(),
        },
    }
}

pub struct BundleVerifier {
    overlay: ImageDigestOverlay,
    images_dir: PathBuf,
}

impl BundleVerifier {
    /// Load the signed overlay from `<images_dir>/image-digests.json`.
    pub fn load(images_dir: &Path) -> Result<Self, OrchestratorError> {
        let json = std::fs::read_to_string(images_dir.join("image-digests.json"))
            .map_err(OrchestratorError::IoError)?;
        Ok(Self { overlay: ImageDigestOverlay::parse(&json)?, images_dir: images_dir.to_path_buf() })
    }

    /// PURE: resolve an ImageRef to its overlay key + pinned `repo@digest`.
    /// Errors if the image isn't in the signed overlay. Unit-tested.
    pub fn pinned_ref(&self, image: &ImageRef) -> Result<(String, String, Option<String>), OrchestratorError> {
        let reference = match image.source {
            ImageSource::Built => image
                .repo
                .clone()
                .ok_or_else(|| OrchestratorError::ExecutionError("built image missing repo".into()))?,
            ImageSource::External => image
                .r#ref
                .clone()
                .ok_or_else(|| OrchestratorError::ExecutionError("external image missing ref".into()))?,
        };
        let key = repo_key(&reference);
        let entry = self.overlay.images.get(&key).ok_or_else(|| {
            OrchestratorError::ExecutionError(format!("image '{key}' is not in the signed overlay — refusing"))
        })?;
        Ok((key.clone(), format!("{key}@{}", entry.digest), entry.tar.clone()))
    }
}

impl ImageVerifier for BundleVerifier {
    fn verify_and_resolve(&self, image: &ImageRef) -> Result<String, OrchestratorError> {
        let (key, pinned, tar) = self.pinned_ref(image)?;
        if !image_present(&pinned) {
            let source = self
                .overlay
                .images
                .get(&key)
                .map(|e| e.source.as_str())
                .unwrap_or("built");
            if source == "external" {
                // Public, digest-pinned upstream image (mitmproxy). Pull it by
                // digest — tamper-proof via the pin, and avoids bundling a third
                // party's image whose oci-archive round-trips unreliably. First
                // launch is online anyway.
                let out = podman(&["pull".into(), pinned.clone()], Duration::from_secs(120))?;
                if !ok(&out) {
                    return Err(OrchestratorError::ExecutionError(format!(
                        "failed to pull external image {pinned}"
                    )));
                }
            } else {
                // Built image: load from the bundled tarball.
                let tar_name =
                    tar.unwrap_or_else(|| format!("{}.tar", key.rsplit('/').next().unwrap_or(&key)));
                let tar_path = self.images_dir.join(&tar_name);
                let out = podman(
                    &["load".into(), "--input".into(), tar_path.display().to_string()],
                    Duration::from_secs(120),
                )?;
                if !ok(&out) {
                    return Err(OrchestratorError::ExecutionError(format!(
                        "failed to load bundled image {tar_name}"
                    )));
                }
            }
        }
        // The image must now be present AT THE PINNED DIGEST. A tampered tarball
        // (or a pull that didn't match) loads under a different digest → this
        // check fails → we refuse.
        if !image_present(&pinned) {
            return Err(OrchestratorError::ExecutionError(format!(
                "image digest mismatch for {key} — expected {pinned} not present; refusing"
            )));
        }
        Ok(pinned)
    }
}

/// True if an image is present locally at an exact `repo@sha256:…` ref.
fn image_present(reference: &str) -> bool {
    podman(
        &["image".into(), "exists".into(), reference.to_string()],
        Duration::from_secs(10),
    )
    .map(|o| ok(&o))
    .unwrap_or(false)
}

/// Select the verifier for a runtime: the digest-pinned [`BundleVerifier`] when
/// a signed overlay is bundled, else the loud dev fallback.
pub fn make_verifier(resource_dir: &Path) -> Box<dyn ImageVerifier> {
    let images_dir = resource_dir.join("images");
    match BundleVerifier::load(&images_dir) {
        Ok(v) => Box::new(v),
        Err(_) => {
            eprintln!("[orchestrator] no signed image overlay found — using DevVerifier (dev only)");
            Box::new(DevVerifier)
        }
    }
}

/// Re-stage the verified policy files from the signed bundle resource dir into
/// the runtime resource dir on every launch (overwriting any tampering). The
/// bundle copy lives inside the read-only, signature-covered AppImage, so it is
/// the source of truth; the runtime copy is what containers bind-mount.
pub fn stage_resources_from_bundle(bundle_perimeter_dir: &Path, runtime_resource_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(runtime_resource_dir)?;
    for entry in std::fs::read_dir(bundle_perimeter_dir)? {
        let entry = entry?;
        let src = entry.path();
        if src.is_file() {
            let dest = runtime_resource_dir.join(entry.file_name());
            std::fs::copy(&src, &dest)?;
        }
    }
    Ok(())
}

/// `podman load` every `*.tar` in the bundle's images dir. Reads directly from
/// the read-only AppImage mount (no multi-hundred-MB copy into the data dir);
/// once loaded, images persist in podman storage so later launches are no-ops.
/// Digest verification happens later, in [`BundleVerifier::verify_and_resolve`].
pub fn load_bundled_images(bundle_images_dir: &Path) -> Result<(), OrchestratorError> {
    if !bundle_images_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(bundle_images_dir).map_err(OrchestratorError::IoError)? {
        let path = entry.map_err(OrchestratorError::IoError)?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("tar") {
            let out = podman(
                &["load".into(), "--input".into(), path.display().to_string()],
                Duration::from_secs(180),
            )?;
            if !ok(&out) {
                eprintln!("[orchestrator] failed to load bundled image {}", path.display());
            }
        }
    }
    Ok(())
}

// ─── Runtime paths + env ────────────────────────────────────────────────

/// The user's runtime data home — where markers, `.env`, and the verified
/// `perimeter/` resources live. Matches `lifecycle::runguard_dir()`.
pub fn runtime_data_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".opentrapp")
}

/// Verified, non-agent-writable resource dir (seccomp profiles, vault-proxy.py,
/// allowlist.txt, resolv.conf). Populated from the signed bundle at first
/// launch (step 5).
pub fn resource_dir() -> PathBuf {
    runtime_data_dir().join("perimeter")
}

/// Parse `~/.opentrapp/.env` into a map for secret resolution. Tolerant of
/// blank lines, `#` comments, `export ` prefixes, and surrounding quotes.
pub fn load_runtime_env(data_dir: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(data_dir.join(".env")) else {
        return map;
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim().to_string();
            let mut v = v.trim();
            if v.len() >= 2
                && ((v.starts_with('"') && v.ends_with('"'))
                    || (v.starts_with('\'') && v.ends_with('\'')))
            {
                v = &v[1..v.len() - 1];
            }
            map.insert(k, v.to_string());
        }
    }
    map
}

fn load_spec() -> Result<PerimeterSpec, OrchestratorError> {
    super::perimeter::load()
        .map_err(|e| OrchestratorError::ExecutionError(format!("perimeter spec parse: {e}")))
}

fn is_running(service_name: &str) -> bool {
    podman(
        &[
            "ps".into(),
            "--filter".into(),
            format!("name=^{service_name}$"),
            "--filter".into(),
            "status=running".into(),
            "--format".into(),
            "{{.Names}}".into(),
        ],
        Duration::from_secs(10),
    )
    .map(|o| ok(&o) && !o.stdout.trim_ascii().is_empty())
    .unwrap_or(false)
}

// ─── Lifecycle façade (drop-in replacements for run_compose call sites) ──
// Each takes the runtime data dir (where `.env` lives) and internally loads
// the signed spec + builds the run context. Keeps call sites a one-liner.

/// Bring up every service in dependency order (proxy+forge+pioneer+egress+agent).
pub fn perimeter_up(data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    let env = load_runtime_env(data_dir);
    let rd = resource_dir();
    let verifier = make_verifier(&rd);
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: verifier.as_ref() };
    up(&spec, &ctx)
}

/// Bring up the security shell only (everything except the agent tenant).
pub fn shell_up(data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    let env = load_runtime_env(data_dir);
    let rd = resource_dir();
    let verifier = make_verifier(&rd);
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: verifier.as_ref() };
    ensure_networks(&spec)?;
    for service_name in spec.start_order() {
        if service_name == "vault-agent" {
            continue;
        }
        let svc = &spec.services[&service_name];
        for dep in &svc.depends_on {
            if matches!(dep.condition, DependencyCondition::Healthy) {
                wait_healthy(&dep.service, Duration::from_secs(60))?;
            }
        }
        let image = ctx.verifier.verify_and_resolve(&svc.image)?;
        rm_service(&service_name)?;
        let args = container_run_args(&service_name, svc, &image, &ctx)?;
        if !ok(&podman(&args, Duration::from_secs(120))?) {
            return Err(OrchestratorError::ExecutionError(format!(
                "failed to start {service_name}"
            )));
        }
    }
    Ok(())
}

/// Tear the whole perimeter down.
pub fn perimeter_down(_data_dir: &Path) -> Result<(), OrchestratorError> {
    down(&load_spec()?)
}

/// Stop (freeze, keep containers) the whole perimeter — used by pause.
pub fn perimeter_stop(_data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    for service_name in spec.services.keys() {
        podman(
            &["stop".into(), "--time".into(), "10".into(), service_name.clone()],
            Duration::from_secs(20),
        )?;
    }
    Ok(())
}

/// Start one service (verifying its image). `force_recreate` removes any
/// existing instance first; otherwise a running instance is left as-is.
pub fn service_up(
    data_dir: &Path,
    service_name: &str,
    force_recreate: bool,
) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    let svc = spec.services.get(service_name).ok_or_else(|| {
        OrchestratorError::ExecutionError(format!("unknown service {service_name}"))
    })?;
    let env = load_runtime_env(data_dir);
    let rd = resource_dir();
    let verifier = make_verifier(&rd);
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: verifier.as_ref() };
    ensure_networks(&spec)?;
    if !force_recreate && is_running(service_name) {
        return Ok(());
    }
    rm_service(service_name)?;
    let image = ctx.verifier.verify_and_resolve(&svc.image)?;
    let args = container_run_args(service_name, svc, &image, &ctx)?;
    if !ok(&podman(&args, Duration::from_secs(120))?) {
        return Err(OrchestratorError::ExecutionError(format!(
            "failed to start {service_name}"
        )));
    }
    Ok(())
}

/// Verify + acquire every image the perimeter needs. With a bundled signed
/// overlay this loads each image from its tarball and pins it to the overlay
/// digest, refusing any mismatch. Without an overlay (dev), it resolves refs
/// and logs anything missing.
pub fn ensure_images(_data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    let verifier = make_verifier(&resource_dir());
    for (name, svc) in &spec.services {
        match verifier.verify_and_resolve(&svc.image) {
            Ok(resolved) => eprintln!("[orchestrator] image ready for {name}: {resolved}"),
            Err(e) => {
                eprintln!("[orchestrator] image NOT ready for {name}: {e}");
                return Err(e);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::perimeter;

    fn ctx_with<'a>(env: &'a BTreeMap<String, String>, res: &'a Path) -> RunContext<'a> {
        RunContext { resource_dir: res, env, verifier: &DevVerifier }
    }

    #[test]
    fn agent_args_are_maximally_contained() {
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::from([("TELEGRAM_BOT_TOKEN".into(), "T".into())]);
        let res = Path::new("/run/opentrapp/perimeter");
        let args =
            container_run_args("vault-agent", &spec.services["vault-agent"], "img:latest", &ctx_with(&env, &res))
                .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("--cap-drop ALL"));
        assert!(joined.contains("--read-only"));
        assert!(joined.contains("--security-opt no-new-privileges"));
        assert!(joined.contains("--network opentrapp_agent-net"));
        // seccomp resolved under the resource dir, NOT an absolute dev path
        assert!(joined.contains("seccomp=/run/opentrapp/perimeter/vault-seccomp.json"));
        assert!(!joined.contains("/home/albertd"));
        // secret resolved from env, not inlined as a placeholder
        assert!(joined.contains("-e TELEGRAM_BOT_TOKEN=T"));
        // name + image + label present
        assert!(joined.contains("--name vault-agent"));
        assert!(joined.ends_with("img:latest"));
        assert!(joined.contains("io.opentrapp.service=vault-agent"));
    }

    #[test]
    fn missing_required_secret_errors() {
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::new(); // ANTHROPIC_API_KEY absent
        let res = Path::new("/run/opentrapp/perimeter");
        let err = container_run_args(
            "vault-proxy",
            &spec.services["vault-proxy"],
            "mitm@sha256:x",
            &ctx_with(&env, &res),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn optional_secret_is_skipped_when_absent() {
        let spec = perimeter::load().unwrap();
        // ANTHROPIC present, OPENAI absent (optional), version has default
        let env = BTreeMap::from([("ANTHROPIC_API_KEY".into(), "sk".into())]);
        let res = Path::new("/run/opentrapp/perimeter");
        let args = container_run_args(
            "vault-proxy",
            &spec.services["vault-proxy"],
            "mitm@sha256:x",
            &ctx_with(&env, &res),
        )
        .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("-e ANTHROPIC_API_KEY=sk"));
        assert!(!joined.contains("OPENAI_API_KEY="), "optional+absent → omitted");
        assert!(joined.contains("-e ANTHROPIC_API_VERSION=2023-06-01"), "default applied");
    }

    #[test]
    fn egress_has_net_admin_and_sysctls() {
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::new();
        let res = Path::new("/run/opentrapp/perimeter");
        let args =
            container_run_args("vault-egress", &spec.services["vault-egress"], "img@sha256:x", &ctx_with(&env, &res))
                .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("--cap-add NET_ADMIN"));
        assert!(joined.contains("--sysctl net.ipv4.ip_forward=1"));
        assert!(joined.contains("--health-cmd"));
        // resource mount resolved under resource dir, read-only
        assert!(joined.contains("/run/opentrapp/perimeter/resolv.conf:/etc/resolv.conf:ro"));
    }

    #[test]
    fn network_create_args_internal_with_subnet() {
        let a = network_create_args("egress-net", true, Some("10.230.0.0/24")).join(" ");
        assert_eq!(a, "network create --internal --subnet 10.230.0.0/24 opentrapp_egress-net");
        let b = network_create_args("external-net", false, None).join(" ");
        assert_eq!(b, "network create opentrapp_external-net");
    }

    const OVERLAY_JSON: &str = r#"{
      "version": 1,
      "tag": "v9.9.9",
      "signer_identity_regexp": "https://github.com/albertdobmeyer/opentrapp/.github/workflows/ci.yml@refs/tags/.*",
      "oidc_issuer": "https://token.actions.githubusercontent.com",
      "images": {
        "ghcr.io/albertdobmeyer/opentrapp/vault-agent": { "digest": "sha256:aaa", "source": "built", "tar": "vault-agent.tar" },
        "docker.io/mitmproxy/mitmproxy": { "digest": "sha256:bbb", "source": "external", "tar": "vault-proxy.tar" }
      }
    }"#;

    fn overlay() -> BundleVerifier {
        BundleVerifier { overlay: ImageDigestOverlay::parse(OVERLAY_JSON).unwrap(), images_dir: PathBuf::from("/x") }
    }

    #[test]
    fn system_command_strips_appimage_lib_env() {
        // Regression for the conmon `undefined symbol: g_assertion_message_cmpint`
        // failure: child podman/conmon must NOT inherit the AppImage's
        // LD_LIBRARY_PATH. Set a sentinel and confirm the child doesn't see it.
        std::env::set_var("LD_LIBRARY_PATH", "/poison/appimage/lib");
        let out = system_command("sh")
            .args(["-c", "printf %s \"${LD_LIBRARY_PATH-UNSET}\""])
            .output()
            .unwrap();
        std::env::remove_var("LD_LIBRARY_PATH");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "UNSET");
    }

    #[test]
    fn repo_key_strips_tag_and_digest_but_keeps_port() {
        assert_eq!(repo_key("ghcr.io/o/opentrapp/vault-agent:v1"), "ghcr.io/o/opentrapp/vault-agent");
        assert_eq!(repo_key("docker.io/mitmproxy/mitmproxy@sha256:abc"), "docker.io/mitmproxy/mitmproxy");
        assert_eq!(repo_key("host:5000/repo:tag"), "host:5000/repo");
    }

    #[test]
    fn overlay_parses_and_pins_built_image_by_digest() {
        let v = overlay();
        let img = ImageRef {
            source: ImageSource::Built,
            repo: Some("ghcr.io/albertdobmeyer/opentrapp/vault-agent".into()),
            r#ref: None,
        };
        let (key, pinned, tar) = v.pinned_ref(&img).unwrap();
        assert_eq!(key, "ghcr.io/albertdobmeyer/opentrapp/vault-agent");
        assert_eq!(pinned, "ghcr.io/albertdobmeyer/opentrapp/vault-agent@sha256:aaa");
        assert_eq!(tar.as_deref(), Some("vault-agent.tar"));
    }

    #[test]
    fn overlay_pins_external_image_by_overlay_digest() {
        let v = overlay();
        let img = ImageRef {
            source: ImageSource::External,
            repo: None,
            r#ref: Some("docker.io/mitmproxy/mitmproxy@sha256:bbb".into()),
        };
        let (_k, pinned, _t) = v.pinned_ref(&img).unwrap();
        assert_eq!(pinned, "docker.io/mitmproxy/mitmproxy@sha256:bbb");
    }

    #[test]
    fn unknown_image_is_refused() {
        // An image not present in the signed overlay must be rejected — this is
        // the tamper guard (e.g. an attacker swaps in a different image repo).
        let v = overlay();
        let img = ImageRef {
            source: ImageSource::Built,
            repo: Some("ghcr.io/attacker/evil".into()),
            r#ref: None,
        };
        let err = format!("{}", v.pinned_ref(&img).unwrap_err());
        assert!(err.contains("not in the signed overlay"), "got: {err}");
    }

    /// LIVE: drives the real arg builders against podman to prove the spec
    /// actually produces a working `podman run` (string assertions can't catch
    /// a flag podman rejects). Uses vault-forge — no creds, no dependencies.
    /// Requires `ghcr.io/albertdobmeyer/opentrapp/vault-forge:latest` present.
    /// Run with: `cargo test --lib -- --ignored live_forge_brings_up`.
    #[test]
    #[ignore]
    fn live_forge_brings_up_and_tears_down() {
        let spec = perimeter::load().unwrap();
        let svc = &spec.services["vault-forge"];
        let env = BTreeMap::new();
        let res = std::env::temp_dir().join("opentrapp-live-test");
        let ctx = ctx_with(&env, &res);
        let image = "ghcr.io/albertdobmeyer/opentrapp/vault-forge:latest";

        // Clean slate.
        let _ = podman(&["rm".into(), "--force".into(), "--ignore".into(), "vault-forge".into()], Duration::from_secs(20));
        let _ = podman(&["network".into(), "rm".into(), "--force".into(), net_name("forge-net")], Duration::from_secs(20));

        // Real network create + real run args.
        assert!(ok(&podman(&network_create_args("forge-net", true, None), Duration::from_secs(20)).unwrap()));
        let args = container_run_args("vault-forge", svc, image, &ctx).unwrap();
        let run = podman(&args, Duration::from_secs(60)).unwrap();
        assert!(ok(&run), "podman run rejected the generated args");

        // It must be running, and carry our label.
        assert!(is_running("vault-forge"), "vault-forge not running after up");
        let labeled = podman(
            &["ps".into(), "--filter".into(), "label=io.opentrapp.service=vault-forge".into(), "--format".into(), "{{.Names}}".into()],
            Duration::from_secs(10),
        ).unwrap();
        assert!(String::from_utf8_lossy(&labeled.stdout).contains("vault-forge"));

        // Tear down.
        assert!(ok(&podman(&["rm".into(), "--force".into(), "vault-forge".into()], Duration::from_secs(20)).unwrap()));
        let _ = podman(&["network".into(), "rm".into(), "--force".into(), net_name("forge-net")], Duration::from_secs(20));
        assert!(!is_running("vault-forge"), "vault-forge still running after down");
    }

    /// LIVE: the digest-tamper guard, end to end. A real local image, an overlay
    /// that pins it to the WRONG digest → verify_and_resolve must refuse (the
    /// wrong digest is never present, and the bundled tar is absent here).
    /// Run with: `cargo test --lib -- --ignored live_tampered_digest`.
    #[test]
    #[ignore]
    fn live_tampered_digest_is_refused() {
        let overlay_json = r#"{
          "version": 1, "tag": "vtest",
          "signer_identity_regexp": "x", "oidc_issuer": "y",
          "images": { "ghcr.io/albertdobmeyer/opentrapp/vault-forge": { "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000", "source": "built", "tar": "vault-forge.tar" } }
        }"#;
        let verifier = BundleVerifier {
            overlay: ImageDigestOverlay::parse(overlay_json).unwrap(),
            images_dir: std::env::temp_dir().join("opentrapp-no-tars"),
        };
        let img = ImageRef {
            source: ImageSource::Built,
            repo: Some("ghcr.io/albertdobmeyer/opentrapp/vault-forge".into()),
            r#ref: None,
        };
        let err = verifier.verify_and_resolve(&img).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("mismatch") || msg.contains("failed to load"), "expected refusal, got: {msg}");
    }

    #[test]
    fn proxy_resource_mounts_resolved_not_bind_from_source_tree() {
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::from([("ANTHROPIC_API_KEY".into(), "sk".into())]);
        let res = Path::new("/run/opentrapp/perimeter");
        let args =
            container_run_args("vault-proxy", &spec.services["vault-proxy"], "mitm@sha256:x", &ctx_with(&env, &res))
                .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("/run/opentrapp/perimeter/vault-proxy.py:/opt/vault/vault-proxy.py:ro"));
        assert!(joined.contains("/run/opentrapp/perimeter/allowlist.txt:/opt/vault/allowlist.txt:ro"));
        assert!(!joined.contains("components/opencli-container"), "no source-tree paths");
    }
}
