//! Bootstrap service — idempotent 7-step pipeline.
//!
//! Runs on every app launch. Each step checks whether it's already done
//! before acting; subsequent launches finish in <1s when the system is
//! already set up. First launch is dominated by image build + pull (~5-8 min).
//!
//! Replaces the ad-hoc wizard-driven `bring_perimeter_up_async`. The new
//! service runs unconditionally; whether to bring vault-agent up afterwards
//! is decided by `auto_activate::after_shell_ready` based on marker files.

pub mod auto_activate;
pub mod migrate_from_lobster_trapp;

use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager as _};

use crate::lifecycle::{
    BootstrapProgress, BootstrapStep, PerimeterStateStore,
};
use crate::orchestrator::podman;

const TOTAL_STEPS: u8 = 7;

/// The security shell to verify — everything except the agent tenant. The
/// containment core (`vault-egress` + `vault-proxy`, per ADR-0009) is always
/// present; the optional workload containers (`vault-skills`, `vault-social`)
/// are included only when this install's profile bundled them (modular
/// distribution — determined by which manifests `build.rs` staged). When the
/// manifests dir is absent (dev / unstaged), assume the full set so dev runs
/// are unaffected.
fn shell_services() -> Vec<&'static str> {
    let mut services = vec!["vault-egress", "vault-proxy"];
    let manifests = podman::resource_dir().join("manifests");
    let unstaged = !manifests.exists();
    if unstaged || manifests.join("skills").exists() {
        services.push("vault-skills");
    }
    if unstaged || manifests.join("social").exists() {
        services.push("vault-social");
    }
    services
}

// ─── Public entry point ───────────────────────────────────────────────

/// Guards against concurrent bootstrap runs. The app kicks off a bootstrap on
/// launch *and* the wizard / retry path can trigger one; without this guard two
/// runs race on `podman run`, colliding on container names
/// (`name "vault-skills" is already in use`). Single-flight: a second spawn while
/// one is in flight is ignored.
static BOOTSTRAP_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Spawn the bootstrap pipeline on a tokio background task.
/// Replaces `bring_perimeter_up_async` from the v0.3 lifecycle.
pub fn spawn_bootstrap(handle: AppHandle, root: PathBuf) {
    if BOOTSTRAP_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        eprintln!("[bootstrap] already in flight — ignoring duplicate spawn");
        return;
    }
    tauri::async_runtime::spawn(async move {
        run_bootstrap(handle, root).await;
        BOOTSTRAP_IN_FLIGHT.store(false, Ordering::SeqCst);
    });
}

/// On-launch entry point. Spawns the bootstrap *only* if the user has already
/// provided real credentials (`.env` exists with a non-placeholder
/// `ANTHROPIC_API_KEY`). On a fresh install with no `.env`, this returns
/// without firing the pipeline — the wizard's "Install" button calls
/// `retry_bootstrap` (which goes through `spawn_bootstrap` directly) once the
/// user saves their keys. Prevents the dead-end where bootstrap silently fails
/// at step 4 before the user has had a chance to configure anything (Zone 1).
pub fn spawn_bootstrap_on_launch(handle: AppHandle, root: PathBuf) {
    let env_path = root.join(".env");
    if !has_real_anthropic_key(&env_path) {
        eprintln!(
            "[bootstrap] no credentials yet at {} — deferring to wizard",
            env_path.display()
        );
        return;
    }
    spawn_bootstrap(handle, root);
}

/// True iff `.env` exists and `ANTHROPIC_API_KEY` is set to a non-placeholder
/// value. Mirrors `auto_activate::read_env_value`'s placeholder rule
/// (`REPLACE` substring, length ≥ 8) so the two stay in sync.
fn has_real_anthropic_key(env_path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(env_path) else {
        return false;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == "ANTHROPIC_API_KEY" {
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                if !v.is_empty() && !v.contains("REPLACE") && v.len() >= 8 {
                    return true;
                }
            }
        }
    }
    false
}

async fn run_bootstrap(handle: AppHandle, root: PathBuf) {
    // Step 0: stage verified resources + load signed images from the bundle.
    prepare_bundle(&handle);

    // Step 1: detect runtime
    let runtime = match step_detect_runtime(&handle, &root) {
        Ok(r) => r,
        Err(cause) => {
            set_failed(&handle, cause, "No container runtime found. Install Podman or Docker.");
            return;
        }
    };

    // Step 2: install runtime (only if step 1 failed — we never reach here
    // if step 1 succeeded, but the structure is here for the sidecar path).

    // Step 3: write .env
    if let Err(cause) = step_write_env(&handle, &root) {
        set_failed(&handle, cause, "Couldn't create the configuration file.");
        return;
    }

    // Step 4: prepare images (verify/acquire pre-built signed images)
    if let Err(cause) = step_prepare_images(&handle, &root).await {
        set_failed(&handle, cause, "Couldn't prepare the security images.");
        return;
    }

    // Step 5: verify images present
    if let Err(cause) = step_pull_images(&handle, &root).await {
        set_failed(&handle, cause, "Image verification failed.");
        return;
    }

    // Step 6: bring shell up
    if let Err(cause) = step_up_shell(&handle, &root) {
        set_failed(&handle, cause, "Couldn't start the security shell.");
        return;
    }

    // Step 7: verify shell
    if let Err(cause) = step_verify_shell(&handle, &runtime) {
        set_failed(&handle, cause, "Shell verification failed.");
        return;
    }

    // Pipeline complete — clear progress and dispatch to auto-activate.
    clear_progress(&handle);
    auto_activate::after_shell_ready(handle, root).await;
}

/// Stage verified policy files into the runtime resource dir and `podman load`
/// the signed image tarballs from the AppImage bundle. No-op in dev (no bundle).
/// The bundle copy is inside the read-only, signature-covered AppImage, so it
/// is the trusted source; restaging on every launch self-heals any tampering of
/// the writable runtime copies.
fn prepare_bundle(handle: &AppHandle) {
    let Ok(res) = handle.path().resource_dir() else {
        return;
    };
    let bundle_perimeter = res.join("perimeter");
    if !bundle_perimeter.exists() {
        // Dev run (cargo tauri dev): no bundled resources — DevVerifier path.
        return;
    }
    let runtime_rd = podman::resource_dir();
    if let Err(e) = podman::stage_resources_from_bundle(&bundle_perimeter, &runtime_rd) {
        eprintln!("[bootstrap] staging perimeter resources failed: {e}");
    }
    // Copy the small signed digest overlay into the runtime images dir. The
    // overlay is the trust anchor (pinned digests + release coordinates); the
    // large image tarballs are fetched from the release at step 4.
    let runtime_images = runtime_rd.join("images");
    let _ = std::fs::create_dir_all(&runtime_images);
    let _ = std::fs::copy(
        bundle_perimeter.join("images").join("image-digests.json"),
        runtime_images.join("image-digests.json"),
    );
}

// ─── Step implementations ─────────────────────────────────────────────

fn step_detect_runtime(handle: &AppHandle, _root: &Path) -> Result<String, &'static str> {
    set_step(handle, BootstrapStep::DetectRuntime, 1, None, None);

    for runtime in &["podman", "docker"] {
        if StdCommand::new(runtime)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            // Verify the runtime is actually operational (not just installed).
            let ok = StdCommand::new(runtime)
                .arg("ps")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if ok {
                eprintln!("[bootstrap] runtime: {runtime}");
                return Ok(runtime.to_string());
            }
        }
    }

    Err("no-container-runtime")
}

fn step_write_env(handle: &AppHandle, root: &Path) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::WriteEnv, 3, None, None);

    // `.env` lives in the runtime data dir (`root`), never the source tree.
    let env_path = root.join(".env");

    if env_path.exists() {
        eprintln!("[bootstrap] .env already exists — skipping write-env");
        return Ok(());
    }

    // The template ships in the verified resource bundle. If it isn't there
    // yet, the setup wizard will write `.env` directly via write_config.
    let example_path = podman::resource_dir().join(".env.example");
    if !example_path.exists() {
        eprintln!("[bootstrap] no .env yet — wizard will write it");
        return Ok(());
    }

    std::fs::copy(&example_path, &env_path)
        .map(|_| ())
        .map_err(|e| {
            eprintln!("[bootstrap] write-env failed: {e}");
            "env-write-failed"
        })
}

/// Prepare the perimeter images. Images are pre-built + signed by CI and
/// loaded from the verified bundle (no on-host build — that was the v0.4.1
/// failure). This step verifies/acquires them via the orchestrator.
async fn step_prepare_images(handle: &AppHandle, root: &Path) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::BuildImages, 4, None, Some("Downloading security images…".into()));
    // Fetch the signed image tarballs from the release (skipped in dev / when
    // already present), then verify + load each against the signed overlay.
    if let Err(e) = podman::fetch_perimeter_images().await {
        eprintln!("[bootstrap] image fetch failed: {e}");
        return Err("image-fetch-failed");
    }
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || podman::ensure_images(&root))
        .await
        .map_err(|_| "image-prepare-join-failed")?
        .map_err(|_| "image-prepare-failed")
}

/// Image acquisition is fully handled by step 4 (`ensure_images` covers both
/// our built images and the external mitmproxy image). This step remains as a
/// distinct, fast verification point so the 7-step progress UX is unchanged.
async fn step_pull_images(handle: &AppHandle, _root: &Path) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::PullImages, 5, None, Some("Verifying images…".into()));
    Ok(())
}

fn step_up_shell(handle: &AppHandle, root: &Path) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::UpShell, 6, None, None);
    podman::shell_up(root).map_err(|_| "shell-up-failed")
}

fn step_verify_shell(handle: &AppHandle, runtime: &str) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::VerifyShell, 7, None, None);

    // Check the profile's shell services are running via label filter.
    for service in &shell_services() {
        let running = StdCommand::new(runtime)
            .args([
                "ps",
                "--filter",
                &format!("label=com.docker.compose.service={service}"),
                "--filter",
                "status=running",
                "--format",
                "{{.Names}}",
            ])
            .output()
            .map(|o| o.status.success() && !o.stdout.trim_ascii().is_empty())
            .unwrap_or(false);

        if !running {
            eprintln!("[bootstrap] verify-shell: {service} not running");
            return Err("shell-verify-failed");
        }
    }

    eprintln!("[bootstrap] shell verified — all shell services running");
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn set_step(
    handle: &AppHandle,
    step: BootstrapStep,
    step_index: u8,
    percent: Option<u8>,
    detail: Option<String>,
) {
    eprintln!("[bootstrap] step {step_index}/{TOTAL_STEPS}: {}", step.as_str());
    let progress = BootstrapProgress::Step {
        step: step.clone(),
        step_index,
        total_steps: TOTAL_STEPS,
        percent,
        detail: detail.clone(),
        started_at_unix_ms: now_unix_ms(),
    };
    if let Some(store) = handle.try_state::<PerimeterStateStore>() {
        if let Ok(mut g) = store.bootstrap_progress.write() {
            *g = Some(progress.clone());
        }
    }
    let _ = handle.emit(
        "bootstrap-step-started",
        serde_json::json!({
            "step": step.as_str(),
            "total_steps": TOTAL_STEPS,
            "current": step_index,
            "detail": detail,
        }),
    );
}

fn set_failed(handle: &AppHandle, cause: &str, message: &str) {
    eprintln!("[bootstrap] failed: {cause} — {message}");
    let progress = BootstrapProgress::Failed {
        cause: cause.to_string(),
        message: message.to_string(),
        last_error: None,
    };
    if let Some(store) = handle.try_state::<PerimeterStateStore>() {
        if let Ok(mut g) = store.bootstrap_progress.write() {
            *g = Some(progress);
        }
    }
    let _ = handle.emit(
        "bootstrap-step-failed",
        serde_json::json!({ "cause": cause, "message": message }),
    );
}

fn clear_progress(handle: &AppHandle) {
    if let Some(store) = handle.try_state::<PerimeterStateStore>() {
        if let Ok(mut g) = store.bootstrap_progress.write() {
            *g = None;
        }
    }
}
