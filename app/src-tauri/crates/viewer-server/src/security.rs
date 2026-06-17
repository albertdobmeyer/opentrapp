//! SCAFFOLD — the loopback security middleware (spec §2 / ADR-0022 §3). THE RISKIEST PART.
//!
//! All controls are required; none is sufficient alone. Every CVE in the prior-art
//! (docs/de-tauri-viewer-research.md) is one of these omitted. Implement as a tower layer.

/// Build the tower middleware stack applied to every request + the WS handshake.
/// TODO: compose the checks below + tower-http SetResponseHeader(CSP), RequestBodyLimit, Timeout.
pub fn layer(/* session: &crate::session::Session */) /* -> impl tower::Layer<…> */ {
    todo!("compose: host_allowlist + origin_allowlist + bearer_check + csp + limits (spec §2)")
}

/// §2.2 — PRIMARY anti-DNS-rebinding control. Accept ONLY `127.0.0.1:<port>` / `localhost:<port>`.
/// Anything else (incl. a forged `Host: attacker.com`) → 403. Do NOT rely on browser PNA/LNA.
pub fn host_allowed(_host_header: &str, _port: u16) -> bool {
    todo!("exact match 127.0.0.1:<port> or localhost:<port>; else false (spec §2.2)")
}

/// §2.2 — Origin allowlist on every request AND the WS handshake. Missing/foreign Origin → 403.
pub fn origin_allowed(_origin_header: Option<&str>, _port: u16) -> bool {
    todo!("allow only the loopback origins for <port>; reject cross-origin (spec §2.2)")
}

/// §2.3 — bearer check. The long-lived token arrives as an HttpOnly+SameSite=Strict cookie
/// (after the /api/session nonce exchange) or, for the WS, in the first frame. NEVER from a
/// query string / URL (that is the leak we designed out). Tokenless → 401.
pub fn bearer_ok(_cookie_or_first_frame: Option<&str>, _expected: &str) -> bool {
    todo!("constant-time compare; honor TTL + revocation (spec §2.3/§2.4)")
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
