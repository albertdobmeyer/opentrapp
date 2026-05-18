mod bootstrap;
mod commands;
mod lifecycle;
mod orchestrator;
mod status_aggregator;
mod util;

// Public surface used exclusively by the `cargo-fuzz` harnesses at
// app/src-tauri/fuzz/. Enabled by the `fuzzing` cargo feature; absent
// from production builds. The two functions below mirror the parser and
// the argument interpolator that handle untrusted input — manifests
// loaded from third-party components, and user-supplied command
// arguments respectively — and are the highest-leverage targets for
// continuous fuzzing.
#[cfg(feature = "fuzzing")]
pub mod fuzz_api {
    use std::collections::HashMap;

    /// Parse a YAML byte slice as a `component.yml` manifest. Mirrors the
    /// production parser invoked by `orchestrator::discovery`.
    pub fn parse_manifest(input: &[u8]) -> Result<crate::orchestrator::manifest::Manifest, serde_yaml::Error> {
        serde_yaml::from_slice(input)
    }

    /// Interpolate user-supplied arguments into a manifest-declared command
    /// template. Mirrors the production path in `orchestrator::runner`.
    pub fn interpolate_args(command: &str, args: &HashMap<String, String>) -> String {
        crate::orchestrator::runner::interpolate_args_for_test(command, args)
    }

    /// Redact known token-bearing environment variables from a string. The
    /// production caller is `lifecycle::redact_secrets`, which is invoked on
    /// `podman compose` stderr before logging — failure modes worth surfacing
    /// are panics, infinite loops, or under-redaction (a real
    /// `TELEGRAM_BOT_TOKEN=…` substring escaping the redactor).
    pub fn redact_secrets(s: &str) -> String {
        crate::lifecycle::redact_secrets(s)
    }
}

use orchestrator::state::AppState;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

use lifecycle::{
    bring_perimeter_down_sync, clear_runguard,
    establish_runguard, install_signal_handlers, spawn_watchdog,
    PerimeterStateStore,
};
use status_aggregator::{spawn_status_evaluator, AssistantStatusStore};

/// How often the watchdog re-probes container state. 30s matches Pass 4's
/// "watchdog notices a dead container within 30s" target from the master
/// plan + Pass 2 spec.
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(30);

/// How often the status aggregator re-evaluates AssistantStatus + alerts.
/// 60s per Pass 7 Day 2 spec — twice the watchdog cadence so we always
/// see the freshest container state, with enough headroom for the
/// (cached) auth probe.
const STATUS_INTERVAL: Duration = Duration::from_secs(60);

/// Find the monorepo root by looking for a `components/` directory.
fn find_monorepo_root() -> PathBuf {
    // Strategy 1: Walk up from executable path
    // During cargo tauri dev: target/debug/opentrapp.exe
    //   -> 4 levels up to reach monorepo root (debug -> target -> src-tauri -> app -> root)
    if let Ok(exe) = std::env::current_exe() {
        let mut candidate = exe.as_path();
        for _ in 0..6 {
            if let Some(parent) = candidate.parent() {
                candidate = parent;
                if candidate.join("components").exists() {
                    return candidate.to_path_buf();
                }
            }
        }
    }

    // Strategy 2: Current working directory (common during dev)
    if let Ok(cwd) = std::env::current_dir() {
        // Check cwd itself and up to 3 parents
        let mut candidate = cwd.as_path().to_path_buf();
        for _ in 0..4 {
            if candidate.join("components").exists() {
                return candidate;
            }
            if let Some(parent) = candidate.parent() {
                candidate = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    // Strategy 3: Fallback to cwd
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Build the system tray menu and register event handlers.
///
/// - Status line (initial placeholder; live-updated by the watchdog tooltip)
/// - Open Dashboard → shows/focuses the main window
/// - Quit → exits the app cleanly (triggers RunEvent::Exit → compose down)
///
/// Left-click on the tray icon brings the main window forward.
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(
        app,
        "status",
        "Assistant — checking…",
        false,
        None::<&str>,
    )?;
    let open_item = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit OpenTrApp", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&status_item, &separator, &open_item, &separator, &quit_item],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip("OpenTrApp — checking…")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

// The Tauri desktop-app entry point. Gated out when the `fuzzing` feature
// is on so the fuzz harness can compile this lib without dragging in
// `tauri::generate_context!()` and the rest of the Tauri builder surface
// — the harness only needs `fuzz_api`, not the live app.
#[cfg(not(feature = "fuzzing"))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // One-shot migration from a prior Lobster-TrApp install. Runs before
    // anything reads marker files so PerimeterStateStore sees the new
    // ~/.opentrapp/ paths. Idempotent — writes a breadcrumb after success
    // and short-circuits on subsequent launches.
    bootstrap::migrate_from_lobster_trapp::migrate_if_legacy_install();

    let monorepo_root = find_monorepo_root();
    let app_state = AppState::new(monorepo_root.clone());

    // RunGuard: reap orphan containers from any prior SIGKILL'd session
    // BEFORE we bring the perimeter up. Reads/writes ~/.opentrapp/runguard.pid.
    establish_runguard(&monorepo_root);

    // Pass-4 lifecycle ownership (P11): the perimeter is bound to the app's
    // lifetime. App start → compose up. Graceful exit (window quit, tray Quit,
    // SIGTERM, SIGINT) → compose down. SIGKILL is reaped on next launch via
    // RunGuard above. Watchdog reports state every 30s; auto-restart of dead
    // containers is delegated to `restart: unless-stopped` in compose.yml.
    let perimeter_root_setup = monorepo_root.clone();
    let perimeter_root_exit = monorepo_root.clone();

    tauri::Builder::default()
        // Single-instance guard: second launch focuses the main window and exits.
        // Must be registered first per tauri-plugin-single-instance docs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(PerimeterStateStore::new())
        .manage(AssistantStatusStore::new())
        .manage(app_state)
        .setup(move |app| {
            setup_tray(app)?;
            // Install Unix signal handlers (SIGTERM, SIGINT → graceful exit).
            install_signal_handlers(app.handle().clone());
            // Spawn the perimeter-state watchdog.
            spawn_watchdog(app.handle().clone(), WATCHDOG_INTERVAL);
            // Spawn the status aggregator: combines perimeter health +
            // .env presence + Anthropic auth probe into AssistantStatus + alerts.
            spawn_status_evaluator(app.handle().clone(), STATUS_INTERVAL);
            // Spawn the bootstrap service. Idempotent 7-step pipeline that brings
            // the security shell up; then auto_activate decides whether to start
            // vault-agent based on activation + credentials markers.
            bootstrap::spawn_bootstrap(app.handle().clone(), perimeter_root_setup.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::manifest_cmds::list_components,
            commands::manifest_cmds::get_component,
            commands::manifest_cmds::set_monorepo_root,
            commands::execute::run_command,
            commands::execute::load_options,
            commands::stream::start_stream,
            commands::stream::stop_stream,
            commands::config::read_config,
            commands::config::write_config,
            commands::status::get_status,
            commands::health::run_health_probe,
            commands::lifecycle::get_perimeter_state,
            commands::lifecycle::restart_perimeter,
            commands::lifecycle::pause_perimeter,
            commands::lifecycle::resume_perimeter,
            commands::lifecycle::retry_bootstrap,
            status_aggregator::get_assistant_status,
            commands::prerequisites::check_prerequisites,
            commands::prerequisites::init_submodules,
            commands::prerequisites::create_config_from_template,
            commands::workflow_cmds::list_workflows,
            commands::workflow_cmds::execute_workflow,
            commands::diagnostics::generate_diagnostic_bundle,
            commands::telegram::derive_telegram_bot_url,
            commands::telegram::telegram_delete_webhook,
            commands::telegram::telegram_poll_for_start,
            commands::telegram::telegram_send_message,
            commands::telegram::telegram_advance_offset,
            commands::credentials::validate_anthropic_key,
            commands::credentials::commit_activation,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app_handle, event| {
            // RunEvent::Exit fires once when the app is about to terminate
            // (after all windows are gone). Tear the perimeter down here so
            // app-close ⇒ perimeter-down (P11). Synchronous, with a 30s
            // ceiling enforced by run_compose's timeout wrapper.
            if let tauri::RunEvent::Exit = &event {
                bring_perimeter_down_sync(&perimeter_root_exit);
                clear_runguard();
            }
        });
}
