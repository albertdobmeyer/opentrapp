//! ADR-0021 — the pending boundary-weakening approval queue.
//!
//! Generalizes the ADR-0016 "sole writer, no agent call edge" pattern from the
//! allowlist to *every* boundary-weakening control op. The supervisor never
//! applies a weakening request that arrived on an agent-reachable transport (the
//! `~/.opentrapp/control` inbox is a file drop any host process can write); it
//! **enqueues** it here. The op is applied only when the human approves it on the
//! out-of-band approval surface (the GUI two-tap, ADR-0016) — `take_approved` is
//! the single edge from "pending" to "apply", reachable only from that surface,
//! never from the inbox-drain path.
//!
//! Enqueuing a request is not approving it: a host process that writes a fake
//! pending entry only causes the human to see (and reject) a spurious prompt — it
//! can never self-approve, because approval is the out-of-band tap (ADR-0021 §2).

use std::path::{Path, PathBuf};

use crate::control::ControlRequest;

/// Where pending weakening approvals live. Distinct from the `control` inbox so
/// the two flows (agent-reachable requests vs. human approvals) never alias.
fn queue_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("pending-approvals")
}

/// Enqueue a boundary-weakening request for out-of-band human approval. Returns
/// the opaque id the approval surface refers to it by. Does NOT apply anything.
pub fn enqueue(data_dir: &Path, req: ControlRequest) -> std::io::Result<String> {
    let dir = queue_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    // Stable, sortable id; the inbox already guarantees per-process uniqueness via
    // its sequence, but here the verb token + a content hash of existing entries
    // is enough — we key by token so a repeated request collapses to one prompt.
    let id = req.as_token().to_string();
    std::fs::write(dir.join(format!("{id}.pending")), req.as_token())?;
    Ok(id)
}

/// The pending weakening requests awaiting approval (for the approval surface to
/// render). Order is unspecified.
pub fn list(data_dir: &Path) -> Vec<ControlRequest> {
    let dir = queue_dir(data_dir);
    let rd = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };
    rd.filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "pending").unwrap_or(false))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|t| ControlRequest::from_token(&t))
        .collect()
}

/// **The single edge from "pending" to "apply."** Consume an approved pending
/// request, returning it for the caller (the supervisor, on the human-approval
/// path) to apply. Removes it from the queue. Returns `None` if no such pending
/// request exists. This is the ADR-0021 analogue of `allowlist::apply_always`:
/// it must be called ONLY from the out-of-band approval surface, never from the
/// agent-reachable inbox-drain path — pinned by tests + orchestrator-check.
pub fn take_approved(data_dir: &Path, id: &str) -> Option<ControlRequest> {
    let path = queue_dir(data_dir).join(format!("{id}.pending"));
    let token = std::fs::read_to_string(&path).ok()?;
    let req = ControlRequest::from_token(&token)?;
    let _ = std::fs::remove_file(&path);
    Some(req)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opentrapp-approvals-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn enqueue_holds_without_applying_and_approval_takes_it() {
        let d = temp("hold");
        // a weakening request is held, not applied
        let id = enqueue(&d, ControlRequest::Pause).unwrap();
        assert_eq!(list(&d), vec![ControlRequest::Pause], "Pause is pending approval");
        // only the approval path yields it back for application
        assert_eq!(take_approved(&d, &id), Some(ControlRequest::Pause));
        assert!(list(&d).is_empty(), "approved request leaves the queue");
        // taking a non-existent id yields nothing (no spurious apply)
        assert_eq!(take_approved(&d, "shutdown"), None);
        let _ = std::fs::remove_dir_all(&d);
    }
}
