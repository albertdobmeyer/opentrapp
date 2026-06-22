//! Diagnostic bundle generation (lifted from the Tauri command layer — ADR-0022 migration
//! step 1 — so both the Tauri command shim AND the on-demand loopback web GUI call the same
//! transport-neutral fn, never a duplicate).
//!
//! Collects app + system info into a single REDACTED text blob the user can paste into a
//! support email or GitHub issue without leaking secrets. The redaction is the security-critical
//! piece: anything that looks like an Anthropic key, Telegram bot token, IP address, home path,
//! or username is rewritten before the string leaves this function.

use regex::Regex;
use std::process::Command;
use std::sync::OnceLock;

/// Generate a freshly collected, redacted diagnostic bundle. `generated_at` (an ISO-8601 string)
/// and `app_version` are injected by the caller, so core stays clock- and transport-neutral
/// (the Tauri shim passes `chrono::Utc::now()` + its `CARGO_PKG_VERSION`; the web GUI route
/// passes its own).
pub fn generate_bundle(generated_at: &str, app_version: &str) -> Result<String, String> {
    let mut out = String::new();

    out.push_str("=== OPENTRAPP DIAGNOSTICS ===\n");
    out.push_str(&format!("Generated: {generated_at}\n"));
    out.push_str(&format!("App version: {app_version}\n"));
    out.push_str(&format!("OS: {}\n", std::env::consts::OS));
    out.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    out.push('\n');

    out.push_str("=== CONTAINER STATUS ===\n");
    out.push_str(&collect_container_status());
    out.push('\n');

    out.push_str("=== ENVIRONMENT ===\n");
    out.push_str(&collect_runtime_versions());
    out.push('\n');

    out.push_str("=== NOT INCLUDED (by design) ===\n");
    out.push_str(
        "- API keys\n\
         - Telegram bot token\n\
         - User's workspace files\n\
         - Agent conversation history\n\
         - IP addresses\n\
         - Username\n",
    );

    Ok(redact(&out))
}

fn collect_container_status() -> String {
    // Try podman first, then docker; surface unavailability cleanly. Filter by the
    // `com.docker.compose.service` label so we don't depend on the compose project name
    // (which is directory-derived and varies by install).
    const SERVICES: [&str; 5] =
        ["vault-agent", "vault-proxy", "vault-egress", "vault-skills", "vault-social"];
    for tool in &["podman", "docker"] {
        let mut lines: Vec<String> = Vec::new();
        let mut tool_ok = false;
        for service in SERVICES {
            let out = Command::new(tool)
                .args([
                    "ps",
                    "-a",
                    "--filter",
                    &format!("label=com.docker.compose.service={}", service),
                    "--format",
                    "{{.Names}}\t{{.Status}}",
                ])
                .output();
            if let Ok(o) = out {
                if o.status.success() {
                    tool_ok = true;
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
                        lines.push(line.to_string());
                    }
                }
            }
        }
        if tool_ok {
            if lines.is_empty() {
                return format!("{}: no perimeter containers found\n", tool);
            }
            return format!("{}:\n{}\n", tool, lines.join("\n"));
        }
    }
    "Container runtime not detected (Podman or Docker missing).\n".to_string()
}

fn collect_runtime_versions() -> String {
    let mut s = String::new();
    for (label, cmd, args) in &[
        ("podman", "podman", vec!["--version"]),
        ("docker", "docker", vec!["--version"]),
        ("git", "git", vec!["--version"]),
    ] {
        match Command::new(cmd).args(args).output() {
            Ok(o) if o.status.success() => {
                s.push_str(&format!("{}: {}", label, String::from_utf8_lossy(&o.stdout)));
            }
            _ => {
                s.push_str(&format!("{}: not available\n", label));
            }
        }
    }
    s
}

// ─── Redaction (security-critical) ───────────────────────────────────────────

fn anthropic_key_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"sk-ant-[A-Za-z0-9_\-]+").unwrap())
}

fn telegram_token_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\b\d{6,12}:[A-Za-z0-9_\-]{30,}\b").unwrap())
}

fn ipv4_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    // Avoid matching version numbers like "0.1.0" by requiring 4 octets.
    R.get_or_init(|| Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap())
}

fn ipv6_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b").unwrap())
}

/// Redact secrets/PII from a text blob. Public for unit tests and external callers.
pub fn redact(input: &str) -> String {
    let mut s = input.to_string();

    // Order matters: redact tokens before paths so a token that contains a home-style
    // substring is still caught as a token.
    s = anthropic_key_re().replace_all(&s, "[REDACTED_ANTHROPIC_KEY]").into_owned();
    s = telegram_token_re().replace_all(&s, "[REDACTED_TELEGRAM_TOKEN]").into_owned();
    s = ipv4_re().replace_all(&s, "[REDACTED_IP]").into_owned();
    s = ipv6_re().replace_all(&s, "[REDACTED_IP]").into_owned();

    // Replace home directory paths with `~`. The process knows its own $HOME; any other
    // user's path still gets its username segment caught by the explicit USER check below.
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() && home != "/" {
            s = s.replace(&home, "~");
        }
    }
    if let Ok(user) = std::env::var("USER") {
        if !user.is_empty() {
            let pat = Regex::new(&format!(r"\b{}\b", regex::escape(&user))).unwrap();
            s = pat.replace_all(&s, "[user]").into_owned();
        }
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_anthropic_key() {
        let out = redact("Header: sk-ant-api03-AbCdEf-12345_xyz body");
        assert!(out.contains("[REDACTED_ANTHROPIC_KEY]"));
        assert!(!out.contains("sk-ant-api03"));
    }

    #[test]
    fn redacts_telegram_token() {
        let out = redact("Token=1234567890:ABCdefGHIjklMNOpqrSTUvwxYZ_-1234567890 trailing");
        assert!(out.contains("[REDACTED_TELEGRAM_TOKEN]"));
        assert!(!out.contains("1234567890:ABC"));
    }

    #[test]
    fn redacts_ipv4() {
        let out = redact("Server: 192.168.1.42 reached at 10.0.0.1");
        assert!(out.contains("[REDACTED_IP]"));
        assert!(!out.contains("192.168.1.42"));
    }

    #[test]
    fn does_not_redact_version_strings() {
        // Only 3 octets — should NOT be matched by the IPv4 regex.
        let out = redact("App version: 0.1.0 (build 2)");
        assert!(out.contains("0.1.0"));
    }

    #[test]
    fn redacts_multiple_secrets_in_one_blob() {
        let out = redact(
            "k=sk-ant-api03-XYZ tg=1234567890:ABCdefGHIjklMNOpqrSTUvwxYZ_-1234567890 ip=8.8.8.8",
        );
        assert!(out.contains("[REDACTED_ANTHROPIC_KEY]"));
        assert!(out.contains("[REDACTED_TELEGRAM_TOKEN]"));
        assert!(out.contains("[REDACTED_IP]"));
    }

    #[test]
    fn passes_through_unrelated_text() {
        let input = "Some perfectly normal log line with no secrets.";
        assert_eq!(redact(input), input);
    }

    #[test]
    fn redacts_home_path_when_home_is_set() {
        std::env::set_var("HOME", "/home/testuser");
        let out = redact("Read failed at /home/testuser/projects/opentrapp/foo");
        assert!(out.contains("~/projects/opentrapp/foo"));
        assert!(!out.contains("/home/testuser"));
    }

    #[test]
    fn generate_bundle_includes_injected_header_and_redacts() {
        // injected timestamp + version appear; structure is present; a stray key would be redacted.
        let b = generate_bundle("2026-06-22T00:00:00Z", "9.9.9").unwrap();
        assert!(b.contains("Generated: 2026-06-22T00:00:00Z"));
        assert!(b.contains("App version: 9.9.9"));
        assert!(b.contains("=== OPENTRAPP DIAGNOSTICS ==="));
        assert!(b.contains("=== NOT INCLUDED (by design) ==="));
    }
}
