//! The route registry (spec §1.1). Each route calls an `opentrapp-core` fn; the server NEVER
//! duplicates orchestration logic (CLAUDE.md §5). Every `#[tauri::command]` becomes
//! `POST /api/<name>` (JSON body = named args, JSON response = return value, errors → 4xx/5xx
//! + `{error}`). The §6 contract test asserts this registry matches `tauri.ts` 1:1.
//!
//! SPIKE scope: `list_components` proves the manifest-render path end-to-end (fetch → axum →
//! core → typed JSON). `get_status` and `run_command` follow the identical shape and are lifted
//! into `opentrapp-core` at migration step 1 (they currently live in the Tauri command layer
//! with `AppState`/`AppHandle` deps).

use std::path::PathBuf;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use opentrapp_core::orchestrator::discovery::{discover_components, DiscoveredComponent};
use opentrapp_core::orchestrator::error::OrchestratorError;

/// Shared route state. The spike holds the monorepo root; the daemon reuses its core handle
/// (the server adds NO orchestration logic — it calls core, like the Tauri commands do).
#[derive(Clone)]
pub struct AppState {
    pub monorepo_root: PathBuf,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/list_components", post(list_components))
        .with_state(state)
}

/// ★ spike — manifest render: discover the workload manifests under the monorepo root and
/// return them as JSON, exactly as the Tauri `list_components` command does (minus the cache).
async fn list_components(
    State(st): State<AppState>,
) -> Result<Json<Vec<DiscoveredComponent>>, ApiError> {
    Ok(Json(discover_components(&st.monorepo_root)?))
}

/// Map a core error to an HTTP status + `{error}` body (spec §1): not-found → 404, else 500.
pub struct ApiError(StatusCode, String);

impl From<OrchestratorError> for ApiError {
    fn from(e: OrchestratorError) -> Self {
        let code = match &e {
            OrchestratorError::ComponentNotFound(_) | OrchestratorError::NotFound(_) => {
                StatusCode::NOT_FOUND
            }
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
