//! SCAFFOLD — the loopback security middleware (spec §2 / ADR-0022 §3). THE RISKIEST PART.
//!
//! All controls are required; none is sufficient alone. Every CVE in the prior-art
//! (docs/de-tauri-viewer-research.md) is one of these omitted. Implemented as axum middleware.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Shared state for the §2 middleware: our loopback port + the expected bearer token.
#[derive(Clone)]
pub struct SecurityState {
    pub port: u16,
    pub token: Arc<String>,
}

/// The §2 security middleware (every request + the WS handshake). Order:
///   `Host` allowlist (ALL requests — primary anti-rebinding) → for `/api/*`: `Origin` allowlist
///   + bearer (except the `/api/session` bootstrap exchange). The SPA shell + static assets are
///   `Host`-checked only — they carry no secret; the bearer guards the `/api` surface.
pub async fn enforce(State(st): State<SecurityState>, req: Request, next: Next) -> Response {
    let headers = req.headers();

    // §2.2 — Host allowlist on EVERY request (primary anti-DNS-rebinding control).
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
    if !host_allowed(host, st.port) {
        return (StatusCode::FORBIDDEN, "forbidden host").into_response();
    }

    let path = req.uri().path();
    if path.starts_with("/api/") {
        // §2.2 — Origin allowlist on the API surface (incl. the WS handshake + /api/session).
        let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
        if !origin_allowed(origin, st.port) {
            return (StatusCode::FORBIDDEN, "forbidden origin").into_response();
        }
        // §2.3 — bearer required on the API surface EXCEPT the nonce-exchange bootstrap
        // (/api/session) and the WS handshake (/api/events authenticates in its FIRST FRAME).
        let bearer_exempt = path == "/api/session" || path == "/api/events";
        if !bearer_exempt && !bearer_ok(cookie_bearer(headers).as_deref(), st.token.as_str()) {
            return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
        }
    }

    next.run(req).await
}

/// Extract the bearer from the HttpOnly session cookie (`otv_bearer`, §2.3). Never a query string.
fn cookie_bearer(headers: &header::HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|kv| kv.trim().strip_prefix("otv_bearer="))
        .next()
        .map(str::to_string)
}

/// §2.2 — PRIMARY anti-DNS-rebinding control. Accept ONLY `127.0.0.1:<port>` / `localhost:<port>`.
/// Anything else (incl. a forged `Host: attacker.com`) → 403. Do NOT rely on browser PNA/LNA.
pub fn host_allowed(host_header: &str, port: u16) -> bool {
    // Exact match only. A forged / DNS-rebinding `Host` (attacker.com, a suffix trick like
    // `127.0.0.1.attacker.com`, the wrong port, or a missing port) is rejected. Loopback
    // bind alone is necessary-but-insufficient (§2.2), so this is the primary control.
    host_header == format!("127.0.0.1:{port}") || host_header == format!("localhost:{port}")
}

/// §2.2 — Origin allowlist on every request AND the WS handshake. Missing/foreign Origin → 403.
pub fn origin_allowed(origin_header: Option<&str>, port: u16) -> bool {
    // Allow ONLY the loopback origin for our port (plaintext http on loopback is correct,
    // §2.4 — TLS on 127.0.0.1 adds a cert-trust problem for no threat reduction). Missing
    // or cross-origin → reject. Applied to API requests and the WS handshake.
    matches!(
        origin_header,
        Some(o) if o == format!("http://127.0.0.1:{port}") || o == format!("http://localhost:{port}")
    )
}

/// §2.3 — bearer check. The long-lived token arrives as an HttpOnly+SameSite=Strict cookie
/// (after the /api/session nonce exchange) or, for the WS, in the first frame. NEVER from a
/// query string / URL (that is the leak we designed out). Tokenless → 401.
pub fn bearer_ok(cookie_or_first_frame: Option<&str>, expected: &str) -> bool {
    // Constant-time value compare. TTL + revocation are enforced by the session layer (the
    // caller passes the live `expected`, or None once revoked); a tokenless request fails here.
    match cookie_or_first_frame {
        Some(presented) => constant_time_eq(presented.as_bytes(), expected.as_bytes()),
        None => false,
    }
}

/// Length-checked constant-time byte comparison (no content-timing side channel; the length
/// itself is not secret — the token length is fixed).
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// §2.6 (recommended) — drop foreign-UID peers via SO_PEERCRED/LOCAL_PEERCRED.
/// Document the residual: an unauthenticated same-host, same-UID user can still probe `/`.
pub fn peer_is_same_uid(/* conn */) -> bool {
    todo!("SO_PEERCRED on Linux; LOCAL_PEERCRED on macOS/BSD; best-effort on Windows")
}

// §2.5 — LIVE VERIFICATION (mandatory, every (re)start; fold into boundary-selftest, not just cold):
//   - a forged `Host` header is rejected (403)
//   - a tokenless request is rejected (401)
//   - the bound address is loopback
//   - a DNS-rebinding-style `Host` is rejected (403)
//   - the secret never appears in any log line
// Expose these as a `pub fn self_verify(addr, token) -> Result<()>` the daemon calls on start.
pub fn self_verify(/* addr, token */) -> Result<(), String> {
    todo!("the VS Code --connection-token-file-was-silently-broken lesson (spec §2.5)")
}

#[cfg(test)]
mod tests {
    use super::*;
    const PORT: u16 = 8080;

    #[test]
    fn host_allowlist_accepts_only_loopback_with_exact_port() {
        assert!(host_allowed("127.0.0.1:8080", PORT));
        assert!(host_allowed("localhost:8080", PORT));
    }

    #[test]
    fn host_allowlist_rejects_forged_and_rebinding_hosts() {
        // The whole point of §2.2: a DNS-rebinding / forged Host must be 403.
        for bad in [
            "attacker.com",
            "attacker.com:8080",
            "evil.localhost:8080",          // not exactly localhost
            "127.0.0.1.attacker.com:8080",  // suffix trick
            "localhost.attacker.com:8080",
            "127.0.0.1:9999",               // wrong port
            "127.0.0.1",                    // no port
            "0.0.0.0:8080",
            "[::1]:8080",                   // not in the allowlist (we bind 127.0.0.1)
            "",
        ] {
            assert!(!host_allowed(bad, PORT), "must reject forged Host: {bad:?}");
        }
    }

    #[test]
    fn origin_allowlist_accepts_only_loopback_origin_for_port() {
        assert!(origin_allowed(Some("http://127.0.0.1:8080"), PORT));
        assert!(origin_allowed(Some("http://localhost:8080"), PORT));
    }

    #[test]
    fn origin_allowlist_rejects_missing_and_cross_origin() {
        for bad in [
            None,
            Some("http://attacker.com"),
            Some("http://127.0.0.1:9999"),  // wrong port
            Some("https://127.0.0.1:8080"), // wrong scheme (loopback is plaintext)
            Some("null"),
            Some("http://evil.localhost:8080"),
        ] {
            assert!(!origin_allowed(bad, PORT), "must reject Origin: {bad:?}");
        }
    }

    #[test]
    fn bearer_requires_exact_token_and_rejects_tokenless() {
        let expected = "sek-256bit-0123456789abcdef0123456789abcdef";
        assert!(bearer_ok(Some(expected), expected));
        assert!(!bearer_ok(None, expected), "tokenless must be 401");
        assert!(!bearer_ok(Some("wrong"), expected));
        assert!(!bearer_ok(Some(""), expected));
        // off-by-one-byte (length mismatch) must fail
        assert!(!bearer_ok(Some("sek-256bit-0123456789abcdef0123456789abcde"), expected));
    }
}

/// §2.5 LIVE verification — the security model end-to-end through a real axum router (not just
/// the pure checks). This IS the spike's SUCCESS criterion #3: forged Host → 403, tokenless →
/// 401, cross-origin → 403, while the SPA shell loads and the bootstrap exchange works.
#[cfg(test)]
mod mw_tests {
    use super::*;
    use axum::http::Request as Req;
    use axum::{
        body::Body,
        routing::{get, post},
        Router,
    };
    use tower::ServiceExt; // oneshot

    fn app(port: u16, token: &str) -> Router {
        let st = SecurityState { port, token: Arc::new(token.to_string()) };
        Router::new()
            .route("/", get(|| async { "spa-shell" }))
            .route("/api/get_status", post(|| async { "ok" }))
            .route("/api/session", post(|| async { "exchanged" }))
            .layer(axum::middleware::from_fn_with_state(st, enforce))
    }

    async fn code(app: Router, req: Req<Body>) -> StatusCode {
        app.oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn spa_shell_loads_with_valid_host_and_no_bearer() {
        let r = Req::get("/").header("host", "127.0.0.1:8080").body(Body::empty()).unwrap();
        assert_eq!(code(app(8080, "tok"), r).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn forged_and_rebinding_host_rejected_403() {
        let r = Req::get("/").header("host", "attacker.com").body(Body::empty()).unwrap();
        assert_eq!(code(app(8080, "tok"), r).await, StatusCode::FORBIDDEN);
        let r2 = Req::get("/").header("host", "evil.com:8080").body(Body::empty()).unwrap();
        assert_eq!(code(app(8080, "tok"), r2).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_tokenless_rejected_401() {
        let r = Req::post("/api/get_status")
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://127.0.0.1:8080")
            .body(Body::empty())
            .unwrap();
        assert_eq!(code(app(8080, "tok"), r).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_cross_origin_rejected_403_even_with_cookie() {
        let r = Req::post("/api/get_status")
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://attacker.com")
            .header("cookie", "otv_bearer=tok")
            .body(Body::empty())
            .unwrap();
        assert_eq!(code(app(8080, "tok"), r).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_with_valid_host_origin_and_bearer_passes() {
        let r = Req::post("/api/get_status")
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://127.0.0.1:8080")
            .header("cookie", "otv_bearer=tok")
            .body(Body::empty())
            .unwrap();
        assert_eq!(code(app(8080, "tok"), r).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn session_exchange_needs_origin_but_no_bearer_yet_blocks_forged_host() {
        // bootstrap exchange: valid host + origin, NO bearer yet → reaches the handler
        let ok = Req::post("/api/session")
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://127.0.0.1:8080")
            .body(Body::empty())
            .unwrap();
        assert_eq!(code(app(8080, "tok"), ok).await, StatusCode::OK);
        // but a forged Host blocks even the exchange
        let bad = Req::post("/api/session")
            .header("host", "attacker.com")
            .header("origin", "http://127.0.0.1:8080")
            .body(Body::empty())
            .unwrap();
        assert_eq!(code(app(8080, "tok"), bad).await, StatusCode::FORBIDDEN);
    }
}
