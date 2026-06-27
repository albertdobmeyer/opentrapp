//! Control channel — the durable request inbox a viewer uses to ask the daemon
//! to act (Phase B, ADR-0019).
//!
//! This is the "markers are truth; the request inbox is the durable control
//! path" half — ADR-0019's transport option (a). A viewer (or `opentrapp-daemon
//! <verb>`) atomically drops a request file under `<data_dir>/control/`; the
//! daemon's supervisor drains + executes them each tick. It is polling-latent by
//! design (bounded by the supervisor tick); the future Unix socket (B4b) is the
//! low-latency fast path layered over the same intent, never a replacement —
//! durable state still lives in the markers + this inbox.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Per-process sequence so two submits in the same nanosecond never collide.
static SEQ: AtomicU64 = AtomicU64::new(0);

/// A request from a viewer to the perimeter-owning daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlRequest {
    /// Drop to dormant now (and arm the wake-on-message waker).
    Pause,
    /// Wake from dormant now (stop the waker, bring the perimeter up).
    Resume,
    /// Bring the perimeter down and back up.
    Restart,
    /// Stop owning the perimeter and exit (tears the perimeter down).
    Shutdown,
}

impl ControlRequest {
    pub fn as_token(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Restart => "restart",
            Self::Shutdown => "shutdown",
        }
    }

    pub fn from_token(s: &str) -> Option<Self> {
        match s.trim() {
            "pause" => Some(Self::Pause),
            "resume" => Some(Self::Resume),
            "restart" => Some(Self::Restart),
            "shutdown" => Some(Self::Shutdown),
            _ => None,
        }
    }

    /// The ADR-0021 boundary classification of this control verb. `Pause` and
    /// `Shutdown` leave the perimeter's protection *reduced* (down) → weakening;
    /// `Resume` and `Restart` return it to full, re-verified protection → neutral.
    /// The supervisor dispatch consults this to refuse an agent-reachable
    /// weakening (the control inbox is a file drop any host process can write).
    /// A mis-tag here (a weakener marked neutral) is a real vulnerability, so it
    /// is test-pinned and fails closed by construction.
    pub fn boundary_impact(self) -> crate::boundary::BoundaryImpact {
        use crate::boundary::BoundaryImpact;
        match self {
            Self::Pause | Self::Shutdown => BoundaryImpact::Weakening,
            Self::Resume | Self::Restart => BoundaryImpact::Neutral,
        }
    }
}

fn inbox_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("control")
}

/// Submit a control request. Atomic (write-tmp + rename) so the daemon never
/// reads a partially-written file. Filenames sort chronologically.
pub fn submit(data_dir: &Path, req: ControlRequest) -> std::io::Result<()> {
    let dir = inbox_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let stem = format!("{nanos:020}-{}-{seq:06}", std::process::id());
    let tmp = dir.join(format!(".{stem}.tmp"));
    let final_path = dir.join(format!("{stem}.req"));
    std::fs::write(&tmp, req.as_token())?;
    std::fs::rename(&tmp, &final_path)
}

/// Drain all pending requests in submission order, removing each as it is read.
/// Unparseable files are discarded too, so a bad drop can't wedge the inbox.
pub fn drain(data_dir: &Path) -> Vec<ControlRequest> {
    let dir = inbox_dir(data_dir);
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|x| x == "req").unwrap_or(false))
            .collect(),
        Err(_) => return Vec::new(),
    };
    entries.sort();
    let mut out = Vec::new();
    for p in entries {
        let token = std::fs::read_to_string(&p).unwrap_or_default();
        if let Some(req) = ControlRequest::from_token(&token) {
            out.push(req);
        }
        let _ = std::fs::remove_file(&p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("opentrapp-control-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn token_round_trip() {
        for r in [
            ControlRequest::Pause,
            ControlRequest::Resume,
            ControlRequest::Restart,
            ControlRequest::Shutdown,
        ] {
            assert_eq!(ControlRequest::from_token(r.as_token()), Some(r));
        }
        assert_eq!(ControlRequest::from_token("bogus"), None);
    }

    #[test]
    fn weakening_control_verbs_are_never_agent_operable() {
        use crate::boundary::BoundaryImpact;
        // Pause + Shutdown leave the perimeter down → weakening; the control
        // inbox is an agent-writable file drop, so these must classify as
        // NOT agent-operable (the supervisor refuses them, ADR-0021 §2).
        for w in [ControlRequest::Pause, ControlRequest::Shutdown] {
            assert_eq!(w.boundary_impact(), BoundaryImpact::Weakening, "{w:?} weakens");
            assert!(!w.boundary_impact().agent_operable(), "{w:?} must not be agent-operable");
        }
        // Resume + Restart return to full, re-verified protection → neutral.
        for n in [ControlRequest::Resume, ControlRequest::Restart] {
            assert_eq!(n.boundary_impact(), BoundaryImpact::Neutral, "{n:?} is neutral");
            assert!(n.boundary_impact().agent_operable(), "{n:?} is agent-operable");
        }
    }

    #[test]
    fn submit_then_drain_in_order_and_clears() {
        let d = temp_dir("io");
        assert!(drain(&d).is_empty());
        submit(&d, ControlRequest::Pause).unwrap();
        submit(&d, ControlRequest::Resume).unwrap();
        submit(&d, ControlRequest::Shutdown).unwrap();
        assert_eq!(
            drain(&d),
            vec![
                ControlRequest::Pause,
                ControlRequest::Resume,
                ControlRequest::Shutdown
            ]
        );
        assert!(drain(&d).is_empty(), "inbox cleared after drain");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unparseable_file_is_discarded_not_wedged() {
        let d = temp_dir("bad");
        std::fs::create_dir_all(inbox_dir(&d)).unwrap();
        std::fs::write(inbox_dir(&d).join("00000000000000000000-0-000000.req"), "garbage")
            .unwrap();
        assert!(drain(&d).is_empty());
        assert!(drain(&d).is_empty()); // and the bad file was removed
        let _ = std::fs::remove_dir_all(&d);
    }
}
