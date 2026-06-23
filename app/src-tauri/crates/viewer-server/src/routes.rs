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

use opentrapp_core::orchestrator::discovery::{discover_components, DiscoveredComponent};
use opentrapp_core::orchestrator::error::OrchestratorError;
use opentrapp_core::orchestrator::runner::CommandResult;

/// Shared route state: the discovered workload manifests (the slice the core fns operate on,
/// cached once at startup like the Tauri backend's component cache) + the runtime data dir (where
/// the perimeter / on-demand shields live). The server adds NO orchestration logic — it calls core.
#[derive(Clone)]
pub struct AppState {
    pub components: Arc<Vec<DiscoveredComponent>>,
    pub runtime_data_dir: PathBuf,
    pub monorepo_root: PathBuf,
}

impl AppState {
    /// Discover the workload manifests under `monorepo_root` and cache them (mirrors the Tauri
    /// backend populating its component cache at startup). `runtime_data_dir` is where the
    /// perimeter `.env` + on-demand shields live (`~/.opentrapp` in production).
    pub fn discover(
        monorepo_root: PathBuf,
        runtime_data_dir: PathBuf,
    ) -> Result<Self, OrchestratorError> {
        let components = Arc::new(discover_components(&monorepo_root)?);
        Ok(Self { components, runtime_data_dir, monorepo_root })
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
        .with_state(state)
}

// ───── argument bodies (mirror the Tauri command parameters) ─────────────────────────────────

#[derive(Deserialize)]
struct ComponentIdArg {
    component_id: String,
}
#[derive(Deserialize)]
struct HealthProbeArgs {
    component_id: String,
    probe_command: String,
    timeout_seconds: u64,
}
#[derive(Deserialize)]
struct ReadConfigArgs {
    component_id: String,
    config_path: String,
}
#[derive(Deserialize)]
struct WriteConfigArgs {
    component_id: String,
    config_path: String,
    content: String,
}
#[derive(Deserialize)]
struct LoadOptionsArgs {
    component_id: String,
    command_string: String,
    timeout_seconds: u64,
}
#[derive(Deserialize)]
struct RunCommandArgs {
    component_id: String,
    command_id: String,
    args: HashMap<String, String>,
}
#[derive(Deserialize)]
struct ExecuteWorkflowArgs {
    component_id: String,
    workflow_id: String,
    inputs: HashMap<String, String>,
}
#[derive(Deserialize)]
struct KeyArg {
    key: String,
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

// ───── error mapping ─────────────────────────────────────────────────────────────────────────

/// Map a core error to an HTTP status + `{error}` body (spec §1): not-found → 404, else 500.
pub struct ApiError(StatusCode, String);

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
