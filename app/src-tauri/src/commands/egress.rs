//! Egress allowlist approvals (v0.6 Item A) — the human-mediated loosening surface.
//!
//! Surfaces the proxy's gray-zone off-allowlist blocks as **explained, one-tap
//! human decisions**. This is the only new write/loosening surface in v0.6, so
//! it is built to hold the ADR-0002 invariant by construction:
//!
//!   - `list_egress_approvals` is **read + recommend only** — it reads the log,
//!     asks the Sentinel judge, and returns pending items. It has no write path.
//!   - `apply_allowlist_decision` is the **only** allowlist writer, and only on
//!     an explicit human "always" tap. A "deny" never writes the allowlist.
//!   - Clear exfil (`EXFIL_BLOCKED`) and DNS-rebinding blocks are filtered out
//!     in `allowlist::parse_egress_blocks` — they never reach the judge here.
//!
//! Stays generic-backend-clean: `vault-proxy` is infra (no `component.yml`), so
//! this lives in the orchestrator's container-management layer, not the manifest
//! channel (spec 08 §3).

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;
use tauri::AppHandle;

use crate::commands::sentinel;
use crate::orchestrator::{allowlist, podman};

/// One gray-zone off-allowlist host awaiting a human decision, with the judge's
/// plain-language reason. `host`/`reason` render directly to the user — both are
/// treated as data (banned-vocabulary applies; the judge prompt is injection-hardened).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PendingApproval {
    pub host: String,
    pub reason: String,
    pub judged_at_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Read the proxy's egress log, find the gray-zone off-allowlist hosts (clear
/// exfil + rebinding already excluded), and ask the Sentinel judge about each.
/// Returns the ones worth a human look (the judge did not clearly block them),
/// each with its plain-language reason. Read-only: it can never loosen anything.
#[tauri::command]
pub async fn list_egress_approvals(app: AppHandle) -> Result<Vec<PendingApproval>, String> {
    let log = podman::read_egress_log();
    if log.trim().is_empty() {
        return Ok(Vec::new());
    }
    let allowed = allowlist::read_hosts(&allowlist::live_allowlist_path());
    let denied = allowlist::read_hosts(&allowlist::denials_path());
    let hosts = allowlist::parse_egress_blocks(&log, &allowed, &denied);

    let mut out = Vec::new();
    for host in hosts {
        let request = json!({
            "context": "egress_request",
            "fragment": host,
            "static_signal": "off_allowlist",
        });
        let verdict = sentinel::judge(&app, request).await;
        // A clear "block" from the judge stays blocked (logged) and is not
        // surfaced as approvable — this is the approval-fatigue guard (T5).
        // "allow" and "escalate" (genuinely uncertain) surface for the human,
        // who is the only one who can actually loosen.
        if verdict.decision == "block" {
            continue;
        }
        out.push(PendingApproval {
            host,
            reason: verdict.reason,
            judged_at_ms: now_ms(),
        });
    }
    Ok(out)
}

/// Apply a human decision on a gray-zone host. **The only allowlist writer.**
/// `"always"` persists the host + adds it to the live allowlist + signals the
/// proxy to reload. `"deny"` only remembers the denial (never writes the
/// allowlist). Validates the host strictly first — it originates from the log.
#[tauri::command]
pub async fn apply_allowlist_decision(host: String, decision: String) -> Result<(), String> {
    let host = host.trim().to_ascii_lowercase();
    if !allowlist::is_valid_host(&host) {
        return Err("That address doesn't look like a valid site name.".to_string());
    }
    match decision.as_str() {
        "always" => {
            allowlist::apply_always(&host).map_err(|e| format!("Could not save your choice: {e}"))?;
            // Make it take effect now. If the gate isn't running, the merged
            // file is read on next start — still applied, just not live yet.
            podman::reload_proxy_allowlist()
                .map_err(|e| format!("Saved your choice, but could not refresh the gate right now: {e}"))?;
            Ok(())
        }
        "deny" => allowlist::record_denial(&host).map_err(|e| format!("Could not save your choice: {e}")),
        _ => Err("Unknown decision.".to_string()),
    }
}
