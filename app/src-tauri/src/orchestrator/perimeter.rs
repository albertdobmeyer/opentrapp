//! Perimeter spec — the runtime topology contract.
//!
//! `perimeter.yml` (under `resources/`) is embedded into the binary at compile
//! time via [`include_str!`], so it is covered by the AppImage signature: the
//! perimeter topology cannot be altered without a rebuild + re-sign. This is a
//! deliberate security posture for a containment wrapper — perimeter changes
//! are code-review-gated build events, never runtime-swappable.
//!
//! The orchestrator (`podman.rs`) translates this spec into `podman run`
//! invocations. Image digests + cosign identities live in a separate,
//! CI-generated `image-digests.json` overlay (verified before run) — not here.
//! See ADR-0009 for the topology rationale.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The embedded spec source. Single source of truth for the orchestrator.
const PERIMETER_YML: &str = include_str!("../../resources/perimeter.yml");

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PerimeterSpec {
    pub version: u32,
    pub services: BTreeMap<String, Service>,
    pub networks: BTreeMap<String, Network>,
    #[serde(default)]
    pub volumes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Service {
    pub image: ImageRef,
    #[serde(default)]
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub read_only: bool,
    /// Default true: containment wants `--security-opt no-new-privileges`.
    #[serde(default = "default_true")]
    pub no_new_privileges: bool,
    /// Default true: containment wants `--cap-drop ALL`.
    #[serde(default = "default_true")]
    pub cap_drop_all: bool,
    /// Resource-relative seccomp profile filename (e.g. `vault-seccomp.json`),
    /// resolved by the orchestrator to a verified runtime path — never an
    /// absolute dev path (unlike compose.yml today).
    #[serde(default)]
    pub seccomp: Option<String>,
    #[serde(default)]
    pub cap_add: Vec<String>,
    #[serde(default)]
    pub pids_limit: Option<u32>,
    #[serde(default)]
    pub mem_limit: Option<String>,
    #[serde(default)]
    pub cpus: Option<f64>,
    #[serde(default)]
    pub networks: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<Dependency>,
    #[serde(default)]
    pub tmpfs: Vec<TmpfsMount>,
    #[serde(default)]
    pub volumes: Vec<VolumeMount>,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    #[serde(default)]
    pub sysctls: Vec<Sysctl>,
    #[serde(default)]
    pub healthcheck: Option<Healthcheck>,
    #[serde(default)]
    pub stdin_open: bool,
    #[serde(default)]
    pub tty: bool,
    #[serde(default = "default_restart")]
    pub restart: String,
    /// Default false. When true, the orchestrator does NOT start this service at
    /// boot (`boot_services` / `up` / `shell_up` skip it); it is started on
    /// demand by the command layer and stopped after an idle grace. Only the
    /// task-runner shields (vault-skills / vault-social) opt in — they are not
    /// daemons, just idle until a command needs them, so booting them wastes a
    /// container in the resting perimeter.
    #[serde(default)]
    pub on_demand: bool,
}

/// How an image is sourced — drives the verification strategy before run.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ImageSource {
    /// Built by our CI; verified by cosign identity + digest from the overlay.
    Built,
    /// Third-party registry image; verified by pinned digest only (no OUR sig).
    External,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ImageRef {
    pub source: ImageSource,
    /// For `built`: repo without tag (e.g. `ghcr.io/.../vault-agent`); the tag
    /// + digest are applied from the release overlay.
    #[serde(default)]
    pub repo: Option<String>,
    /// For `external`: a fully-pinned ref including `@sha256:…`.
    #[serde(default)]
    pub r#ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Dependency {
    pub service: String,
    pub condition: DependencyCondition,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyCondition {
    Started,
    Healthy,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TmpfsMount {
    pub path: String,
    #[serde(default)]
    pub options: Option<String>,
}

/// A bind/volume mount. `kind` decides where `source` resolves:
/// - `Named`: a podman named volume (auto-created).
/// - `Resource`: a verified file extracted from the signed bundle (policy
///   files: seccomp, proxy addon, allowlist, resolv.conf). Never user-writable.
///
/// `chown` (podman `:U` suffix) — chown the volume to the container's user
/// namespace mapping at mount time. Required when the container's process
/// runs as a non-root user and needs to write to a named volume (rootless
/// podman creates named volumes owned by container-root by default, so the
/// non-root process gets EACCES otherwise). Only applies when the mount is
/// writable; setting it on a `read_only` mount is rejected.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub chown: bool,
    pub kind: MountKind,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MountKind {
    Named,
    Resource,
}

/// Environment entry. Secrets are referenced by variable name and resolved
/// from the user's runtime `.env` at launch — never inlined in the spec.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EnvVar {
    pub name: String,
    pub kind: EnvKind,
    /// For `literal`.
    #[serde(default)]
    pub value: Option<String>,
    /// For `secret`: the runtime variable name to look up.
    #[serde(default)]
    pub var: Option<String>,
    /// For `secret`: tolerate absence (e.g. OPENAI_API_KEY).
    #[serde(default)]
    pub optional: bool,
    /// For `secret`: fallback if the variable is unset.
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EnvKind {
    Literal,
    Secret,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Sysctl {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Healthcheck {
    pub test: String,
    #[serde(default)]
    pub interval: Option<String>,
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub retries: Option<u32>,
    #[serde(default)]
    pub start_period: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Network {
    #[serde(default)]
    pub internal: bool,
    #[serde(default)]
    pub subnet: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_restart() -> String {
    "unless-stopped".to_string()
}

/// Parse the embedded perimeter spec. Infallible at runtime in practice because
/// the spec is compile-time data; a parse failure here is a build-time bug
/// caught by [`tests::spec_parses`].
pub fn load() -> Result<PerimeterSpec, serde_yaml::Error> {
    serde_yaml::from_str(PERIMETER_YML)
}

impl PerimeterSpec {
    /// Service names in dependency-respecting start order (egress → proxy →
    /// agent/forge/social). Topological sort over `depends_on`.
    pub fn start_order(&self) -> Vec<String> {
        let mut ordered: Vec<String> = Vec::new();
        let mut remaining: Vec<String> = self.services.keys().cloned().collect();
        // Deterministic: BTreeMap keys are sorted; repeatedly place any service
        // whose deps are all already placed.
        while !remaining.is_empty() {
            let next: Vec<String> = remaining
                .iter()
                .filter(|name| {
                    self.services[*name]
                        .depends_on
                        .iter()
                        .all(|d| ordered.contains(&d.service))
                })
                .cloned()
                .collect();
            if next.is_empty() {
                // Cycle or dangling dep — return what we have plus the rest, so
                // the orchestrator surfaces a clear error rather than hanging.
                ordered.extend(remaining.drain(..));
                break;
            }
            for name in next {
                ordered.push(name.clone());
                remaining.retain(|n| n != &name);
            }
        }
        ordered
    }

    /// Services to start at boot, in dependency order: `start_order()` minus the
    /// `on_demand` services. `up`/`shell_up` iterate this so on-demand shields
    /// stay absent from the resting perimeter and are started lazily by the
    /// command layer instead.
    pub fn boot_services(&self) -> Vec<String> {
        self.start_order()
            .into_iter()
            .filter(|name| !self.services[name].on_demand)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_parses() {
        let spec = load().expect("embedded perimeter.yml must parse");
        assert_eq!(spec.version, 1);
        assert_eq!(spec.services.len(), 5, "five-container perimeter (ADR-0009)");
        for svc in ["vault-agent", "vault-proxy", "vault-egress", "vault-skills", "vault-social"] {
            assert!(spec.services.contains_key(svc), "missing service {svc}");
        }
    }

    #[test]
    fn agent_is_maximally_contained() {
        let spec = load().unwrap();
        let agent = &spec.services["vault-agent"];
        assert!(agent.read_only, "agent must be read_only");
        assert!(agent.cap_drop_all, "agent must drop all caps");
        assert!(agent.no_new_privileges);
        assert!(agent.cap_add.is_empty(), "agent must add NO caps");
        assert_eq!(agent.networks, vec!["agent-net"], "agent only on agent-net");
        assert_eq!(agent.seccomp.as_deref(), Some("vault-seccomp.json"));
    }

    #[test]
    fn shields_mount_shared_sentinel_lib_as_verified_readonly_resource() {
        // The shared Sentinel lib is staged like every other policy artifact
        // (kind: resource, :ro) and mounted at /opt/sentinel in both shields so
        // the in-container bash shields resolve it in a packaged build (spec 08 §5).
        let spec = load().unwrap();
        for svc in ["vault-skills", "vault-social"] {
            let mount = spec.services[svc]
                .volumes
                .iter()
                .find(|v| v.target == "/opt/sentinel")
                .unwrap_or_else(|| panic!("{svc} must mount /opt/sentinel"));
            assert_eq!(mount.source, "sentinel", "{svc} sentinel source");
            assert_eq!(mount.kind, MountKind::Resource, "{svc} sentinel must be a verified resource");
            assert!(mount.read_only, "{svc} sentinel mount must be read-only");
            assert!(!mount.chown, "{svc} sentinel mount must not chown a :ro mount");
        }
    }

    #[test]
    fn only_egress_holds_net_admin() {
        let spec = load().unwrap();
        for (name, svc) in &spec.services {
            let has_net_admin = svc.cap_add.iter().any(|c| c == "NET_ADMIN");
            if name == "vault-egress" {
                assert!(has_net_admin, "egress must hold NET_ADMIN");
            } else {
                assert!(!has_net_admin, "{name} must NOT hold NET_ADMIN (ADR-0009)");
            }
        }
    }

    #[test]
    fn no_secret_values_are_inlined() {
        let spec = load().unwrap();
        for (name, svc) in &spec.services {
            for e in &svc.env {
                if matches!(e.kind, EnvKind::Secret) {
                    assert!(e.value.is_none(), "{name}/{} secret must not inline a value", e.name);
                    assert!(e.var.is_some(), "{name}/{} secret must name a var", e.name);
                }
            }
        }
    }

    #[test]
    fn start_order_respects_dependencies() {
        let spec = load().unwrap();
        let order = spec.start_order();
        let pos = |s: &str| order.iter().position(|n| n == s).unwrap();
        assert!(pos("vault-egress") < pos("vault-proxy"), "egress before proxy");
        assert!(pos("vault-proxy") < pos("vault-agent"), "proxy before agent");
        assert_eq!(order.len(), 5);
    }

    #[test]
    fn shields_are_on_demand_others_are_not() {
        let spec = load().unwrap();
        for svc in ["vault-skills", "vault-social"] {
            assert!(spec.services[svc].on_demand, "{svc} must be on_demand");
        }
        for svc in ["vault-agent", "vault-proxy", "vault-egress"] {
            assert!(!spec.services[svc].on_demand, "{svc} must NOT be on_demand");
        }
    }

    #[test]
    fn boot_services_excludes_on_demand_shields() {
        let spec = load().unwrap();
        let boot = spec.boot_services();
        assert_eq!(
            boot,
            vec!["vault-egress", "vault-proxy", "vault-agent"],
            "boot set is the three always-on services in dependency order"
        );
        assert!(!boot.contains(&"vault-skills".to_string()));
        assert!(!boot.contains(&"vault-social".to_string()));
    }

    #[test]
    fn proxy_is_external_pinned_by_digest() {
        let spec = load().unwrap();
        let proxy = &spec.services["vault-proxy"];
        assert_eq!(proxy.image.source, ImageSource::External);
        let r = proxy.image.r#ref.as_deref().unwrap();
        assert!(r.contains("@sha256:"), "external image must be digest-pinned");
    }
}
