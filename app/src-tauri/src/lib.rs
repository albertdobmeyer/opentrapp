mod bootstrap;
mod commands;
mod daemon_link;
mod idle;
mod lifecycle;
mod status_aggregator;

// Phase B (ADR-0019): orchestrator + util now live in the tauri-free
// `opentrapp-core` crate. Re-export them at the original crate paths so the
// (unchanged) GUI call sites — `crate::orchestrator::…`, `crate::util::…`,
// `State<'_, AppState>` — keep resolving.
pub use opentrapp_core::{orchestrator, util};

// The `cargo-fuzz` harnesses (app/src-tauri/fuzz/) target the parser,
// interpolator, and redactor — all of which now live in the tauri-free
// `opentrapp-core`. Their fuzz shim is `opentrapp_core::fuzz_api`, so the fuzz
// build never compiles this GUI crate's `tauri-build` (which fails under the
// sanitizer). No `fuzz_api` module here anymore.

use orchestrator::state::AppState;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_store::StoreExt;

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

/// Resolve the runtime data home (`~/.opentrapp/`), creating it if absent.
///
/// Replaces the old `find_monorepo_root()`, which walked the filesystem
/// looking for a source tree of workloads — an assumption that only held when
/// the app ran from a dev clone and broke on every installed AppImage. The
/// perimeter no longer needs a source tree: images are pre-built and verified,
/// and policy files live under `<data_dir>/perimeter/` (the resource dir).
fn runtime_data_dir() -> PathBuf {
    let dir = orchestrator::podman::runtime_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir
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
            "open" => open_dashboard(app),
            "quit" => request_quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_dashboard(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Open the dashboard: focus it if it already exists, otherwise BUILD it on
/// demand. The webview is a transient resource — it does not exist while the app
/// runs tray-only at rest (the ~222 MB WebKitWebProcess is freed on close). The
/// label MUST be "main" — `capabilities/default.json` binds all IPC/plugin
/// permissions to `windows:["main"]`.
fn open_dashboard(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    if let Err(e) = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("OpenTrApp")
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .resizable(true)
        .build()
    {
        eprintln!("[lib] failed to build dashboard window: {e}");
    }
}

/// Set the explicit-quit flag, then exit. The `RunEvent::ExitRequested` handler
/// only allows the process (and the perimeter teardown) to terminate when this
/// flag is set — otherwise it `prevent_exit()`s so closing the dashboard leaves
/// the lean tray-only daemon running.
fn request_quit(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.quitting.store(true, Ordering::SeqCst);
    }
    app.exit(0);
}

/// Whether closing the window should hide to tray (keep the app + the idle
/// waker alive) rather than quit. Reads `closeToTray` from the frontend
/// `settings.json` store; defaults to true to match `DEFAULT_SETTINGS`.
fn close_to_tray_enabled(handle: &tauri::AppHandle) -> bool {
    handle
        .store("settings.json")
        .ok()
        .and_then(|s| s.get("app_settings"))
        .and_then(|v| v.get("closeToTray").and_then(|b| b.as_bool()))
        .unwrap_or(true)
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

    let data_dir = runtime_data_dir();

    // Phase B / B4b (ADR-0019): opt-in (OPENTRAPP_DAEMON_DEFER=1) — ensure a
    // headless daemon owns the perimeter. If so, this GUI is a viewer and skips
    // RunGuard / bring-up / idle auto-pause / teardown, routing mutating commands
    // through the control channel. Default OFF → self-own exactly as before; any
    // failure to launch the daemon also falls back to self-owning.
    let daemon_owned = daemon_link::ensure_daemon(&data_dir);

    let app_state = AppState::new(data_dir.clone());
    app_state.daemon_owned.store(daemon_owned, Ordering::SeqCst);

    // RunGuard: reap orphan containers from any prior SIGKILL'd session BEFORE we
    // bring the perimeter up. When a daemon owns the perimeter, IT holds the
    // guard — the GUI must not establish (or later clear) it.
    if !daemon_owned {
        establish_runguard(&data_dir);
    }

    // Dormant (idle auto-pause) is a runtime-only state: opening the app means
    // the assistant is awake. Clear any stale dormant marker left by a crash
    // while dormant so a previous sleep can't strand the perimeter (ADR-0018).
    // When a daemon owns the perimeter (B4b), it owns the dormant lifecycle —
    // the GUI must not clear it (that would desync the daemon's sleep state).
    if !daemon_owned {
        lifecycle::clear_dormant_marker();
    }

    // Pass-4 lifecycle ownership (P11): the perimeter is bound to the app's
    // lifetime. App start → perimeter up. Graceful exit (window quit, tray Quit,
    // SIGTERM, SIGINT) → perimeter down. SIGKILL is reaped on next launch via
    // RunGuard above. Watchdog reports state every 30s; auto-restart of dead
    // containers is delegated to `restart: unless-stopped` in the spec.
    let perimeter_root_setup = data_dir.clone();
    let perimeter_root_exit = data_dir.clone();

    tauri::Builder::default()
        // Single-instance guard: second launch focuses the main window and exits.
        // Must be registered first per tauri-plugin-single-instance docs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            open_dashboard(app);
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
        .manage(commands::sentinel::SentinelActivityStore::new())
        .manage(app_state)
        .setup(move |app| {
            setup_tray(app)?;
            // Open the dashboard on launch (first-run wizard + normal UX). The
            // window is destroyed on close — leaving the lean tray-only daemon —
            // and rebuilt on demand from the tray. (windows:[] in tauri.conf.json
            // means we own window creation here, not at config-time.)
            open_dashboard(app.handle());
            // Bridge the core event bus → the Tauri event system (ADR-0022 §4): `core::stream`
            // (and future emitters) emit to `AppState.event_bus`; forward every event to the
            // webview's `listen()` hooks via `AppHandle::emit`. One core, two transports — the
            // loopback viewer-server fans the same bus out to its WS.
            let event_bus = app.state::<AppState>().event_bus.clone();
            let emit_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut rx = event_bus.subscribe();
                while let Ok(env) = rx.recv().await {
                    let _ = emit_handle.emit(env.event.as_str(), env.payload);
                }
            });
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
            // Bring the perimeter up only when the GUI owns it; when a daemon
            // owns it (B4b), it has already done so.
            if !daemon_owned {
                bootstrap::spawn_bootstrap_on_launch(app.handle().clone(), perimeter_root_setup.clone());
            }
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
            commands::credentials::save_credentials,
            commands::credentials::read_runtime_env,
            commands::sentinel::get_sentinel_activity,
            commands::sentinel::sentinel_judge,
            commands::egress::list_egress_approvals,
            commands::egress::apply_allowlist_decision,
        ])
        // On window close: let the window DESTROY (frees the ~222 MB
        // WebKitWebProcess — the old `hide()` kept it resident, which was the
        // heaviness + the SIGBUS-under-memory-pressure source). Whether the
        // *daemon* survives is decided in `RunEvent::ExitRequested` below:
        // closeToTray=true (default) keeps the tray daemon + idle waker alive via
        // `prevent_exit`; closeToTray=false means close==quit, so set the flag
        // here. (ADR-0018: the waker survives structurally now, not via hide.)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if !close_to_tray_enabled(window.app_handle()) {
                    if let Some(state) = window.app_handle().try_state::<AppState>() {
                        state.quitting.store(true, Ordering::SeqCst);
                    }
                }
                // Do NOT prevent_close — the window is allowed to destroy.
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |app_handle, event| match &event {
            // Fires when the last window closes OR on an explicit exit request.
            // Unless an explicit Quit set the flag, veto the exit so the lean
            // tray-only daemon (watchdog + idle waker + perimeter) lives on with
            // zero windows. The tray icon holds the event loop open.
            tauri::RunEvent::ExitRequested { api, .. } => {
                let quitting = app_handle
                    .try_state::<AppState>()
                    .map(|s| s.quitting.load(Ordering::SeqCst))
                    .unwrap_or(false);
                if !quitting {
                    api.prevent_exit();
                }
            }
            // Only reached on an explicit, un-vetoed Quit (tray Quit / SIGTERM /
            // SIGINT, which set the flag). Tear the perimeter down (P11):
            // app-quit ⇒ perimeter-down. Synchronous, timeouts in the orchestrator.
            tauri::RunEvent::Exit => {
                // The GUI tears down only what it owns. When a daemon owns the
                // perimeter (B4b), leave it — and its RunGuard — running.
                if !daemon_owned {
                    bring_perimeter_down_sync(&perimeter_root_exit);
                    clear_runguard();
                }
            }
            _ => {}
        });
}
