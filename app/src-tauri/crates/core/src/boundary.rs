//! ADR-0021 — the danger-gated control-plane authorization primitive.
//!
//! `boundary_impact` is a security axis **distinct from** the operational `danger`
//! field ([`crate::orchestrator::manifest::Danger`]): `danger` measures data-loss /
//! disruption; `boundary_impact` measures whether an operation reduces the
//! *perimeter's* protection. The gate (ADR-0021 §2): `neutral` ops are
//! agent-operable; `weakening` ops are **never** auto-applied from any
//! agent-reachable transport (CLI / MCP / loopback / the control channel) — they
//! require an out-of-band human confirmation. **Fail-closed:** a missing
//! classification is treated as `weakening` (the more restrictive gate, §1).

use serde::{Deserialize, Serialize};

/// Whether a control-plane operation reduces the perimeter's protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoundaryImpact {
    /// Operates *within* the perimeter; does not reduce its protection
    /// (status, logs, scans, in-policy restarts, resume).
    Neutral,
    /// Reduces the perimeter's protection (loosen the egress allowlist;
    /// pause/stop the boundary; loosen the shell level; edit egress/proxy/CA
    /// policy; disable a layer).
    Weakening,
}

impl Default for BoundaryImpact {
    /// Fail-closed (ADR-0021 §1): an unclassified operation is treated as
    /// boundary-weakening, so a mis-tag fails safe rather than open.
    fn default() -> Self {
        BoundaryImpact::Weakening
    }
}

impl BoundaryImpact {
    /// Whether an **agent-reachable** transport (CLI, MCP, loopback API, the
    /// control channel) may apply this operation *without* an out-of-band human
    /// confirmation. Only `neutral` is agent-operable (ADR-0021 §2): a
    /// `weakening` op must be routed to the human-approval surface, never
    /// auto-applied.
    pub fn agent_operable(self) -> bool {
        matches!(self, BoundaryImpact::Neutral)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_neutral_is_agent_operable() {
        assert!(
            BoundaryImpact::Neutral.agent_operable(),
            "neutral ops are agent-operable"
        );
        assert!(
            !BoundaryImpact::Weakening.agent_operable(),
            "a weakening op must NEVER be agent-operable (ADR-0021 §2)"
        );
    }

    #[derive(Deserialize)]
    struct Holder {
        #[serde(default)]
        boundary_impact: BoundaryImpact,
    }

    #[test]
    fn unclassified_fails_closed_to_weakening() {
        // The single most important safety property (ADR-0021 §1): an operation
        // with NO boundary_impact must classify as weakening — never agent-operable.
        assert_eq!(BoundaryImpact::default(), BoundaryImpact::Weakening);
        let missing: Holder = serde_yaml::from_str("{}").unwrap();
        assert_eq!(
            missing.boundary_impact,
            BoundaryImpact::Weakening,
            "a manifest omitting boundary_impact must fail closed"
        );
        assert!(!missing.boundary_impact.agent_operable());
    }

    #[test]
    fn explicit_values_round_trip() {
        let n: Holder = serde_yaml::from_str("boundary_impact: neutral").unwrap();
        assert_eq!(n.boundary_impact, BoundaryImpact::Neutral);
        let w: Holder = serde_yaml::from_str("boundary_impact: weakening").unwrap();
        assert_eq!(w.boundary_impact, BoundaryImpact::Weakening);
        // an unknown/misspelled value is rejected outright (also fail-closed:
        // the manifest fails to load rather than silently defaulting open).
        assert!(serde_yaml::from_str::<Holder>("boundary_impact: nuetral").is_err());
    }
}
