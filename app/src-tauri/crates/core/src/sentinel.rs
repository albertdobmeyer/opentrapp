//! Sentinel judge machinery (lifted from the Tauri command layer — ADR-0022 migration step 1).
//!
//! The transport-neutral half of the Sentinel bridge: run the rung-2 local judge (`sentinel/judge.sh`)
//! on an opaque request and parse its `Verdict`. Moved into core so both the Tauri shim and the
//! future loopback web route can run a judgment by calling `judge_with(dir, request)` — neither has
//! to reimplement the bash spawn or the parse.
//!
//! The **fail-safe** contract is the security-load-bearing part and is preserved exactly: any infra
//! failure (lib/bash missing, bad serialise, timeout, unreadable output) yields an `escalate`
//! verdict — **never a silent `allow`** (`parse_verdict` + the `Verdict::escalate` paths).
//!
//! What stays GUI-side (`commands/sentinel.rs`): the activity indicator (`SentinelActivityStore` +
//! the Tauri `emit`), and lib *location* — `sentinel_candidates` bakes in a crate-relative
//! `CARGO_MANIFEST_DIR` dev fallback, so it must resolve against the GUI crate, not core. The GUI
//! `judge(app, ..)` locates the dir, drives the activity indicator, and delegates the run here.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;

use crate::util::shell::find_bash;

/// How long to wait on the local judge before giving up (matches the lib's own SENTINEL_TIMEOUT
/// default headroom; the judge is load-on-demand so a cold start can take a few seconds).
const JUDGE_TIMEOUT: Duration = Duration::from_secs(90);

/// The Verdict the lib returns (mirrors `sentinel/verdict-schema.json`). The `reason` is
/// user-facing plain language.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Verdict {
    pub decision: String,
    pub confidence: f64,
    pub resolved_at_rung: u8,
    pub reason: String,
}

impl Verdict {
    /// The fail-safe verdict when the lib can't be run or its output can't be parsed — mirrors
    /// judge.sh's own escalate-on-uncertainty contract so the caller decides its policy rather than
    /// defaulting to allow. Public so the GUI's lib-not-located path can return it too.
    pub fn escalate(reason: &str) -> Self {
        Self {
            decision: "escalate".to_string(),
            confidence: 0.0,
            resolved_at_rung: 2,
            reason: reason.to_string(),
        }
    }
}

/// Parse the lib's stdout into a Verdict. Pure + unit-testable. Any malformed output becomes an
/// `escalate` verdict (never a silent `allow`).
pub fn parse_verdict(stdout: &str) -> Verdict {
    let line = stdout.trim();
    if line.is_empty() {
        return Verdict::escalate("The local check produced no answer; please review manually.");
    }
    match serde_json::from_str::<Verdict>(line) {
        Ok(v) if matches!(v.decision.as_str(), "allow" | "block" | "escalate") => v,
        _ => Verdict::escalate(
            "The local check returned an unreadable answer; please review manually.",
        ),
    }
}

/// Run the rung-2 judge in an already-located lib `dir` on an opaque request, returning a
/// [`Verdict`]. Infra failures (bash missing, bad serialise) return an `escalate` verdict — never a
/// silent allow. Transport-neutral: the caller locates `dir` and drives any activity indicator.
pub async fn judge_with(dir: &Path, request: serde_json::Value) -> Verdict {
    let Some(bash) = find_bash() else {
        return Verdict::escalate(
            "Could not find a shell to run the local check; please review manually.",
        );
    };
    let request_json = match serde_json::to_string(&request) {
        Ok(s) => s,
        Err(_) => {
            return Verdict::escalate(
                "The local check input could not be prepared; please review manually.",
            )
        }
    };
    run_judge(&bash, &dir.to_path_buf(), &request_json).await
}

async fn run_judge(bash: &PathBuf, dir: &PathBuf, request_json: &str) -> Verdict {
    let spawn = TokioCommand::new(bash)
        .arg(dir.join("judge.sh"))
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match spawn {
        Ok(c) => c,
        Err(_) => return Verdict::escalate("The local check could not be started."),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request_json.as_bytes()).await;
        // Drop closes stdin so judge.sh's `cat` returns.
    }

    let out = match tokio::time::timeout(JUDGE_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(_)) => return Verdict::escalate("The local check failed to run."),
        Err(_) => return Verdict::escalate("The local check took too long and was stopped."),
    };

    parse_verdict(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_verdict_accepts_a_valid_verdict() {
        let v = parse_verdict(r#"{"decision":"block","confidence":0.9,"resolved_at_rung":2,"reason":"reads your saved passwords"}"#);
        assert_eq!(v.decision, "block");
        assert_eq!(v.resolved_at_rung, 2);
    }

    #[test]
    fn parse_verdict_escalates_on_empty() {
        assert_eq!(parse_verdict("").decision, "escalate");
        assert_eq!(parse_verdict("   \n").decision, "escalate");
    }

    #[test]
    fn parse_verdict_escalates_on_garbage_never_allows() {
        // A malformed answer must NEVER silently become allow.
        let v = parse_verdict("not json at all");
        assert_eq!(v.decision, "escalate");
        let v2 = parse_verdict(r#"{"decision":"yolo","confidence":1.0,"resolved_at_rung":2,"reason":"x"}"#);
        assert_eq!(v2.decision, "escalate");
    }
}
