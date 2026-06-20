//! SCAFFOLD — the route registry (spec §1.1). THE API CONTRACT, made concrete.
//!
//! Every `#[tauri::command]` becomes `POST /api/<command_name>`: JSON body = named args,
//! JSON response = the return value, errors → 4xx/5xx + {error}. Each route calls an
//! `opentrapp-core` fn (the lifted, transport-neutral handler) — the server NEVER duplicates
//! orchestration logic (CLAUDE.md §5). `bi` = boundary_impact (ADR-0021): N=neutral
//! (agent-operable), W=weakening (out-of-band human confirmation; NO agent call edge).
//!
//! The §6 contract test (orchestrator-check) asserts this registry matches tauri.ts 1:1 — so
//! this list is the source of truth for completeness. Reconcile against the real
//! `#[tauri::command]` set when lifting (the spike only needs the 3 starred handlers).

// use axum::{Router, routing::post};
// pub fn router(core: opentrapp_core::Engine) -> Router {
//   Router::new()
//     // ── Manifest / discovery (N) ──────────────────────────────────────────
//     .route("/api/list_components",            post(list_components))   // ★ spike → orchestrator::discover
//     .route("/api/get_component",              post(get_component))     //          orchestrator::discover
//     .route("/api/set_monorepo_root",          post(set_monorepo_root)) //          orchestrator::discover
//     // ── Status / health (N) ───────────────────────────────────────────────
//     .route("/api/get_status",                 post(get_status))        // ★ spike → orchestrator::status
//     .route("/api/run_health_probe",           post(run_health_probe))  //          orchestrator::health
//     // ── Command exec (N*; per-command danger from component.yml still applies) ─
//     .route("/api/run_command",                post(run_command))       // ★ spike → orchestrator::runner
//     .route("/api/load_options",               post(load_options))      //          orchestrator::runner
//     .route("/api/start_stream",               post(start_stream))      //          runner (emits stream-line/-end on WS)
//     .route("/api/stop_stream",                post(stop_stream))       //          runner
//     // ── Config ────────────────────────────────────────────────────────────
//     .route("/api/read_config",                post(read_config))       // N        orchestrator::config (path-traversal guard)
//     .route("/api/write_config",               post(write_config))      // W        orchestrator::config
//     .route("/api/create_config_from_template",post(create_config_from_template)) // N  orchestrator::config
//     // ── Workflows (N*) ────────────────────────────────────────────────────
//     .route("/api/list_workflows",             post(list_workflows))    // N        orchestrator::workflow
//     .route("/api/execute_workflow",           post(execute_workflow))  // N*       orchestrator::workflow
//     // ── Lifecycle / perimeter (reads N, mutations W via control::submit) ──
//     .route("/api/get_perimeter_state",        post(get_perimeter_state))  // N     supervisor/markers
//     .route("/api/get_assistant_status",       post(get_assistant_status)) // N     markers/status-agg
//     .route("/api/restart_perimeter",          post(restart_perimeter))    // W     control::submit(Restart)
//     .route("/api/pause_perimeter",            post(pause_perimeter))      // W     control::submit(Pause)
//     .route("/api/resume_perimeter",           post(resume_perimeter))     // W     control::submit(Resume)
//     .route("/api/retry_bootstrap",            post(retry_bootstrap))      // W     supervisor
//     // ── Prerequisites (N) ─────────────────────────────────────────────────
//     .route("/api/check_prerequisites",        post(check_prerequisites))  // N     orchestrator::prereq
//     .route("/api/init_submodules",            post(init_submodules))      // N     host git
//     // ── Diagnostics (N) ───────────────────────────────────────────────────
//     .route("/api/generate_diagnostic_bundle", post(generate_diagnostic_bundle)) // N  diagnostics (redacted)
//     // ── Credentials / activation ──────────────────────────────────────────
//     .route("/api/validate_anthropic_key",     post(validate_anthropic_key)) // N   credentials (live ping)
//     .route("/api/save_credentials",           post(save_credentials))       // W   credentials (0600 .env)
//     .route("/api/read_runtime_env",           post(read_runtime_env))       // N   credentials
//     .route("/api/commit_activation",          post(commit_activation))      // W   credentials/supervisor
//     // ── Sentinel (N) ──────────────────────────────────────────────────────
//     .route("/api/sentinel_judge",             post(sentinel_judge))      // N      sentinel (judge.sh)
//     .route("/api/get_sentinel_activity",      post(get_sentinel_activity)) // N    sentinel
//     // ── Egress approvals (read N; write W — ADR-0016 human-only writer) ──
//     .route("/api/list_egress_approvals",      post(list_egress_approvals))   // N  egress (read+judge)
//     .route("/api/apply_allowlist_decision",   post(apply_allowlist_decision))// W  egress
//     // ── Telegram (N; all calls Rust-side so the token never hits the browser) ─
//     .route("/api/derive_telegram_bot_url",    post(derive_telegram_bot_url)) // N  telegram
//     .route("/api/telegram_delete_webhook",    post(telegram_delete_webhook)) // N  telegram
//     .route("/api/telegram_poll_for_start",    post(telegram_poll_for_start)) // N  telegram
//     .route("/api/telegram_send_message",      post(telegram_send_message))   // N  telegram
//     .route("/api/telegram_advance_offset",    post(telegram_advance_offset)) // N  telegram
//     .with_state(core)
// }

// ── The 3 spike handlers (sketch the real ones; the rest follow the same shape) ──
// async fn list_components(State(core): State<Engine>) -> Result<Json<Vec<DiscoveredComponent>>, ApiError> {
//     Ok(Json(core.list_components().await?))
// }
// async fn get_status(State(core): State<Engine>, Json(a): Json<GetStatusArgs>) -> ... { ... }
// async fn run_command(State(core): State<Engine>, Json(a): Json<RunCommandArgs>) -> ... { ... }
//
// W-tagged handlers (write_config, pause/resume/restart_perimeter, save_credentials,
// commit_activation, apply_allowlist_decision, retry_bootstrap): these MUST go through the
// ADR-0021 danger-gate — no direct agent-callable edge to the weakening write.

#![allow(dead_code)]
