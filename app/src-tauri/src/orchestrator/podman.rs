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

/// Run `podman <args>` with a timeout wrapper (falls back to a direct call if
/// `timeout(1)` is absent). Stderr is redacted before logging.
fn podman(args: &[String], timeout: Duration) -> Result<Output, OrchestratorError> {
    let secs = timeout.as_secs().max(1).to_string();
    let wrapped = StdCommand::new("timeout")
        .args(["--signal=TERM", "--kill-after=5s", &secs, "podman"])
        .args(args)
        .output();
    let out = match wrapped {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            StdCommand::new("podman").args(args).output()
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
/// DEV-ONLY: performs no signature/digest verification. The production verifier
/// (step 5) replaces this with cosign + a signed digest overlay. Loud by design.
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

/// The single dev verifier instance (referenced by `'static` from contexts).
static DEV_VERIFIER: DevVerifier = DevVerifier;

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
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: &DEV_VERIFIER };
    up(&spec, &ctx)
}

/// Bring up the security shell only (everything except the agent tenant).
pub fn shell_up(data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    let env = load_runtime_env(data_dir);
    let rd = resource_dir();
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: &DEV_VERIFIER };
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
    let ctx = RunContext { resource_dir: &rd, env: &env, verifier: &DEV_VERIFIER };
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

/// Verify (and in production, acquire) every image the perimeter needs. In dev
/// this resolves refs and checks local presence, logging anything missing.
pub fn ensure_images(_data_dir: &Path) -> Result<(), OrchestratorError> {
    let spec = load_spec()?;
    for (name, svc) in &spec.services {
        let resolved = DEV_VERIFIER.verify_and_resolve(&svc.image)?;
        let exists = podman(
            &["image".into(), "exists".into(), resolved.clone()],
            Duration::from_secs(10),
        )?;
        if !ok(&exists) {
            eprintln!("[orchestrator] image for {name} not present locally: {resolved}");
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
