//! Secret redaction for log output. Moved from the GUI's `lifecycle.rs` into
//! core in Phase B (ADR-0019) so the orchestrator that PRODUCES the logs
//! (`podman.rs`) can redact them without reaching back into the GUI crate — the
//! one dependency-direction escape that the daemon split has to remove.

/// The replacement token substituted for a redacted secret value.
pub const REDACTED: &str = "<REDACTED>";

/// Redact known token-bearing environment variables from a string before it is
/// logged. `podman compose` echoes the full container-creation command on
/// failure, including `TELEGRAM_BOT_TOKEN=...` in cleartext — which would leak
/// into our log if surfaced verbatim. Mirrors the vault-proxy redaction pattern.
pub fn redact_secrets(s: &str) -> String {
    const SENSITIVE_VARS: &[&str] = &[
        "TELEGRAM_BOT_TOKEN",
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut out = s.to_string();
    for var in SENSITIVE_VARS {
        let needle = format!("{var}=");
        let mut search_from = 0;
        while let Some(rel) = out[search_from..].find(&needle) {
            let pos = search_from + rel;
            let after = pos + needle.len();
            let end = out[after..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|n| after + n)
                .unwrap_or(out.len());
            out.replace_range(after..end, REDACTED);
            search_from = after + REDACTED.len();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_telegram_bot_token() {
        let input = "podman run -e TELEGRAM_BOT_TOKEN=12345:abcdef -e FOO=bar ...";
        let out = redact_secrets(input);
        assert!(!out.contains("12345:abcdef"));
        assert!(out.contains("TELEGRAM_BOT_TOKEN=<REDACTED>"));
        assert!(out.contains("FOO=bar"));
    }

    #[test]
    fn redacts_multiple_occurrences_without_looping() {
        let input = "ANTHROPIC_API_KEY=sk-ant-aaa OPENAI_API_KEY=sk-bbb";
        let out = redact_secrets(input);
        assert!(!out.contains("sk-ant-aaa"));
        assert!(!out.contains("sk-bbb"));
        assert!(out.matches(REDACTED).count() == 2);
    }

    #[test]
    fn passes_through_unrelated_text() {
        let input = "exit 137: SIGKILL received";
        assert_eq!(redact_secrets(input), input);
    }
}
