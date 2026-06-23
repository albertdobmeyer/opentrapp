//! viewer-server — the on-demand loopback web GUI server (ADR-0022 §0 spike).
//!
//! Flow: bind 127.0.0.1:0 → build router (security layer + routes + SPA) → write {port,token}
//! to a 0600 file → open the browser to a bare loopback URL carrying a single-use launch nonce
//! in the URL *fragment* → serve until session end / idle. ON-DEMAND ONLY: never start this
//! from the always-on daemon path (spec §2; CLAUDE.md §10).
#![allow(dead_code)] // SPIKE: some scaffold paths (events WS, browser-open) still being wired.

mod events;
mod routes;
mod security;
mod session;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::{response::Html, routing::get, Router};
use tower_http::services::{ServeDir, ServeFile};

/// Compose the router: the `/api` routes + the `/api/session` bootstrap + the `/api/events` WS +
/// the SPA (the built React app via `ServeDir`, or the placeholder shell when no `dist/` is
/// present), wrapped in the §2 security middleware (Host allowlist → Origin allowlist → bearer;
/// the SPA, `/api/session`, and the WS handshake are bearer-exempt — Host+Origin checked only —
/// because the SPA carries no secret and is where the bearer is issued / first-frame-verified).
fn build_app(
    sec: security::SecurityState,
    app_state: routes::AppState,
    session: session::SharedSession,
    dist_dir: Option<PathBuf>,
) -> Router {
    let api = routes::router(app_state)
        .merge(session::router(session)) // /api/session nonce→bearer bootstrap exchange
        .merge(events::router(sec.clone())); // /api/events WS (first-frame bearer auth)

    // Serve the built React app from `dist/` with an index.html SPA fallback (client-side routes);
    // fall back to the placeholder shell when there is no build (dev / tests).
    let with_spa = match dist_dir {
        Some(dir) if dir.join("index.html").is_file() => {
            let index = ServeFile::new(dir.join("index.html"));
            api.fallback_service(ServeDir::new(&dir).fallback(index))
        }
        _ => api.route("/", get(spa_shell)),
    };

    with_spa.layer(axum::middleware::from_fn_with_state(sec, security::enforce))
}

/// SPIKE SPA shell. The real serve mounts `app/dist/` via tower-http `ServeDir` (§5); this
/// placeholder proves the shell loads under the Host check with no bearer.
async fn spa_shell() -> Html<&'static str> {
    Html("<!doctype html><meta charset=utf-8><title>OpenTrApp viewer (spike)</title><div id=\"root\">spike</div>")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // §2.1 — bind loopback + ephemeral port; ASSERT loopback before serving.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    assert!(addr.ip().is_loopback(), "viewer-server MUST bind loopback only");

    // §2.3 — mint a 256-bit token + single-use launch nonce; persist {port,token} 0600.
    let viewer_dir = PathBuf::from("/tmp/opentrapp-viewer-spike"); // TODO: ~/.opentrapp/viewer/
    let (session, _path) = session::Session::mint_in(addr.port(), &viewer_dir)?;
    let nonce = session.nonce().unwrap_or("").to_string();

    let sec = security::SecurityState {
        port: addr.port(),
        token: Arc::new(session.token().to_string()),
    };
    // The /api/session route mutates the session to burn the launch nonce.
    let shared_session: session::SharedSession = Arc::new(Mutex::new(session));
    // Discover the workload manifests (dev: under the cwd) + the runtime data dir where the
    // perimeter / on-demand shields live (the real daemon passes ~/.opentrapp).
    let app_state = routes::AppState::discover(
        std::env::current_dir()?,
        opentrapp_core::orchestrator::podman::runtime_data_dir(),
    )?;
    // The daemon / launcher points this at the bundled React `dist/` (step 4); absent → the
    // placeholder shell, so the server still boots in a bare dev checkout.
    let dist_dir = std::env::var("OPENTRAPP_VIEWER_DIST").ok().map(PathBuf::from);
    let app = build_app(sec, app_state, shared_session, dist_dir);

    // §2.3 — open the browser to the BARE loopback URL with the nonce in the FRAGMENT (#n=…),
    // never a query string (that is the leak we designed out). TODO: xdg-open/open/start
    // directly (NOT tauri-plugin-shell). For the spike, print it.
    eprintln!("[viewer-server] open: http://127.0.0.1:{}/#n={nonce}", addr.port());

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod integration {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request as Req, StatusCode};
    use tower::ServiceExt;

    /// The repo root (4 levels up from this crate): discover finds the real workloads.
    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../..")
            .canonicalize()
            .expect("repo root")
    }

    /// A throwaway session for tests that exercise the API/WS surface (they auth via the bearer
    /// cookie directly, not the nonce exchange).
    fn dummy_session() -> session::SharedSession {
        let dir = std::env::temp_dir().join(format!("otv-dummy-{}", std::process::id()));
        Arc::new(Mutex::new(session::Session::mint_in(8080, &dir).unwrap().0))
    }

    fn app() -> Router {
        let sec = security::SecurityState { port: 8080, token: Arc::new("tok".to_string()) };
        let state = routes::AppState::discover(repo_root(), std::env::temp_dir()).expect("discover workloads");
        build_app(sec, state, dummy_session(), None)
    }

    #[tokio::test]
    async fn list_components_returns_real_manifests_through_the_secure_path() {
        // valid Host + Origin + bearer → 200 + JSON with the REAL workload manifests (core call).
        let req = Req::post("/api/list_components")
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://127.0.0.1:8080")
            .header("cookie", "otv_bearer=tok")
            .body(Body::empty())
            .unwrap();
        let resp = app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        let raw = String::from_utf8_lossy(&bytes);
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            !v.as_array().expect("array").is_empty(),
            "discovered the real workload manifests through the secure path"
        );
        assert!(raw.contains("agent"), "the agent workload is in the response");
    }

    #[tokio::test]
    async fn list_components_tokenless_is_401() {
        let req = Req::post("/api/list_components")
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://127.0.0.1:8080")
            .body(Body::empty())
            .unwrap();
        assert_eq!(app().oneshot(req).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    }

    /// A POST through the full secure path (valid Host + Origin + bearer) carrying a JSON body.
    fn authed_post(path: &str, body: serde_json::Value) -> Req<Body> {
        Req::post(path)
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://127.0.0.1:8080")
            .header("cookie", "otv_bearer=tok")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn get_component_returns_the_agent_through_the_secure_path() {
        let req = authed_post("/api/get_component", serde_json::json!({ "componentId": "agent" }));
        let resp = app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("agent"), "the agent component is returned");
    }

    #[tokio::test]
    async fn get_component_unknown_is_404() {
        // The route's ApiError maps ComponentNotFound → 404 (spec §1 error contract).
        let req = authed_post("/api/get_component", serde_json::json!({ "componentId": "no-such-x" }));
        assert_eq!(app().oneshot(req).await.unwrap().status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_status_evaluates_a_real_component_through_core() {
        // evaluate_status runs the component's probes (or returns the default/"unknown" with no
        // perimeter up); the point is the slice-based core call returns a typed state through the route.
        let req = authed_post("/api/get_status", serde_json::json!({ "componentId": "agent" }));
        let resp = app().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["component_id"], "agent");
        assert!(v["state_id"].is_string(), "a resolved state is returned");
    }

    /// A new route with a MULTI-field camelCase body (the shape the frontend shim actually sends):
    /// the args deserialize (componentId/configPath/templatePath), and an unknown component → 404.
    #[tokio::test]
    async fn create_config_from_template_uses_camelcase_args_and_maps_not_found() {
        let req = authed_post(
            "/api/create_config_from_template",
            serde_json::json!({
                "componentId": "no-such-x",
                "configPath": ".env",
                "templatePath": ".env.example",
            }),
        );
        assert_eq!(app().oneshot(req).await.unwrap().status(), StatusCode::NOT_FOUND);
    }

    /// The §2.3 bootstrap: POST /api/session {nonce} issues the bearer as an HttpOnly+SameSite
    /// cookie (never in the body), and a replay of the burned nonce is rejected.
    #[tokio::test]
    async fn session_exchange_issues_httponly_cookie_then_burns_the_nonce() {
        let dir = std::env::temp_dir().join(format!("otv-exchange-{}", std::process::id()));
        let (sess, _) = session::Session::mint_in(8080, &dir).unwrap();
        let nonce = sess.nonce().unwrap().to_string();
        let sec = security::SecurityState { port: 8080, token: Arc::new(sess.token().to_string()) };
        let state = routes::AppState::discover(repo_root(), std::env::temp_dir()).unwrap();
        let app = build_app(sec, state, Arc::new(Mutex::new(sess)), None);

        let exchange = |n: &str| {
            Req::post("/api/session")
                .header("host", "127.0.0.1:8080")
                .header("origin", "http://127.0.0.1:8080")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "nonce": n }).to_string()))
                .unwrap()
        };

        // first exchange: the valid nonce → 200 + Set-Cookie (HttpOnly, SameSite=Strict), no token in body
        let resp = app.clone().oneshot(exchange(&nonce)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cookie = resp.headers().get("set-cookie").unwrap().to_str().unwrap().to_string();
        assert!(cookie.contains("otv_bearer="), "the bearer is issued as a cookie");
        assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"), "hardened cookie attrs");
        let body = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("otv_bearer"), "the bearer is NOT echoed in the body");

        // replay the now-burned nonce → 401 (single use)
        assert_eq!(app.oneshot(exchange(&nonce)).await.unwrap().status(), StatusCode::UNAUTHORIZED);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With a built `dist/`, the SPA is served at `/` (Host-checked, no bearer — it carries no
    /// secret); the security layer still wraps the static serving (a forged Host is 403 even for
    /// the SPA, so ServeDir is not a Host-check bypass).
    #[tokio::test]
    async fn serves_the_built_spa_from_dist_under_the_host_check() {
        let dist = std::env::temp_dir().join(format!("otv-dist-{}", std::process::id()));
        std::fs::create_dir_all(&dist).unwrap();
        std::fs::write(dist.join("index.html"), "<!doctype html><div id=root>real-app</div>").unwrap();
        let sec = security::SecurityState { port: 8080, token: Arc::new("tok".to_string()) };
        let state = routes::AppState::discover(repo_root(), std::env::temp_dir()).unwrap();
        let app = build_app(sec, state, dummy_session(), Some(dist.clone()));

        // valid Host, no bearer → the built SPA loads
        let ok = Req::get("/").header("host", "127.0.0.1:8080").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(ok).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("real-app"), "the built index.html is served");

        // forged Host → 403 even for the SPA (the §2 layer wraps ServeDir too)
        let bad = Req::get("/").header("host", "attacker.com").body(Body::empty()).unwrap();
        assert_eq!(app.oneshot(bad).await.unwrap().status(), StatusCode::FORBIDDEN);
        let _ = std::fs::remove_dir_all(&dist);
    }

    /// Risk item #3 — the bursty `stream-line` path carries over the WS, end-to-end through a
    /// REAL server: handshake passes Host+Origin, first-frame bearer authenticates, the
    /// stream-line frame arrives with the expected payload.
    #[tokio::test]
    async fn ws_carries_stream_line_after_first_frame_auth() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::Message as TMsg;

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let sec = security::SecurityState { port, token: Arc::new("tok".to_string()) };
        let state = routes::AppState::discover(repo_root(), std::env::temp_dir()).unwrap();
        let app = build_app(sec, state, dummy_session(), None);
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        // connect with the required Origin header (Host is set by the client to 127.0.0.1:port)
        let mut req = format!("ws://127.0.0.1:{port}/api/events").into_client_request().unwrap();
        req.headers_mut()
            .insert("origin", format!("http://127.0.0.1:{port}").parse().unwrap());
        let (mut ws, _resp) = tokio_tungstenite::connect_async(req).await.unwrap();

        // first frame = the bearer; then the stream-line should arrive
        ws.send(TMsg::Text("tok".into())).await.unwrap();
        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.into_text().unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["event"], "stream-line");
        assert_eq!(v["payload"]["line"], "hello from the perimeter");
    }
}
