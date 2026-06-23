//! Token handoff that does not leak (spec §2.3 / ADR-0022 §3.3).
//!
//! 1. Mint a ≥256-bit `token` (rand 32 bytes) + a single-use, short-TTL launch `nonce`.
//! 2. Write `{port, token}` to `~/.opentrapp/viewer/session.json` at mode 0600 (assert the mode).
//! 3. The daemon opens the browser to a BARE loopback URL with the nonce in the URL *fragment*:
//!        http://127.0.0.1:<port>/#n=<nonce>
//!    The fragment is never sent to the server and never in `Referer`.
//! 4. Bootstrap JS reads `#n=`, calls `POST /api/session {nonce}`; the server validates+BURNS the
//!    nonce and returns the bearer, which it ALSO sets as `Set-Cookie: <name>=<bearer>;
//!    HttpOnly; SameSite=Strict; Path=/` (Secure omitted — loopback plaintext is correct, §2.3).
//! 5. JS clears the fragment (`history.replaceState`) and boots React. WS auth = first frame.
//! The long-lived token NEVER appears in URL / history / `Referer` / `argv`. TTL + revocation (§2.4).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Context;
use rand::RngCore;

/// Single-use launch-nonce TTL (short; the browser exchanges it immediately on load, §2.3).
const NONCE_TTL: Duration = Duration::from_secs(120);

pub struct Session {
    pub port: u16,
    token: String,
    nonce: Option<String>, // None once burned (single use)
    nonce_expires: Instant,
}

impl Session {
    /// Mint a fresh session — a ≥256-bit bearer + a single-use launch nonce — and persist
    /// `{port, token}` to a 0600 `session.json` under `dir`. Returns the session + the file path.
    pub fn mint_in(port: u16, dir: &Path) -> anyhow::Result<(Self, PathBuf)> {
        let token = random_hex(32); // 32 bytes = 256-bit
        let nonce = random_hex(16);
        let path = write_session_file(dir, port, &token)?;
        Ok((
            Self { port, token, nonce: Some(nonce), nonce_expires: Instant::now() + NONCE_TTL },
            path,
        ))
    }

    pub fn nonce(&self) -> Option<&str> {
        self.nonce.as_deref()
    }
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Validate + BURN the single-use launch nonce, returning the bearer exactly once.
    /// Any attempt consumes the nonce; returns `None` if it is wrong, expired, or already burned.
    pub fn exchange_nonce(&mut self, presented: &str, now: Instant) -> Option<String> {
        let n = self.nonce.take()?; // burn on ANY attempt — single use, even on failure
        if now > self.nonce_expires {
            return None;
        }
        if !crate::security::constant_time_eq(presented.as_bytes(), n.as_bytes()) {
            return None;
        }
        Some(self.token.clone())
    }

    /// Revoke the bearer on session end / idle (§2.4). After this no presented token matches.
    pub fn revoke(&mut self) {
        self.token.clear();
    }
    pub fn is_revoked(&self) -> bool {
        self.token.is_empty()
    }
}

fn random_hex(n_bytes: usize) -> String {
    let mut buf = vec![0u8; n_bytes];
    rand::rng().fill_bytes(&mut buf);
    let mut s = String::with_capacity(n_bytes * 2);
    for b in &buf {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Write `{port, token}` to `<dir>/session.json` at mode 0600 (owner-only) from creation —
/// never briefly world-readable. Asserts the mode is owner-only on Unix.
fn write_session_file(dir: &Path, port: u16, token: &str) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("create viewer session dir {dir:?}"))?;
    let path = dir.join("session.json");
    let json = format!("{{\"port\":{port},\"token\":\"{token}\"}}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600) // owner-only from creation
            .open(&path)
            .with_context(|| format!("create 0600 {path:?}"))?;
        f.write_all(json.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, json.as_bytes())?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("otv-test-{}", random_hex(6)))
    }

    #[test]
    fn token_is_at_least_256_bit_hex() {
        let t = random_hex(32);
        assert_eq!(t.len(), 64, "32 bytes = 64 hex chars = 256 bits");
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn nonce_exchange_returns_bearer_once_then_is_burned() {
        let dir = temp_dir();
        let (mut s, _) = Session::mint_in(7777, &dir).unwrap();
        let nonce = s.nonce().unwrap().to_string();
        let now = Instant::now();
        assert_eq!(s.exchange_nonce(&nonce, now), Some(s.token().to_string()));
        // already burned → None even with the right nonce
        assert_eq!(s.exchange_nonce(&nonce, now), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonce_exchange_rejects_wrong_value_and_burns_it() {
        let dir = temp_dir();
        let (mut s, _) = Session::mint_in(7777, &dir).unwrap();
        let real = s.nonce().unwrap().to_string();
        // a wrong guess burns the nonce, so even the real one then fails
        assert_eq!(s.exchange_nonce("deadbeefdeadbeef", Instant::now()), None);
        assert_eq!(s.exchange_nonce(&real, Instant::now()), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonce_exchange_rejects_expired() {
        let dir = temp_dir();
        let (mut s, _) = Session::mint_in(7777, &dir).unwrap();
        let nonce = s.nonce().unwrap().to_string();
        let way_past = Instant::now() + Duration::from_secs(10_000);
        assert_eq!(s.exchange_nonce(&nonce, way_past), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn session_file_is_owner_only_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir();
        let (_s, path) = Session::mint_in(7777, &dir).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "session.json must be owner-only 0600 (the token lives here)");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn revoke_clears_the_bearer() {
        let dir = temp_dir();
        let (mut s, _) = Session::mint_in(7777, &dir).unwrap();
        assert!(!s.is_revoked());
        s.revoke();
        assert!(s.is_revoked());
        fs::remove_dir_all(&dir).ok();
    }
}
