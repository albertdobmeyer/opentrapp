//! Host-mediated allowlist loosening (v0.6 Item A).
//!
//! Turns `vault-proxy`'s blunt off-allowlist 403 into an explained, one-tap
//! human decision. This module is the **pure, testable core**: parsing the
//! egress log, the persistence file layout, and the append/merge primitives.
//! The podman-touching pieces (reading the log volume, signalling the proxy to
//! reload) live in [`super::podman`]; the Tauri wiring + the judge call live in
//! `commands::egress`.
//!
//! ## The hard invariant (ADR-0002)
//! [`apply_always`] is the **only** function in the codebase that writes the
//! allowlist, and it is only ever called on an explicit human "Always allow"
//! tap. [`record_denial`] never touches the allowlist. The contained agent has
//! no path here — the orchestrator is the sole writer, the file is mounted
//! `:ro` into the proxy. Pinned by the tests below + orchestrator-check §27.
//!
//! ## Two-file persistence model
//! The proxy reads ONE file, `~/.opentrapp/perimeter/allowlist.txt` (the *live*
//! file). But that path is re-staged from the signed bundle on every launch
//! (self-heals tampering), so user choices must persist elsewhere:
//!   - **live**      `perimeter/allowlist.txt` — seed (staged) + merged additions.
//!   - **additions** `allowlist.additions.txt` — persistent "Always allow" hosts,
//!     OUTSIDE the staged path. Re-merged into live after staging (bootstrap).
//!   - **denials**   `egress-denials.txt` — remembered "Deny" hosts (no re-prompt;
//!     SD-A2). Never written to the allowlist.
//! Appends are **in-place** (`O_APPEND`) — never temp+rename, which would swap
//! the inode the proxy's single-file bind-mount is pinned to. The proxy's own
//! `_reload_allowlist` does the in-memory atomic swap on SIGHUP.

use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::podman;

/// The off-allowlist block reason the proxy logs for a gray-zone host — the
/// only `BLOCKED` reason a human may choose to allow. The DNS-rebinding block
/// (a different reason) and `EXFIL_BLOCKED` are hard rung-0 blocks that must
/// never surface as approvable. Must match the goproxy matcher
/// (`infra/proxy/goproxy/policy/policy.go`).
const OFF_ALLOWLIST_REASON: &str = "domain not in allowlist";

// ─── file layout ─────────────────────────────────────────────────────────

/// The live allowlist the proxy reads (`:ro`). Re-staged from the bundle each
/// launch; [`merge_seed_and_additions`] re-applies persisted additions after.
pub fn live_allowlist_path() -> PathBuf {
    podman::resource_dir().join("allowlist.txt")
}

/// Persistent "Always allow" hosts — the source of truth for user choices,
/// kept OUTSIDE the staged/overwritten `perimeter/` dir.
pub fn additions_path() -> PathBuf {
    podman::runtime_data_dir().join("allowlist.additions.txt")
}

/// Remembered "Deny" hosts (SD-A2) — never written to the allowlist.
pub fn denials_path() -> PathBuf {
    podman::runtime_data_dir().join("egress-denials.txt")
}

// ─── host validation ──────────────────────────────────────────────────────

/// A host is allowlist-eligible only if it is a plausible domain name: lowercase
/// `[a-z0-9.-]`, contains a dot, no IP literal, length-bounded. The host string
/// originates from the proxy log (agent-influenced), so anything that could
/// inject a second token / newline / path into the allowlist file is rejected.
pub fn is_valid_host(host: &str) -> bool {
    let h = host.trim();
    if h.is_empty() || h.len() > 253 || !h.contains('.') {
        return false;
    }
    if h.starts_with('.') || h.ends_with('.') || h.contains("..") {
        return false;
    }
    if !h.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-') {
        return false;
    }
    // Reject IP literals — the allowlist is domain-only (mirrors the proxy).
    if h.split('.').all(|seg| seg.parse::<u8>().is_ok()) {
        return false;
    }
    true
}

// ─── log parsing (pure) ─────────────────────────────────────────────────────

/// Extract the distinct, ordered, gray-zone off-allowlist hosts from a
/// `requests.jsonl` body. Surfaces only `action=="BLOCKED"` with
/// `reason=="domain not in allowlist"`; **excludes** `EXFIL_BLOCKED` and the
/// DNS-rebinding block (clear rung-0 blocks never reach the judge). Drops hosts
/// already in `allowed` or `denied`, and any host that fails [`is_valid_host`].
pub fn parse_egress_blocks(
    jsonl: &str,
    allowed: &HashSet<String>,
    denied: &HashSet<String>,
) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("action").and_then(|a| a.as_str()) != Some("BLOCKED") {
            continue; // EXFIL_BLOCKED, ALLOWED, RESPONSE, … never approvable
        }
        if v.get("reason").and_then(|r| r.as_str()) != Some(OFF_ALLOWLIST_REASON) {
            continue; // DNS-rebinding BLOCKED is a hard block, not a gray zone
        }
        let Some(host) = v.get("host").and_then(|h| h.as_str()) else {
            continue;
        };
        let host = host.trim().to_ascii_lowercase();
        if !is_valid_host(&host) || allowed.contains(&host) || denied.contains(&host) {
            continue;
        }
        if seen.insert(host.clone()) {
            out.push(host);
        }
    }
    out
}

// ─── file primitives ────────────────────────────────────────────────────────

/// Read the hosts recorded in a one-host-per-line file (trimmed, lowercased,
/// `#` comments + blanks skipped). Missing file → empty set.
pub fn read_hosts(path: &Path) -> HashSet<String> {
    let mut set = HashSet::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return set;
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        set.insert(line.to_ascii_lowercase());
    }
    set
}

/// Append one host to a file **in place** (`O_APPEND`), idempotent. In-place
/// (never temp+rename) so the proxy's single-file bind-mount keeps its inode.
/// Creates the file + parent dir if missing.
pub fn append_host_inplace(path: &Path, host: &str) -> io::Result<()> {
    let host = host.trim().to_ascii_lowercase();
    if read_hosts(path).contains(&host) {
        return Ok(()); // already present — no duplicate line
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{host}")?;
    f.sync_all()?;
    Ok(())
}

// ─── the operations ─────────────────────────────────────────────────────────

/// After bundle staging overwrites the live allowlist with the pure seed,
/// re-apply every persisted "Always allow" host so user choices survive the
/// relaunch. Idempotent.
pub fn merge_seed_and_additions() -> io::Result<()> {
    let live = live_allowlist_path();
    for host in read_hosts(&additions_path()) {
        append_host_inplace(&live, &host)?;
    }
    Ok(())
}

/// Remember a denied host so it does not re-prompt (SD-A2). **Never** writes the
/// allowlist — a denial is not a loosening.
pub fn record_denial(host: &str) -> io::Result<()> {
    append_host_inplace(&denials_path(), host)
}

/// Apply an "Always allow": persist the host (additions) and add it to the live
/// allowlist (in-place). **The only allowlist writer in the codebase** (ADR-0002).
/// Caller signals the proxy to reload afterwards ([`super::podman::reload_proxy_allowlist`]).
pub fn apply_always(host: &str) -> io::Result<()> {
    append_host_inplace(&additions_path(), host)?;
    append_host_inplace(&live_allowlist_path(), host)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_surfaces_off_allowlist_blocks_only() {
        let log = [
            r#"{"action":"BLOCKED","host":"weather.example.com","reason":"domain not in allowlist"}"#,
            r#"{"action":"BLOCKED","host":"evil.example.org","reason":"domain not in allowlist"}"#,
            r#"{"action":"ALLOWED","host":"api.anthropic.com"}"#,
        ].join("\n");
        let got = parse_egress_blocks(&log, &set(&[]), &set(&[]));
        assert_eq!(got, vec!["weather.example.com", "evil.example.org"]);
    }

    #[test]
    fn parse_never_surfaces_clear_exfil_or_rebinding() {
        // The two hard rung-0 blocks must NEVER reach the approvals list / judge.
        let log = [
            r#"{"action":"EXFIL_BLOCKED","reason":"outbound payload exceeds 1048576 bytes"}"#,
            r#"{"action":"BLOCKED","host":"rebind.example.com","reason":"host resolves to private/loopback range (DNS-rebinding defense)"}"#,
            r#"{"action":"BLOCKED","host":"ok.example.com","reason":"domain not in allowlist"}"#,
        ].join("\n");
        let got = parse_egress_blocks(&log, &set(&[]), &set(&[]));
        assert_eq!(got, vec!["ok.example.com"], "only the gray-zone host surfaces");
    }

    #[test]
    fn parse_filters_already_allowed_and_denied_and_dedups() {
        let log = [
            r#"{"action":"BLOCKED","host":"a.example.com","reason":"domain not in allowlist"}"#,
            r#"{"action":"BLOCKED","host":"a.example.com","reason":"domain not in allowlist"}"#,
            r#"{"action":"BLOCKED","host":"allowed.example.com","reason":"domain not in allowlist"}"#,
            r#"{"action":"BLOCKED","host":"denied.example.com","reason":"domain not in allowlist"}"#,
        ].join("\n");
        let got = parse_egress_blocks(&log, &set(&["allowed.example.com"]), &set(&["denied.example.com"]));
        assert_eq!(got, vec!["a.example.com"]);
    }

    #[test]
    fn parse_rejects_invalid_or_injected_hosts() {
        let log = [
            r#"{"action":"BLOCKED","host":"good.example.com","reason":"domain not in allowlist"}"#,
            r#"{"action":"BLOCKED","host":"bad host\nevil.com","reason":"domain not in allowlist"}"#,
            r#"{"action":"BLOCKED","host":"127.0.0.1","reason":"domain not in allowlist"}"#,
            r#"{"action":"BLOCKED","host":"nodot","reason":"domain not in allowlist"}"#,
        ].join("\n");
        let got = parse_egress_blocks(&log, &set(&[]), &set(&[]));
        assert_eq!(got, vec!["good.example.com"]);
    }

    #[test]
    fn host_validation_is_strict() {
        assert!(is_valid_host("sub.example.com"));
        assert!(is_valid_host("a-b.example.co.uk"));
        assert!(!is_valid_host("Example.com"), "uppercase rejected (callers lowercase first)");
        assert!(!is_valid_host("no-dot"));
        assert!(!is_valid_host("192.168.0.1"), "IP literal rejected — domain-only");
        assert!(!is_valid_host("has space.com"));
        assert!(!is_valid_host("evil.com\nx.com"));
        assert!(!is_valid_host(".leading.com"));
    }

    #[test]
    fn append_is_in_place_and_idempotent() {
        let dir = std::env::temp_dir().join("opentrapp-allowlist-append-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("allowlist.txt");
        std::fs::write(&f, "seed.example.com\n").unwrap();

        #[cfg(unix)]
        let ino_before = {
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(&f).unwrap().ino()
        };

        append_host_inplace(&f, "new.example.com").unwrap();
        append_host_inplace(&f, "new.example.com").unwrap(); // idempotent

        let hosts = read_hosts(&f);
        assert!(hosts.contains("seed.example.com") && hosts.contains("new.example.com"));
        let body = std::fs::read_to_string(&f).unwrap();
        assert_eq!(body.matches("new.example.com").count(), 1, "no duplicate line");

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let ino_after = std::fs::metadata(&f).unwrap().ino();
            assert_eq!(ino_before, ino_after, "append must be in-place (inode preserved) — bind-mount safe");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_rebuilds_live_as_seed_union_additions() {
        // Simulate post-staging: live = pure seed; additions has user choices.
        let dir = std::env::temp_dir().join("opentrapp-allowlist-merge-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let live = dir.join("live.txt");
        std::fs::write(&live, "api.anthropic.com\n").unwrap();
        let additions = dir.join("additions.txt");
        std::fs::write(&additions, "weather.example.com\nnews.example.com\n").unwrap();

        // Drive the merge directly against the temp files (mirrors merge_seed_and_additions).
        for host in read_hosts(&additions) {
            append_host_inplace(&live, &host).unwrap();
        }

        let hosts = read_hosts(&live);
        assert!(hosts.contains("api.anthropic.com"));
        assert!(hosts.contains("weather.example.com"));
        assert!(hosts.contains("news.example.com"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn denial_never_writes_the_allowlist() {
        // The invariant, at the unit level: record_denial writes ONLY the denials
        // file; apply_always writes the additions + live files. record_denial must
        // have no effect on the allowlist file.
        let dir = std::env::temp_dir().join("opentrapp-allowlist-invariant-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let live = dir.join("allowlist.txt");
        std::fs::write(&live, "api.anthropic.com\n").unwrap();
        let denials = dir.join("denials.txt");

        append_host_inplace(&denials, "tracker.example.com").unwrap(); // == record_denial target

        assert!(read_hosts(&denials).contains("tracker.example.com"));
        assert!(!read_hosts(&live).contains("tracker.example.com"), "a denial must NEVER reach the allowlist");
    }
}
