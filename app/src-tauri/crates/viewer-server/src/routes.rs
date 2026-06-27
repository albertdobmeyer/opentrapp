//! The route registry (spec §1.1 / ADR-0022). Each route calls an `opentrapp-core` fn; the server
//! NEVER duplicates orchestration logic (CLAUDE.md §5). Every `#[tauri::command]` becomes
//! `POST /api/<name>` — JSON body = named args, JSON response = the return value, errors → 4xx/5xx
//! + `{error}`. This is the same surface `tauri.ts` invokes; the §6 contract test asserts the 1:1
//! mapping. The handlers are thin: deserialize args → call the lifted core fn with the cached
//! component slice / runtime dir → serialize. The Tauri command layer became one-line shims over
//! exactly these core fns (migration step 1), so this route layer and the Tauri layer are two
//! transport projections of one core.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;

use opentrapp_core::events::EventBus;
use opentrapp_core::orchestrator::discovery::{discover_components, DiscoveredComponent};
use opentrapp_core::orchestrator::error::OrchestratorError;
use opentrapp_core::orchestrator::runner::CommandResult;
use opentrapp_core::stream::ActiveStreams;

/// Shared route state: the discovered workload manifests (the slice the core fns operate on,
/// cached once at startup like the Tauri backend's component cache) + the runtime data dir (where
/// the perimeter / on-demand shields live) + the event bus (streaming commands emit `stream-line` /
/// `stream-end` here; the `/api/events` WS fans them out) + the active-stream PID table. The server
/// adds NO orchestration logic — it calls core.
#[derive(Clone)]
pub struct AppState {
    pub components: Arc<Vec<DiscoveredComponent>>,
    pub runtime_data_dir: PathBuf,
    pub monorepo_root: PathBuf,
    pub event_bus: EventBus,
    pub active_streams: Arc<ActiveStreams>,
}

impl AppState {
    /// Discover the workload manifests under `monorepo_root` and cache them (mirrors the Tauri
    /// backend populating its component cache at startup). `runtime_data_dir` is where the
    /// perimeter `.env` + on-demand shields live (`~/.opentrapp` in production). A fresh event bus +
    /// active-stream table are created here; `main` clones the bus into the `/api/events` WS router.
    pub fn discover(
        monorepo_root: PathBuf,
        runtime_data_dir: PathBuf,
    ) -> Result<Self, OrchestratorError> {
        let components = Arc::new(discover_components(&monorepo_root)?);
        Ok(Self {
            components,
            runtime_data_dir,
            monorepo_root,
            event_bus: EventBus::new(),
            active_streams: Arc::new(ActiveStreams::default()),
        })
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        // read surface
        .route("/api/list_components", post(list_components))
        .route("/api/get_component", post(get_component))
        .route("/api/get_status", post(get_status))
        .route("/api/run_health_probe", post(run_health_probe))
        .route("/api/list_workflows", post(list_workflows))
        .route("/api/read_config", post(read_config))
        .route("/api/load_options", post(load_options))
        .route("/api/check_prerequisites", post(check_prerequisites))
        .route("/api/validate_anthropic_key", post(validate_anthropic_key))
        // write / execute surface (the same ops the Tauri GUI exposes; none is boundary-weakening —
        // the danger-gated allowlist/lifecycle ops, ADR-0021, are deliberately NOT mounted here)
        .route("/api/write_config", post(write_config))
        .route("/api/run_command", post(run_command))
        .route("/api/execute_workflow", post(execute_workflow))
        // ADR-0021 out-of-band approval surface — the human two-tap that APPLIES a held
        // boundary-weakening request. This is the *inverse* of the daemon-only direct-apply
        // ops (which §6 forbids here): listing never weakens, and `approve_weakening` is the
        // sole pending→applied edge (`supervisor::apply_approved`), reachable only behind this
        // surface's §2 transport (loopback + Host/Origin + bearer; ADR-0022).
        .route("/api/list_pending_approvals", post(list_pending_approvals))
        .route("/api/approve_weakening", post(approve_weakening))
        // first-run setup
        .route("/api/init_submodules", post(init_submodules))
        .route("/api/create_config_from_template", post(create_config_from_template))
        .route("/api/generate_diagnostic_bundle", post(generate_diagnostic_bundle))
        // telegram waker channel (token-based, stateless network ops)
        .route("/api/derive_telegram_bot_url", post(derive_telegram_bot_url))
        .route("/api/telegram_delete_webhook", post(telegram_delete_webhook))
        .route("/api/telegram_poll_for_start", post(telegram_poll_for_start))
        .route("/api/telegram_send_message", post(telegram_send_message))
        .route("/api/telegram_advance_offset", post(telegram_advance_offset))
        // streaming command output — emits stream-line/stream-end to the bus the /api/events WS
        // fans out (the high-frequency event path the WS was designed for, spec §1.2)
        .route("/api/start_stream", post(start_stream))
        .route("/api/stop_stream", post(stop_stream))
        .with_state(state)
}

// ───── argument bodies (mirror the Tauri command parameters) ─────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentIdArg {
    component_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HealthProbeArgs {
    component_id: String,
    probe_command: String,
    timeout_seconds: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadConfigArgs {
    component_id: String,
    config_path: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteConfigArgs {
    component_id: String,
    config_path: String,
    content: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadOptionsArgs {
    component_id: String,
    command_string: String,
    timeout_seconds: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunCommandArgs {
    component_id: String,
    command_id: String,
    args: HashMap<String, String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopStreamArgs {
    component_id: String,
    command_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteWorkflowArgs {
    component_id: String,
    workflow_id: String,
    inputs: HashMap<String, String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyArg {
    key: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateConfigArgs {
    component_id: String,
    config_path: String,
    template_path: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenArg {
    token: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramPollArgs {
    token: String,
    offset: i64,
    timeout_secs: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramSendArgs {
    token: String,
    chat_id: i64,
    text: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramAdvanceArgs {
    token: String,
    update_id: i64,
}

// ───── handlers (thin projections over opentrapp-core) ───────────────────────────────────────

/// Return the cached workload manifests (the Tauri `list_components` returns its cache; here the
/// cache is `AppState.components`, discovered at startup).
async fn list_components(State(st): State<AppState>) -> Json<Vec<DiscoveredComponent>> {
    Json((*st.components).clone())
}

async fn get_component(
    State(st): State<AppState>,
    Json(a): Json<ComponentIdArg>,
) -> Result<Json<DiscoveredComponent>, ApiError> {
    st.components
        .iter()
        .find(|c| c.manifest.identity.id == a.component_id)
        .cloned()
        .map(Json)
        .ok_or_else(|| {
            ApiError(StatusCode::NOT_FOUND, format!("component not found: {}", a.component_id))
        })
}

async fn get_status(
    State(st): State<AppState>,
    Json(a): Json<ComponentIdArg>,
) -> Result<Json<opentrapp_core::status::ComponentStatus>, ApiError> {
    Ok(Json(opentrapp_core::status::evaluate_status(&st.components, a.component_id).await?))
}

async fn run_health_probe(
    State(st): State<AppState>,
    Json(a): Json<HealthProbeArgs>,
) -> Result<Json<opentrapp_core::health::HealthResult>, ApiError> {
    Ok(Json(
        opentrapp_core::health::run_health_probe(
            &st.components,
            a.component_id,
            a.probe_command,
            a.timeout_seconds,
        )
        .await?,
    ))
}

async fn list_workflows(
    State(st): State<AppState>,
    Json(a): Json<ComponentIdArg>,
) -> Result<Json<Vec<opentrapp_core::orchestrator::manifest::Workflow>>, ApiError> {
    Ok(Json(opentrapp_core::workflow_ops::list_workflows(&st.components, a.component_id)?))
}

async fn read_config(
    State(st): State<AppState>,
    Json(a): Json<ReadConfigArgs>,
) -> Result<Json<String>, ApiError> {
    Ok(Json(opentrapp_core::config_ops::read_config(&st.components, a.component_id, a.config_path)?))
}

async fn load_options(
    State(st): State<AppState>,
    Json(a): Json<LoadOptionsArgs>,
) -> Result<Json<Vec<String>>, ApiError> {
    Ok(Json(
        opentrapp_core::execute::load_options(
            &st.components,
            a.component_id,
            a.command_string,
            a.timeout_seconds,
        )
        .await?,
    ))
}

async fn check_prerequisites(
    State(st): State<AppState>,
) -> Result<Json<opentrapp_core::prerequisites::PrerequisiteReport>, ApiError> {
    Ok(Json(opentrapp_core::prerequisites::check_prerequisites(&st.runtime_data_dir).await?))
}

/// Host-side Anthropic key pre-flight. `credentials::validate_anthropic_key` returns a `String`
/// error only on a total network failure (already redacted in core); a 502 conveys that.
async fn validate_anthropic_key(
    Json(a): Json<KeyArg>,
) -> Result<Json<opentrapp_core::credentials::ValidationOutcome>, ApiError> {
    opentrapp_core::credentials::validate_anthropic_key(a.key)
        .await
        .map(Json)
        .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e))
}

async fn write_config(
    State(st): State<AppState>,
    Json(a): Json<WriteConfigArgs>,
) -> Result<Json<()>, ApiError> {
    opentrapp_core::config_ops::write_config(
        &st.components,
        a.component_id,
        a.config_path,
        a.content,
    )?;
    Ok(Json(()))
}

/// Run a manifest command. The lifted core fn returns a `RunOutcome` whose `on_demand_service` lets
/// the *daemon* arm an idle-stop (it owns container lifetime, ADR-0019); the viewer-server is a
/// projection and just returns the command result.
async fn run_command(
    State(st): State<AppState>,
    Json(a): Json<RunCommandArgs>,
) -> Result<Json<CommandResult>, ApiError> {
    let outcome = opentrapp_core::execute::run_command(
        &st.components,
        &st.runtime_data_dir,
        a.component_id,
        a.command_id,
        &a.args,
    )
    .await?;
    outcome.result.map(Json).map_err(ApiError::from)
}

/// ADR-0021 approval surface — list the boundary-weakening requests the daemon has HELD for
/// out-of-band human approval (id + a plain-language label). Read-only: listing never weakens.
async fn list_pending_approvals(
    State(st): State<AppState>,
) -> Json<Vec<opentrapp_core::approvals::PendingApproval>> {
    Json(opentrapp_core::approvals::list_pending(&st.runtime_data_dir))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApproveWeakeningArgs {
    id: String,
}

/// ADR-0021 approval surface — APPLY a held weakening request the human approves (the two-tap).
/// The sole pending→applied edge (`supervisor::apply_approved`), reachable only behind this
/// surface's §2 transport (loopback + Host/Origin + bearer). Returns whether a pending request
/// with `id` was found and applied (idempotent — a missing id is `false`, never a double-apply).
async fn approve_weakening(
    State(st): State<AppState>,
    Json(a): Json<ApproveWeakeningArgs>,
) -> Json<bool> {
    Json(opentrapp_core::supervisor::apply_approved(&st.runtime_data_dir, &a.id).await)
}

/// Start streaming a command's output. Lines arrive asynchronously as `stream-line` / `stream-end`
/// events on the bus, which the `/api/events` WS fans out to the browser (mirrors the Tauri
/// `start_stream` + its `emit`). Returns `()` once the process is spawned.
async fn start_stream(
    State(st): State<AppState>,
    Json(a): Json<RunCommandArgs>,
) -> Result<Json<()>, ApiError> {
    opentrapp_core::stream::start_stream(
        &st.components,
        &st.active_streams,
        &st.event_bus,
        a.component_id,
        a.command_id,
        &a.args,
    )
    .await?;
    Ok(Json(()))
}

/// Stop a running stream (best-effort kill). Never boundary-weakening — it only cancels a process
/// this server started.
async fn stop_stream(
    State(st): State<AppState>,
    Json(a): Json<StopStreamArgs>,
) -> Result<Json<()>, ApiError> {
    opentrapp_core::stream::stop_stream(&st.active_streams, &a.component_id, &a.command_id)?;
    Ok(Json(()))
}

async fn execute_workflow(
    State(st): State<AppState>,
    Json(a): Json<ExecuteWorkflowArgs>,
) -> Result<Json<opentrapp_core::orchestrator::workflow::WorkflowResult>, ApiError> {
    Ok(Json(
        opentrapp_core::workflow_ops::execute_workflow(
            &st.components,
            a.component_id,
            a.workflow_id,
            &a.inputs,
        )
        .await?,
    ))
}

// ───── first-run setup ───────────────────────────────────────────────────────────────────────

async fn init_submodules(State(st): State<AppState>) -> Result<Json<String>, ApiError> {
    Ok(Json(opentrapp_core::prerequisites::init_submodules(&st.runtime_data_dir).await?))
}

async fn create_config_from_template(
    State(st): State<AppState>,
    Json(a): Json<CreateConfigArgs>,
) -> Result<Json<()>, ApiError> {
    opentrapp_core::prerequisites::create_config_from_template(
        &st.components,
        a.component_id,
        a.config_path,
        a.template_path,
    )?;
    Ok(Json(()))
}

/// Redacted diagnostic bundle. Core stays clock/transport-neutral, so the route injects the
/// timestamp (unix seconds — no chrono dep) + this server's version, mirroring the Tauri shim.
async fn generate_diagnostic_bundle() -> Result<Json<String>, ApiError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let generated_at = format!("unix:{now}");
    opentrapp_core::diagnostics::generate_bundle(&generated_at, env!("CARGO_PKG_VERSION"))
        .map(Json)
        .map_err(ApiError::upstream)
}

// ───── telegram waker channel (token-based; the String errors are upstream/network → 502) ─────

async fn derive_telegram_bot_url(
    Json(a): Json<TokenArg>,
) -> Result<Json<opentrapp_core::telegram::TelegramBot>, ApiError> {
    opentrapp_core::telegram::derive_telegram_bot_url(a.token)
        .await
        .map(Json)
        .map_err(ApiError::upstream)
}

async fn telegram_delete_webhook(Json(a): Json<TokenArg>) -> Result<Json<()>, ApiError> {
    opentrapp_core::telegram::telegram_delete_webhook(a.token)
        .await
        .map(Json)
        .map_err(ApiError::upstream)
}

async fn telegram_poll_for_start(
    Json(a): Json<TelegramPollArgs>,
) -> Result<Json<Option<opentrapp_core::telegram::TelegramUpdate>>, ApiError> {
    opentrapp_core::telegram::telegram_poll_for_start(a.token, a.offset, a.timeout_secs)
        .await
        .map(Json)
        .map_err(ApiError::upstream)
}

async fn telegram_send_message(Json(a): Json<TelegramSendArgs>) -> Result<Json<()>, ApiError> {
    opentrapp_core::telegram::telegram_send_message(a.token, a.chat_id, a.text)
        .await
        .map(Json)
        .map_err(ApiError::upstream)
}

async fn telegram_advance_offset(Json(a): Json<TelegramAdvanceArgs>) -> Result<Json<()>, ApiError> {
    opentrapp_core::telegram::telegram_advance_offset(a.token, a.update_id)
        .await
        .map(Json)
        .map_err(ApiError::upstream)
}

// ───── error mapping ─────────────────────────────────────────────────────────────────────────

/// Map a core error to an HTTP status + `{error}` body (spec §1): not-found → 404, else 500.
pub struct ApiError(StatusCode, String);

impl ApiError {
    /// A plain-`String` error from a network/upstream/IO op (telegram, diagnostics, key validation)
    /// → 502 Bad Gateway — not a client 4xx fault.
    fn upstream(msg: String) -> Self {
        ApiError(StatusCode::BAD_GATEWAY, msg)
    }
}

impl From<OrchestratorError> for ApiError {
    fn from(e: OrchestratorError) -> Self {
        let code = match &e {
            OrchestratorError::ComponentNotFound(_)
            | OrchestratorError::CommandNotFound { .. }
            | OrchestratorError::WorkflowNotFound { .. }
            | OrchestratorError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiError(code, e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}
