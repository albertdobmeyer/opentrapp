//! SCAFFOLD — token handoff that does not leak (spec §2.3 / ADR-0022 §3.3).
//!
//! 1. Daemon mints a ≥256-bit `token` (rand 32 bytes) + a single-use, short-TTL launch `nonce`.
//! 2. Writes `{port, token}` to ~/.opentrapp/viewer/session.json at mode 0600 (assert the mode).
//! 3. Opens the browser to a BARE loopback URL with the nonce in the URL *fragment*:
//!        http://127.0.0.1:<port>/#n=<nonce>
//!    The fragment is never sent to the server and never in `Referer`.
//! 4. Bootstrap JS reads `#n=`, calls `POST /api/session {nonce}`, the server validates+burns the
//!    nonce and returns the bearer, which it ALSO sets as `Set-Cookie: <name>=<bearer>;
//!    HttpOnly; SameSite=Strict; Path=/` (Secure omitted — loopback plaintext is correct, §2.3).
//! 5. JS clears the fragment (`history.replaceState`) and boots React. WS auth = first frame.
//! The long-lived token NEVER appears in URL / history / `Referer` / `argv`. TTL + revocation (§2.4).

pub struct Session {
    pub port: u16,
    // token: secret,  nonce: single-use,  expires_at: Instant
}

impl Session {
    pub fn new(_port: u16) -> Self {
        todo!("mint token+nonce; write 0600 ~/.opentrapp/viewer/session.json (spec §2.3)")
    }
    // pub fn exchange_nonce(&self, nonce: &str) -> Option<Bearer> { /* validate + burn, once */ }
    // pub fn revoke(&self) { /* on session end / idle */ }
}

// pub fn router() -> axum::Router { Router::new().route("/api/session", post(exchange)) }

#![allow(dead_code)]
