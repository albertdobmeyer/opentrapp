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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager as _};

use crate::lifecycle::{
    run_compose, BootstrapProgress, BootstrapStep, PerimeterStateStore,
};

const TOTAL_STEPS: u8 = 7;
const SHELL_SERVICES: [&str; 3] = ["vault-proxy", "vault-forge", "vault-pioneer"];

// ─── Public entry point ───────────────────────────────────────────────

/// Spawn the bootstrap pipeline on a tokio background task.
/// Replaces `bring_perimeter_up_async` from the v0.3 lifecycle.
pub fn spawn_bootstrap(handle: AppHandle, root: PathBuf) {
    tauri::async_runtime::spawn(async move {
        run_bootstrap(handle, root).await;
    });
}

async fn run_bootstrap(handle: AppHandle, root: PathBuf) {
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

    // Step 4: build images
    if let Err(cause) = step_build_images(&handle, &root, &runtime).await {
        set_failed(&handle, cause, "Image build failed.");
        return;
    }

    // Step 5: pull images
    if let Err(cause) = step_pull_images(&handle, &root, &runtime).await {
        set_failed(&handle, cause, "Image pull failed.");
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

    let vault_dir = root.join("components").join("opencli-container");
    let env_path = vault_dir.join(".env");

    if env_path.exists() {
        eprintln!("[bootstrap] .env already exists — skipping write-env");
        return Ok(());
    }

    let example_path = vault_dir.join(".env.example");
    if !example_path.exists() {
        eprintln!("[bootstrap] .env.example missing — skipping write-env (fresh clone?)");
        return Ok(());
    }

    std::fs::copy(&example_path, &env_path)
        .map(|_| ())
        .map_err(|e| {
            eprintln!("[bootstrap] write-env failed: {e}");
            "env-write-failed"
        })
}

async fn step_build_images(
    handle: &AppHandle,
    root: &Path,
    runtime: &str,
) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::BuildImages, 4, None, Some("Building images…".into()));

    // Check if images exist first. `compose images` lists images for the
    // compose project; if it returns non-empty output, they exist.
    let images_exist = images_already_built(root, runtime);
    if images_exist {
        eprintln!("[bootstrap] build images — already exist, skipping");
        return Ok(());
    }

    let ok = tokio::task::spawn_blocking({
        let root = root.to_path_buf();
        let runtime = runtime.to_string();
        move || {
            // Build vault-agent, vault-forge, vault-pioneer (not vault-proxy — it's an image pull).
            run_compose_with_runtime(
                &root,
                &runtime,
                &["compose", "build", "vault-agent", "vault-forge", "vault-pioneer"],
                Duration::from_secs(600),
            )
        }
    })
    .await
    .unwrap_or(false);

    if ok {
        Ok(())
    } else {
        Err("image-build-failed")
    }
}

async fn step_pull_images(
    handle: &AppHandle,
    root: &Path,
    runtime: &str,
) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::PullImages, 5, None, Some("Pulling mitmproxy…".into()));

    let ok = tokio::task::spawn_blocking({
        let root = root.to_path_buf();
        let runtime = runtime.to_string();
        move || {
            run_compose_with_runtime(
                &root,
                &runtime,
                &["compose", "pull", "vault-proxy"],
                Duration::from_secs(300),
            )
        }
    })
    .await
    .unwrap_or(false);

    if ok {
        Ok(())
    } else {
        Err("image-pull-failed")
    }
}

fn step_up_shell(handle: &AppHandle, root: &Path) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::UpShell, 6, None, None);

    let ok = run_compose(root, &["up", "-d", "vault-proxy", "vault-forge", "vault-pioneer"], Duration::from_secs(90));
    if ok { Ok(()) } else { Err("shell-up-failed") }
}

fn step_verify_shell(handle: &AppHandle, runtime: &str) -> Result<(), &'static str> {
    set_step(handle, BootstrapStep::VerifyShell, 7, None, None);

    // Check all 3 shell services are running via label filter.
    for service in &SHELL_SERVICES {
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

    eprintln!("[bootstrap] shell verified — all 3 services running");
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn images_already_built(root: &Path, runtime: &str) -> bool {
    // Derive the compose project name from the directory name (lowercase, hyphens preserved).
    let project = root
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "opentrapp".to_string());
    let image_name = format!("localhost/{project}_vault-agent:latest");
    // `podman image exists` exits 0 if the image is present — no stdout needed.
    // podman-compose 1.0.6 doesn't support `compose images`, so we bypass it.
    StdCommand::new(runtime)
        .args(["image", "exists", &image_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_compose_with_runtime(root: &Path, runtime: &str, args: &[&str], timeout: Duration) -> bool {
    let secs = timeout.as_secs().max(1).to_string();
    let result = StdCommand::new("timeout")
        .args(["--signal=TERM", "--kill-after=5s", &secs])
        .arg(runtime)
        .args(args)
        .current_dir(root)
        .output();

    // If timeout not found, run directly.
    let output = match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            StdCommand::new(runtime).args(args).current_dir(root).output()
        }
        other => other,
    };

    match output {
        Ok(o) if o.status.success() => true,
        Ok(o) => {
            eprintln!(
                "[bootstrap] {} {} → exit {}: {}",
                runtime,
                args.join(" "),
                o.status,
                String::from_utf8_lossy(&o.stderr).trim()
            );
            false
        }
        Err(e) => {
            eprintln!("[bootstrap] spawn error: {e}");
            false
        }
    }
}

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
