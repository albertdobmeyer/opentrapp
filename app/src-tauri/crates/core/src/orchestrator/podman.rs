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
use crate::util::secrets::redact_secrets;

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
    /// allowlist.txt, resolv.conf). See ADR-0009.
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
    // `--replace` atomically removes any same-named container before creating
    // this one. Defense-in-depth against the retry/concurrency collision
    // (`name "<svc>" is already in use`); podman >= 4.0 (we require 4.9.3+).
    let mut a: Vec<String> = vec!["run".into(), "--detach".into(), "--replace".into()];

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
        } else if v.chown {
            // Podman ':U' — chown the volume to the container's user namespace
            // mapping at mount time. Required for non-root processes writing to
            // named volumes. Mutually exclusive with ':ro' (chowning a ro mount
            // is meaningless). See VolumeMount docs.
            spec.push_str(":U");
        }
        a.push("-v".into());
        a.push(spec);
    }
    // Literals (e.g. HTTP_PROXY) are non-secret — fine to inline on the argv.
    for e in &svc.env {
        if let EnvKind::Literal = e.kind {
            let val = e.value.clone().unwrap_or_default();
            a.push("-e".into());
            a.push(format!("{}={}", e.name, val));
        }
    }
    // Secrets are passed by NAME only (`-e NAME`): podman forwards the value from
    // its own environment (injected via `podman_run` → `apply_secret_env`), so the
    // value never lands on the argv / host process table (#75). `resolve_secret_env`
    // is the single source of truth for presence (required → error, optional → omit).
    for (name, _value) in resolve_secret_env(service_name, svc, ctx)? {
        a.push("-e".into());
        a.push(name);
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

    if let Some(ep) = &svc.entrypoint {
        // Podman `--entrypoint` accepts a JSON array for a multi-element
        // entrypoint. vault-proxy uses this to chown its log volume as root
        // before the upstream image's gosu privilege-drop (the ZONE 3 fix).
        a.push("--entrypoint".into());
        a.push(serde_json::to_string(ep).unwrap_or_else(|_| "[]".into()));
    }

    a.push(resolved_image.to_string());
    if let Some(cmd) = &svc.command {
        a.extend(cmd.iter().cloned());
    }
    Ok(a)
}

/// Resolve a service's `secret`-kind env vars to `(container_var_name, value)`
/// pairs, from the runtime env (`ctx.env`) or each var's declared default. This
/// is the single source of truth for secret presence: a **required** secret that
/// is absent is an error; an **optional** one is omitted. The values are injected
/// into podman's process environment at run time (see [`podman_run`]); the argv
/// only ever carries the bare `-e NAME` passthrough — so a secret value never
/// reaches the host process table (#75).
fn resolve_secret_env(
    service_name: &str,
    svc: &Service,
    ctx: &RunContext,
) -> Result<Vec<(String, String)>, OrchestratorError> {
    let mut out = Vec::new();
    for e in &svc.env {
        if !matches!(e.kind, EnvKind::Secret) {
            continue;
        }
        let var = e.var.as_deref().unwrap_or(&e.name);
        match ctx.env.get(var).cloned().or_else(|| e.default.clone()) {
            Some(val) => out.push((e.name.clone(), val)),
            None if e.optional => {} // tolerated absence (e.g. OPENAI_API_KEY)
            None => {
                return Err(OrchestratorError::ExecutionError(format!(
                    "required secret '{var}' for {service_name} is not set"
                )));
            }
        }
    }
    Ok(out)
}

// ─── Process helpers ────────────────────────────────────────────────────

/// Env vars an AppImage injects to point at its OWN bundled libraries. When the
/// app shells out to system `podman`/`conmon`, these poison the child: conmon
/// loads the AppImage's glib and dies with `undefined symbol:
/// g_assertion_message_cmpint`. We strip them so child processes use system
/// libs. (Confirmed: system conmon works with a clean env, fails with the
/// AppImage LD_LIBRARY_PATH.)
pub(crate) const APPIMAGE_LIB_ENV: &[&str] = &[
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
    podman_raw(args, timeout, true, &[])
}

/// Like [`podman`] but never logs a non-zero exit. For existence probes
/// (`network exists`, `image exists`) where exit 1 means "not found" — an
/// expected, non-error outcome that would otherwise spam the log.
fn podman_probe(args: &[String], timeout: Duration) -> Result<Output, OrchestratorError> {
    podman_raw(args, timeout, false, &[])
}

/// Like [`podman`] but injects `secret_env` into podman's PROCESS ENVIRONMENT so
/// a `-e NAME` passthrough (see [`container_run_args`]) forwards each secret into
/// the container without the value ever appearing on the argv / host process
/// table (#75). Used for the `podman run` of perimeter services.
fn podman_run(
    args: &[String],
    timeout: Duration,
    secret_env: &[(String, String)],
) -> Result<Output, OrchestratorError> {
    podman_raw(args, timeout, true, secret_env)
}

/// Inject secret env vars into a child command's process environment (never the
/// argv) so a `-e NAME` passthrough forwards them to the container (#75).
fn apply_secret_env(cmd: &mut StdCommand, secret_env: &[(String, String)]) {
    for (k, v) in secret_env {
        cmd.env(k, v);
    }
}

fn podman_raw(
    args: &[String],
    timeout: Duration,
    log_errors: bool,
    secret_env: &[(String, String)],
) -> Result<Output, OrchestratorError> {
    let secs = timeout.as_secs().max(1).to_string();
    let mut primary = system_command("timeout");
    primary
        .args(["--signal=TERM", "--kill-after=5s", &secs, "podman"])
        .args(args);
    apply_secret_env(&mut primary, secret_env);
    let out = match primary.output() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut fallback = system_command("podman");
            fallback.args(args);
            apply_secret_env(&mut fallback, secret_env);
            fallback.output()
        }
        other => other,
    }
    .map_err(OrchestratorError::IoError)?;
    if log_errors && !out.status.success() {
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
        let exists = podman_probe(
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
    for service_name in spec.boot_services() {
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
        let secret_env = resolve_secret_env(&service_name, svc, ctx)?;
        let out = podman_run(&args, Duration::from_secs(120), &secret_env)?;
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
    /// `owner/repo` — used to build the release-asset download URL at runtime.
    #[serde(default)]
    pub repo: String,
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
    podman_probe(
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
        let dest = runtime_resource_dir.join(entry.file_name());
        if src.is_dir() {
            // Recurse: the Sentinel lib (`perimeter/sentinel/**`) is a directory
            // tree that the shield containers bind-mount at `/opt/sentinel`
            // (spec 08 §5). The `images/` dir is handled separately and is
            // re-staged here harmlessly if present.
            stage_resources_from_bundle(&src, &dest)?;
        } else if src.is_file() {
            std::fs::copy(&src, &dest)?;
        }
    }
    Ok(())
}

/// Download the built-image tarballs listed in the (bundled, signed) overlay
/// from the GitHub release into the runtime images dir. The overlay rides
/// inside the signed AppImage and defines the truth (digests); the large
/// tarballs are fetched as release assets — keeping the AppImage small while
/// the post-load digest check (in [`BundleVerifier`]) catches any tampering.
/// External images (mitmproxy) are pulled by the verifier, not downloaded here.
/// No-op in dev (no overlay) or when assets are already present.
pub async fn fetch_perimeter_images() -> Result<(), OrchestratorError> {
    let images_dir = resource_dir().join("images");
    let overlay_path = images_dir.join("image-digests.json");
    let Ok(json) = std::fs::read_to_string(&overlay_path) else {
        return Ok(()); // dev: no bundled overlay
    };
    let overlay = ImageDigestOverlay::parse(&json)?;
    if overlay.repo.is_empty() || overlay.tag.is_empty() {
        return Ok(());
    }
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| OrchestratorError::ExecutionError(format!("http client: {e}")))?;

    for (key, entry) in &overlay.images {
        if entry.source != "built" {
            continue;
        }
        let Some(tar) = &entry.tar else { continue };
        let dest = images_dir.join(tar);
        if dest.exists() {
            continue; // already fetched
        }
        let url = format!(
            "https://github.com/{}/releases/download/{}/{}",
            overlay.repo, overlay.tag, tar
        );
        eprintln!("[orchestrator] fetching image {key} ← {url}");
        let resp = client
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| OrchestratorError::ExecutionError(format!("download {tar}: {e}")))?;
        // Stream to a temp file (memory-safe for multi-hundred-MB tarballs),
        // then rename so a partial download is never seen as complete.
        let tmp = dest.with_extension("tar.part");
        {
            let mut file = std::fs::File::create(&tmp).map_err(OrchestratorError::IoError)?;
            let mut resp = resp;
            while let Some(chunk) = resp
                .chunk()
                .await
                .map_err(|e| OrchestratorError::ExecutionError(format!("download {tar}: {e}")))?
            {
                use std::io::Write as _;
                file.write_all(&chunk).map_err(OrchestratorError::IoError)?;
            }
        }
        std::fs::rename(&tmp, &dest).map_err(OrchestratorError::IoError)?;
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

/// Verified, non-agent-writable resource dir (seccomp profiles, allowlist.txt,
/// resolv.conf). Populated from the signed bundle at first launch (step 5).
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

/// Read the proxy's persistent egress log (`requests.jsonl`) from the
/// `vault-proxy-logs` named volume. Resolves the volume's host mountpoint via
/// `podman volume inspect` and reads the file. Fail-soft: returns an empty
/// string if the volume/file is absent (dev, fresh install, log not yet
/// written) — the caller treats "no log" as "no pending approvals". Used by the
/// allowlist-approval read path (v0.6 Item A).
pub fn read_egress_log() -> String {
    let out = match podman_probe(
        &[
            "volume".into(),
            "inspect".into(),
            "vault-proxy-logs".into(),
            "--format".into(),
            "{{.Mountpoint}}".into(),
        ],
        Duration::from_secs(10),
    ) {
        Ok(o) if ok(&o) => o,
        _ => return String::new(),
    };
    let mountpoint = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if mountpoint.is_empty() {
        return String::new();
    }
    std::fs::read_to_string(Path::new(&mountpoint).join("requests.jsonl")).unwrap_or_default()
}

/// Telegram bot keep-alive endpoints the agent hits *continuously* even when
/// idle (OpenClaw long-polls `getUpdates` every ~10-20s forever, plus periodic
/// `getMe`/`getWebhookInfo`). These must NOT count as activity — otherwise the
/// idle timer never goes stale and auto-pause never fires (verified live
/// 2026-06-09: the old mtime-based signal sat at ~15s indefinitely). Setters,
/// deleters, `sendMessage` replies, Anthropic calls, and tool fetches are all
/// real activity and are deliberately NOT listed here.
fn is_keepalive_poll(url: &str) -> bool {
    const POLL_MARKERS: &[&str] = &[
        "/getUpdates",
        "/getMe",
        "/getWebhookInfo",
        "/getMyCommands",
    ];
    POLL_MARKERS.iter().any(|m| url.contains(m))
}

/// Parse the egress log content and return ms since the agent's last *real-
/// activity* request (newest non-keep-alive request), or `None` if the log holds
/// no real-activity request. Pure (no I/O) so it can be unit-tested. `now_ms` is
/// the current epoch-ms.
fn last_activity_ms_from_log(content: &str, now_ms: u64) -> Option<u64> {
    for line in content.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let url = v.get("url").and_then(|u| u.as_str()).unwrap_or("");
        if url.is_empty() || is_keepalive_poll(url) {
            continue;
        }
        // Newest real-activity request. Skip entries without ts_ms (pre-fix log
        // lines) and keep scanning for an older one that has it.
        match v.get("ts_ms").and_then(|t| t.as_u64()) {
            Some(ts) => return Some(now_ms.saturating_sub(ts)),
            None => continue,
        }
    }
    None
}

/// Milliseconds since the agent's last *real-activity* egress, parsed from
/// `requests.jsonl` in the `vault-proxy-logs` volume. The idle signal for
/// auto-pause-to-dormant (Phase 3).
///
/// CRITICAL: this must NOT count the agent's own Telegram keep-alive polling.
/// OpenClaw long-polls `getUpdates` (+ periodic `getMe`/`getWebhookInfo`) every
/// ~10-20s forever, even when completely idle, so "time since *any* egress" never
/// goes stale and auto-pause would never fire. We measure idle from the most
/// recent request that is NOT a keep-alive endpoint (a real LLM call, a tool
/// fetch, or a `sendMessage` reply — all of which only happen on a real turn).
///
/// `None` when the volume/file is absent or the log holds no real-activity
/// request (e.g. fresh perimeter, started but never interacted with) — the caller
/// MUST treat that as "no signal, do not auto-pause" (fail-safe). Depends on the
/// proxy log persisting to its volume (the ZONE 3 entrypoint-chown fix).
pub fn read_egress_log_last_activity_ms() -> Option<u64> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    last_activity_ms_from_log(&read_egress_log(), now_ms)
}

/// Signal `vault-proxy` to reload its allowlist (SIGHUP → the addon's
/// `_reload_allowlist`, which does an in-memory atomic swap). No-op when the
/// proxy isn't running. The proxy reads the live allowlist file the host app
/// just appended to; this makes the change take effect without a restart.
pub fn reload_proxy_allowlist() -> Result<(), OrchestratorError> {
    if !is_running("vault-proxy") {
        return Ok(()); // nothing to reload; the merged file is read on next start
    }
    let out = podman(
        &["kill".into(), "--signal=HUP".into(), "vault-proxy".into()],
        Duration::from_secs(10),
    )?;
    if ok(&out) {
        Ok(())
    } else {
        Err(OrchestratorError::ExecutionError(
            "could not signal the gate to reload its list".into(),
        ))
    }
}

// ─── Lifecycle façade (drop-in replacements for run_compose call sites) ──
// Each takes the runtime data dir (where `.env` lives) and internally loads
// the signed spec + builds the run context. Keeps call sites a one-liner.

/// Resolve the runtime resource dir AND (re)provision it from the policy set
/// embedded in the signed binary — overwriting any tampered or stale copy.
/// Every bring-up calls this, so the daemon is self-sufficient post de-Tauri
/// (no Tauri `handle.path().resource_dir()` bundle, no build.rs staging) and the
/// bind-mounted seccomp/proxy/sentinel policy is always the signed source of
/// truth. See [`super::embedded_resources`] + the `pin_*` provisioning tests.
fn provisioned_resource_dir() -> Result<PathBuf, OrchestratorError> {
    let rd = resource_dir();
    super::embedded_resources::extract_embedded_resources(&rd)
        .map_err(|e| OrchestratorError::ExecutionError(format!("provision resources: {e}")))?;
    Ok(rd)
}

/// Bring up every service in dependency order (egress+proxy+forge+social+agent).
pub fn perimeter_up(data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    let env = load_runtime_env(data_dir);
    let rd = provisioned_resource_dir()?;
    let verifier = make_verifier(&rd);
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: verifier.as_ref() };
    up(&spec, &ctx)
}

/// Bring up the security shell only (everything except the agent tenant).
pub fn shell_up(data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    let env = load_runtime_env(data_dir);
    let rd = provisioned_resource_dir()?;
    let verifier = make_verifier(&rd);
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: verifier.as_ref() };
    ensure_networks(&spec)?;
    for service_name in spec.boot_services() {
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
        let secret_env = resolve_secret_env(&service_name, svc, &ctx)?;
        if !ok(&podman_run(&args, Duration::from_secs(120), &secret_env)?) {
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
        // `--ignore`: an on-demand shield may not be running; absence is not an error.
        podman(
            &["stop".into(), "--ignore".into(), "--time".into(), "10".into(), service_name.clone()],
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
    let rd = provisioned_resource_dir()?;
    let verifier = make_verifier(&rd);
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: verifier.as_ref() };
    ensure_networks(&spec)?;
    if !force_recreate && is_running(service_name) {
        return Ok(());
    }
    rm_service(service_name)?;
    let image = ctx.verifier.verify_and_resolve(&svc.image)?;
    let args = container_run_args(service_name, svc, &image, &ctx)?;
    let secret_env = resolve_secret_env(service_name, svc, &ctx)?;
    if !ok(&podman_run(&args, Duration::from_secs(120), &secret_env)?) {
        return Err(OrchestratorError::ExecutionError(format!(
            "failed to start {service_name}"
        )));
    }
    Ok(())
}

/// Stop and remove one on-demand service (after its idle grace). Idempotent —
/// a no-op if the container is already gone (`rm --ignore`).
pub fn service_down(service_name: &str) -> Result<(), OrchestratorError> {
    rm_service(service_name)
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
        let env = BTreeMap::from([("TELEGRAM_BOT_TOKEN".into(), "bot-SENTINEL".into())]);
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
        // secret passed by NAME only (`-e TELEGRAM_BOT_TOKEN`) so podman forwards
        // it from its env; the VALUE must never reach the argv / process table (#75).
        assert!(joined.contains("-e TELEGRAM_BOT_TOKEN"));
        assert!(!joined.contains("bot-SENTINEL"), "secret value leaked onto the argv");
        // name + image + label present
        assert!(joined.contains("--name vault-agent"));
        assert!(joined.ends_with("img:latest"));
        assert!(joined.contains("io.opentrapp.service=vault-agent"));
        // --replace so a re-run/retry atomically supersedes a stale same-named
        // container instead of colliding with "name already in use".
        assert!(joined.contains("--replace"));
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
        let env = BTreeMap::from([("ANTHROPIC_API_KEY".into(), "sk-SENTINEL".into())]);
        let res = Path::new("/run/opentrapp/perimeter");
        let args = container_run_args(
            "vault-proxy",
            &spec.services["vault-proxy"],
            "mitm@sha256:x",
            &ctx_with(&env, &res),
        )
        .unwrap();
        let joined = args.join(" ");
        // present secret + non-secret default both passed by NAME, value off the argv
        assert!(joined.contains("-e ANTHROPIC_API_KEY"));
        assert!(!joined.contains("sk-SENTINEL"), "secret value must not be on the argv");
        assert!(joined.contains("-e ANTHROPIC_API_VERSION"), "default still passed by name");
        assert!(!joined.contains("OPENAI_API_KEY"), "optional+absent → omitted entirely");
    }

    #[test]
    fn resolve_secret_env_resolves_present_default_and_omits_optional() {
        let spec = perimeter::load().unwrap();
        // ANTHROPIC present, OPENAI absent (optional), version has a default.
        let env = BTreeMap::from([("ANTHROPIC_API_KEY".into(), "sk-live-7".into())]);
        let res = Path::new("/run/opentrapp/perimeter");
        let secrets =
            resolve_secret_env("vault-proxy", &spec.services["vault-proxy"], &ctx_with(&env, &res))
                .unwrap();
        assert!(secrets.iter().any(|(k, v)| k == "ANTHROPIC_API_KEY" && v == "sk-live-7"));
        assert!(secrets.iter().any(|(k, v)| k == "ANTHROPIC_API_VERSION" && v == "2023-06-01"));
        assert!(!secrets.iter().any(|(k, _)| k == "OPENAI_API_KEY"), "optional+absent omitted");
    }

    #[test]
    fn resolve_secret_env_errors_on_missing_required() {
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::new(); // ANTHROPIC_API_KEY absent
        let res = Path::new("/run/opentrapp/perimeter");
        let err =
            resolve_secret_env("vault-proxy", &spec.services["vault-proxy"], &ctx_with(&env, &res))
                .unwrap_err();
        assert!(format!("{err}").contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn no_secret_value_appears_on_the_command_line() {
        // The core #75 guarantee, swept across every service that carries a secret.
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::from([
            ("ANTHROPIC_API_KEY".into(), "sk-ant-SUPERSECRET-xyz".into()),
            ("TELEGRAM_BOT_TOKEN".into(), "12345:BOT-SUPERSECRET".into()),
        ]);
        let res = Path::new("/run/opentrapp/perimeter");
        for svc_name in ["vault-proxy", "vault-agent"] {
            let joined = container_run_args(
                svc_name,
                &spec.services[svc_name],
                "img@sha256:x",
                &ctx_with(&env, &res),
            )
            .unwrap()
            .join(" ");
            assert!(
                !joined.contains("SUPERSECRET"),
                "{svc_name}: a secret value leaked onto the argv:\n{joined}"
            );
        }
    }

    #[test]
    fn apply_secret_env_injects_into_child_process_environment() {
        // The other half of #75: the value reaches the child via its ENVIRONMENT
        // (so `-e NAME` passthrough delivers it), never the argv. Mirrors the
        // `system_command_strips_appimage_lib_env` child-process probe.
        let mut cmd = system_command("sh");
        cmd.args(["-c", "printf %s \"${SEKRET-UNSET}\""]);
        apply_secret_env(&mut cmd, &[("SEKRET".to_string(), "hunter2".to_string())]);
        let out = cmd.output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout), "hunter2");
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
    /// a flag podman rejects). Uses vault-skills — no creds, no dependencies.
    /// Requires `ghcr.io/albertdobmeyer/opentrapp/vault-skills:latest` present.
    /// Run with: `cargo test --lib -- --ignored live_forge_brings_up`.
    #[test]
    #[ignore]
    fn live_forge_brings_up_and_tears_down() {
        let spec = perimeter::load().unwrap();
        let svc = &spec.services["vault-skills"];
        let env = BTreeMap::new();
        let res = std::env::temp_dir().join("opentrapp-live-test");
        let ctx = ctx_with(&env, &res);
        let image = "ghcr.io/albertdobmeyer/opentrapp/vault-skills:latest";

        // Clean slate.
        let _ = podman(&["rm".into(), "--force".into(), "--ignore".into(), "vault-skills".into()], Duration::from_secs(20));
        let _ = podman(&["network".into(), "rm".into(), "--force".into(), net_name("skills-net")], Duration::from_secs(20));

        // Real network create + real run args.
        assert!(ok(&podman(&network_create_args("skills-net", true, None), Duration::from_secs(20)).unwrap()));
        let args = container_run_args("vault-skills", svc, image, &ctx).unwrap();
        let run = podman(&args, Duration::from_secs(60)).unwrap();
        assert!(ok(&run), "podman run rejected the generated args");

        // It must be running, and carry our label.
        assert!(is_running("vault-skills"), "vault-skills not running after up");
        let labeled = podman(
            &["ps".into(), "--filter".into(), "label=io.opentrapp.service=vault-skills".into(), "--format".into(), "{{.Names}}".into()],
            Duration::from_secs(10),
        ).unwrap();
        assert!(String::from_utf8_lossy(&labeled.stdout).contains("vault-skills"));

        // Tear down.
        assert!(ok(&podman(&["rm".into(), "--force".into(), "vault-skills".into()], Duration::from_secs(20)).unwrap()));
        let _ = podman(&["network".into(), "rm".into(), "--force".into(), net_name("skills-net")], Duration::from_secs(20));
        assert!(!is_running("vault-skills"), "vault-skills still running after down");
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
          "images": { "ghcr.io/albertdobmeyer/opentrapp/vault-skills": { "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000", "source": "built", "tar": "vault-skills.tar" } }
        }"#;
        let verifier = BundleVerifier {
            overlay: ImageDigestOverlay::parse(overlay_json).unwrap(),
            images_dir: std::env::temp_dir().join("opentrapp-no-tars"),
        };
        let img = ImageRef {
            source: ImageSource::Built,
            repo: Some("ghcr.io/albertdobmeyer/opentrapp/vault-skills".into()),
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
        // WS-C: the vault-proxy.py addon mount is gone (the goproxy compiles the
        // policy in); allowlist.txt is still a verified resource mount, resolved
        // from the resource dir — never a source-tree bind.
        assert!(joined.contains("/run/opentrapp/perimeter/allowlist.txt:/opt/vault/allowlist.txt:ro"));
        assert!(!joined.contains("workloads/agent") && !joined.contains("infra/proxy"), "no source-tree paths");
    }

    #[test]
    fn shields_mount_the_shared_sentinel_lib_read_only(){
        // The Sentinel lib is staged into the verified resource dir and bind-mounted
        // :ro into both shields at /opt/sentinel (spec 08 §5). Mirrors the proxy.py
        // resource-mount pattern — never a source-tree path.
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::new();
        let res = Path::new("/run/opentrapp/perimeter");
        for svc in ["vault-skills", "vault-social"] {
            let args =
                container_run_args(svc, &spec.services[svc], "img:latest", &ctx_with(&env, &res))
                    .unwrap();
            let joined = args.join(" ");
            assert!(
                joined.contains("/run/opentrapp/perimeter/sentinel:/opt/sentinel:ro"),
                "{svc} must mount the verified sentinel lib read-only, got: {joined}"
            );
        }
    }

    #[test]
    fn stage_resources_from_bundle_recurses_into_subdirs() {
        // The Sentinel lib is a directory tree (sentinel/lib/*, sentinel/corpus/*),
        // so staging must recurse — not just copy top-level files.
        let base = std::env::temp_dir().join("opentrapp-stage-recurse-test");
        let _ = std::fs::remove_dir_all(&base);
        let bundle = base.join("bundle");
        let runtime = base.join("runtime");
        std::fs::create_dir_all(bundle.join("sentinel/lib")).unwrap();
        std::fs::write(bundle.join("allowlist.txt"), "example.com\n").unwrap();
        std::fs::write(bundle.join("sentinel/judge.sh"), "#!/usr/bin/env bash\n").unwrap();
        std::fs::write(bundle.join("sentinel/lib/embed.py"), "x = 1\n").unwrap();

        stage_resources_from_bundle(&bundle, &runtime).unwrap();

        assert!(runtime.join("allowlist.txt").is_file(), "top-level file staged");
        assert!(runtime.join("sentinel/judge.sh").is_file(), "nested file staged");
        assert!(runtime.join("sentinel/lib/embed.py").is_file(), "deeply-nested file staged");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Zone 3 / B-bug: the `vault-proxy-logs` named volume must be chowned on
    /// mount to the mitmproxy uid; otherwise the non-root mitmproxy process
    /// in the container can't write `requests.jsonl` and the addon silently
    /// falls back to in-container `/tmp` (logs never reach the host). Podman's
    /// `:U` suffix is the documented mechanism for this. Test that the
    /// orchestrator emits it for that mount and ONLY that mount (read-only
    /// mounts like `proxy-ca` must not be chowned).
    #[test]
    fn vault_proxy_logs_mount_is_chown_on_mount() {
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::from([("ANTHROPIC_API_KEY".into(), "sk".into())]);
        let res = Path::new("/run/opentrapp/perimeter");
        let args =
            container_run_args("vault-proxy", &spec.services["vault-proxy"], "mitm@sha256:x", &ctx_with(&env, &res))
                .unwrap();

        // Find every `-v` arg and check the logs mount has :U.
        let mut found_logs_mount_with_chown = false;
        let mut found_proxy_ca_without_chown = false;
        let mut prev_was_v = false;
        for a in &args {
            if prev_was_v {
                if a.contains(":/var/log/vault-proxy") {
                    assert!(
                        a.ends_with(":U") || a.contains(":U,") || a.contains(":U:"),
                        "vault-proxy-logs mount must use podman ':U' chown-on-mount, got: {a}"
                    );
                    found_logs_mount_with_chown = true;
                }
                // proxy-ca is read-only; it must NOT carry :U (chown a ro mount is meaningless).
                if a.contains(":/home/mitmproxy/.mitmproxy") {
                    assert!(
                        !a.contains(":U"),
                        "proxy-ca is read-only; must not carry :U, got: {a}"
                    );
                    found_proxy_ca_without_chown = true;
                }
            }
            prev_was_v = a == "-v";
        }
        assert!(found_logs_mount_with_chown, "vault-proxy-logs mount not found in args");
        assert!(found_proxy_ca_without_chown, "proxy-ca mount not found in args");
    }

    #[test]
    fn vault_proxy_uses_the_image_entrypoint_no_override() {
        // WS-C (ADR-0026): the Zone-3 chown moved IN-IMAGE — the goproxy image's
        // own entrypoint chowns the mounted log + CA volumes then su-exec-drops to
        // the mitmproxy user. So perimeter.yml no longer overrides --entrypoint;
        // the run args must NOT carry an entrypoint override (the image provides it).
        let spec = perimeter::load().unwrap();
        let env = BTreeMap::from([("ANTHROPIC_API_KEY".into(), "sk".into())]);
        let res = Path::new("/run/opentrapp/perimeter");
        let args = container_run_args(
            "vault-proxy",
            &spec.services["vault-proxy"],
            "ghcr.io/x/vault-proxy@sha256:x",
            &ctx_with(&env, &res),
        )
        .unwrap();
        assert!(
            !args.iter().any(|a| a == "--entrypoint"),
            "vault-proxy must NOT override the entrypoint — the goproxy image provides the Zone-3 chown shim"
        );
    }

    #[test]
    fn idle_signal_ignores_telegram_keepalive_polls() {
        let now = 1_000_000u64;

        // Only keep-alive polls (getUpdates/getMe) → no real activity → None.
        // The whole bug: this must NOT report the agent as recently active.
        let polls = "{\"url\":\"https://api.telegram.org/botX/getUpdates\",\"ts_ms\":999000}\n\
                     {\"url\":\"https://api.telegram.org/botX/getMe\",\"ts_ms\":999500}";
        assert_eq!(last_activity_ms_from_log(polls, now), None);

        // A real Anthropic call at ts=700000, then NEWER keep-alive polls → idle
        // is measured from the Anthropic call (300_000ms ago), not the polls.
        let mixed = "{\"url\":\"https://api.anthropic.com/v1/messages\",\"ts_ms\":700000}\n\
                     {\"url\":\"https://api.telegram.org/botX/getUpdates\",\"ts_ms\":999000}\n\
                     {\"url\":\"https://api.telegram.org/botX/getUpdates\",\"ts_ms\":999900}";
        assert_eq!(last_activity_ms_from_log(mixed, now), Some(300_000));

        // A sendMessage reply (responding to a user) IS real activity.
        let reply = "{\"url\":\"https://api.telegram.org/botX/sendMessage\",\"ts_ms\":950000}\n\
                     {\"url\":\"https://api.telegram.org/botX/getUpdates\",\"ts_ms\":999000}";
        assert_eq!(last_activity_ms_from_log(reply, now), Some(50_000));

        // Empty / unparseable → None (fail-safe: no signal).
        assert_eq!(last_activity_ms_from_log("", now), None);
        assert_eq!(last_activity_ms_from_log("not json\n{bad", now), None);
    }

    // ========================================================================
    // SECURITY PIN SUITE — perimeter resource provisioning (ADR-0019/0022).
    //
    // These pins lock one contract: the perimeter's mounted security policy is
    // ALWAYS fully provisioned into the runtime resource dir, byte-identical to
    // its canonical source, with exec bits intact. They are GREEN today against
    // `stage_resources_from_bundle` (the recursive-copy primitive, which SURVIVES
    // the de-Tauri cutover). At the cutover the `provision_into` SEAM below swaps
    // from "copy the Tauri-bundled dir" to "extract the bytes embedded in the
    // signed binary"; THE #[test] ASSERTION BODIES MUST NOT CHANGE. A red pin =
    // a dropped policy file, a lost exec bit, or content drift — i.e. a silently
    // broken boundary. They fail LOUD and never skip (a skipped security pin is
    // exactly the gloss this suite exists to prevent).
    // ========================================================================

    struct ResourceEntry {
        src: PathBuf,
        dest: String,
        exec: bool,
    }

    /// Ascend from this crate to the repo root (the dir holding `compose.yml` +
    /// `workloads/`). Panics — never skips — if not found: these pins must RUN.
    fn pin_repo_root() -> PathBuf {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        loop {
            if dir.join("compose.yml").is_file() && dir.join("workloads").is_dir() {
                return dir;
            }
            if !dir.pop() {
                panic!(
                    "security pin: repo root (compose.yml + workloads/) not found from \
                     CARGO_MANIFEST_DIR — run the pins from the workspace checkout"
                );
            }
        }
    }

    #[cfg(unix)]
    fn pin_is_executable(p: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    fn pin_is_executable(_p: &Path) -> bool {
        false
    }

    fn pin_walk_files(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    out.extend(pin_walk_files(&p));
                } else if p.is_file() {
                    out.push(p);
                }
            }
        }
        out
    }

    /// The canonical-source → resource-dir-relative-dest mapping the perimeter
    /// depends on. SINGLE SOURCE OF TRUTH for both the synthetic bundle and the
    /// assertions. Mirrors build.rs STAGED_RESOURCES + stage_manifests +
    /// stage_sentinel. The sentinel tree is walked dynamically (excluding
    /// `*.test.sh`, runtime-only) so an added sentinel file can never silently
    /// escape provisioning.
    fn pin_expected_resources(root: &Path) -> Vec<ResourceEntry> {
        let mut v = vec![
            ResourceEntry {
                src: root.join("workloads/agent/config/vault-seccomp.json"),
                dest: "vault-seccomp.json".into(),
                exec: false,
            },
            ResourceEntry {
                src: root.join("workloads/agent/config/vault-proxy-seccomp.json"),
                dest: "vault-proxy-seccomp.json".into(),
                exec: false,
            },
            ResourceEntry {
                src: root.join("infra/proxy/allowlist.txt"),
                dest: "allowlist.txt".into(),
                exec: false,
            },
            ResourceEntry {
                src: root.join("infra/egress/resolv.conf"),
                dest: "resolv.conf".into(),
                exec: false,
            },
        ];
        for wl in ["agent", "skills", "social"] {
            v.push(ResourceEntry {
                src: root.join(format!("workloads/{wl}/component.yml")),
                dest: format!("manifests/{wl}/component.yml"),
                exec: false,
            });
        }
        let sentinel_root = root.join("sentinel");
        for src in pin_walk_files(&sentinel_root) {
            let name = src.file_name().unwrap().to_string_lossy().to_string();
            if name.ends_with(".test.sh") {
                continue;
            }
            let rel = src.strip_prefix(&sentinel_root).unwrap();
            let exec = pin_is_executable(&src);
            v.push(ResourceEntry {
                dest: format!("sentinel/{}", rel.to_string_lossy()),
                exec,
                src,
            });
        }
        v
    }

    /// A unique, clean scratch dir (no Date/random needed — keyed by the tag and
    /// the running thread, removed first so it is always fresh).
    fn pin_tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("{tag}-{:?}", std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// THE SWAP SEAM. Post de-Tauri cutover this provisions from the bytes
    /// embedded in the signed binary (`embedded_resources::extract_embedded_resources`)
    /// — no Tauri bundle dir, no build.rs staging. The #[test] assertions below
    /// are byte-for-byte unchanged from the pre-cutover bundle-copy version: the
    /// contract (every file present, byte-identical, exec bits intact, covers
    /// every perimeter.yml resource/seccomp mount) is identical, so the pins that
    /// were green before the swap stay green after it.
    fn pin_provision_into(dst: &Path) {
        crate::orchestrator::embedded_resources::extract_embedded_resources(dst).unwrap();
    }

    #[test]
    fn pin_every_perimeter_resource_mount_and_seccomp_is_provisioned() {
        // ANTI-DRIFT CROSS-CHECK: every `kind: resource` mount source and every
        // `seccomp:` profile that perimeter.yml references MUST be produced by
        // provisioning. If perimeter.yml grows a new resource mount, the embedded
        // set must include it or this fails — the perimeter can never mount a
        // policy file that provisioning failed to deliver.
        let spec = perimeter::load().unwrap();
        let dst = pin_tmp("opentrapp-pin-xref");
        pin_provision_into(&dst);

        let mut required: Vec<String> = Vec::new();
        for svc in spec.services.values() {
            if let Some(sc) = &svc.seccomp {
                required.push(sc.clone());
            }
            for vol in &svc.volumes {
                if matches!(vol.kind, perimeter::MountKind::Resource) {
                    required.push(vol.source.clone());
                }
            }
        }
        required.sort();
        required.dedup();
        assert!(
            !required.is_empty(),
            "perimeter.yml must declare resource/seccomp mounts"
        );
        for name in &required {
            let p = dst.join(name);
            assert!(
                p.exists(),
                "perimeter.yml mounts `{name}` (kind:resource / seccomp) but \
                 provisioning produced no {} — the boundary would fail to start",
                p.display()
            );
        }
        let _ = std::fs::remove_dir_all(&dst);
    }

    #[test]
    fn pin_provisioned_resources_are_byte_identical_to_canonical_source() {
        let root = pin_repo_root();
        let dst = pin_tmp("opentrapp-pin-bytes");
        pin_provision_into(&dst);
        let entries = pin_expected_resources(&root);
        assert!(entries.len() >= 13, "expected the full policy set, got {}", entries.len());
        for r in entries {
            let got = std::fs::read(dst.join(&r.dest))
                .unwrap_or_else(|e| panic!("not provisioned: {} ({e})", r.dest));
            let want = std::fs::read(&r.src).unwrap();
            assert_eq!(
                got, want,
                "provisioned `{}` differs from canonical source {} — policy content drift",
                r.dest,
                r.src.display()
            );
        }
        let _ = std::fs::remove_dir_all(&dst);
    }

    #[test]
    #[cfg(unix)]
    fn pin_provisioned_scripts_keep_their_exec_bit() {
        let root = pin_repo_root();
        let dst = pin_tmp("opentrapp-pin-exec");
        pin_provision_into(&dst);
        let mut checked = 0;
        for r in pin_expected_resources(&root).into_iter().filter(|r| r.exec) {
            assert!(
                pin_is_executable(&dst.join(&r.dest)),
                "provisioned `{}` lost its exec bit — the in-container shield cannot run it",
                r.dest
            );
            checked += 1;
        }
        assert!(
            checked >= 4,
            "expected the sentinel shield scripts (judge/embed/config/egress-advisor) \
             to be exec-checked, got {checked}"
        );
        let _ = std::fs::remove_dir_all(&dst);
    }

    #[test]
    fn pin_reprovisioning_overwrites_tampering() {
        // The runtime resource dir is re-provisioned on every launch so a tampered
        // policy file is overwritten from the signed source (the documented
        // stage_resources_from_bundle contract). A compromised host write must not
        // survive a perimeter restart.
        let root = pin_repo_root();
        let dst = pin_tmp("opentrapp-pin-tamper");
        pin_provision_into(&dst);
        let victim = dst.join("allowlist.txt");
        std::fs::write(&victim, "evil.example.com\n").unwrap();
        pin_provision_into(&dst); // re-provision overwrites
        let restored = std::fs::read(&victim).unwrap();
        let canonical = std::fs::read(root.join("infra/proxy/allowlist.txt")).unwrap();
        assert_eq!(
            restored, canonical,
            "re-provisioning must overwrite a tampered allowlist with the canonical source"
        );
        let _ = std::fs::remove_dir_all(&dst);
    }

    #[test]
    fn pin_seccomp_profiles_are_valid_json_with_a_default_action() {
        // The seccomp profiles are load-bearing syscall containment. Pin that they
        // parse as JSON and declare a defaultAction — an empty/garbage profile
        // would silently weaken the filter.
        let root = pin_repo_root();
        for name in ["vault-seccomp.json", "vault-proxy-seccomp.json"] {
            let path = root.join("workloads/agent/config").join(name);
            let txt = std::fs::read_to_string(&path).unwrap();
            let json: serde_json::Value = serde_json::from_str(&txt)
                .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
            assert!(
                json.get("defaultAction").and_then(|v| v.as_str()).is_some(),
                "{name} must declare a defaultAction (the syscall default posture)"
            );
        }
    }
}
