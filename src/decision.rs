use serde::{Deserialize, Serialize};

use crate::time;

/// Status of a decision through its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecisionStatus {
    Proposed,
    Discussing,
    Agreed,
    AgreedWithReservations,
    ResolvedByOwner,
    Dropped,
    Superseded,
}

impl DecisionStatus {
    /// Whether this status is terminal (no further transitions expected).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Agreed
                | Self::AgreedWithReservations
                | Self::ResolvedByOwner
                | Self::Dropped
                | Self::Superseded
        )
    }
}

/// A participant's position on a decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PositionKind {
    Agree,
    Disagree,
    DisagreeAndCommit,
}

/// A recorded position from a participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub role: String,
    pub position: PositionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addressed: Option<bool>,
}

/// A decision record stored in decisions.ndjson.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub summary: String,
    pub status: DecisionStatus,
    #[serde(default)]
    pub informed_by: Vec<String>,
    #[serde(default)]
    pub positions: Vec<Position>,
    #[serde(default)]
    pub iteration: u32,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drop_reason: Option<String>,
}

impl Decision {
    /// Create a new proposed decision.
    pub fn new(id: String, summary: String) -> Self {
        Self {
            id,
            summary,
            status: DecisionStatus::Proposed,
            informed_by: Vec::new(),
            positions: Vec::new(),
            iteration: 0,
            timestamp: time::now_iso(),
            resolved_by: None,
            justification: None,
            superseded_by: None,
            drop_reason: None,
        }
    }

    /// Register or update a participant's position. Increments iteration.
    pub fn set_position(&mut self, role: &str, kind: PositionKind, reason: Option<String>) {
        if let Some(existing) = self.positions.iter_mut().find(|p| p.role == role) {
            existing.position = kind;
            existing.reason = reason;
            existing.addressed = None;
        } else {
            self.positions.push(Position {
                role: role.to_owned(),
                position: kind,
                reason,
                addressed: None,
            });
        }
        self.iteration += 1;
        if self.status == DecisionStatus::Proposed {
            self.status = DecisionStatus::Discussing;
        }
    }

    /// Check if the decision can be recorded (all positions agree or
    /// disagree-and-commit).
    pub fn can_record(&self) -> bool {
        !self.positions.is_empty()
            && self.positions.iter().all(|p| {
                matches!(
                    p.position,
                    PositionKind::Agree | PositionKind::DisagreeAndCommit
                )
            })
    }

    /// Record the decision as agreed. Returns error if prerequisites not met.
    pub fn record(&mut self) -> Result<(), String> {
        if !self.can_record() {
            return Err("cannot record: unresolved disagree positions remain".to_owned());
        }
        self.status = if self
            .positions
            .iter()
            .any(|p| p.position == PositionKind::DisagreeAndCommit)
        {
            DecisionStatus::AgreedWithReservations
        } else {
            DecisionStatus::Agreed
        };
        self.timestamp = time::now_iso();
        Ok(())
    }

    /// Check if the decision can be force-resolved by the step owner.
    pub fn can_resolve(&self, min_iterations: u32) -> Result<(), String> {
        if self.iteration < min_iterations {
            return Err(format!(
                "minimum {} iterations required, only {} completed",
                min_iterations, self.iteration
            ));
        }
        for p in &self.positions {
            if p.position == PositionKind::Disagree && p.reason.is_none() {
                return Err(format!("disagree from {} has no reason", p.role));
            }
            if p.position == PositionKind::Disagree && p.addressed != Some(true) {
                return Err(format!(
                    "disagreement from {} has not been addressed",
                    p.role
                ));
            }
        }
        Ok(())
    }

    /// Force-resolve the decision by the step owner.
    pub fn resolve(
        &mut self,
        resolved_by: &str,
        justification: &str,
        min_iterations: u32,
    ) -> Result<(), String> {
        self.can_resolve(min_iterations)?;
        self.status = DecisionStatus::ResolvedByOwner;
        self.resolved_by = Some(resolved_by.to_owned());
        self.justification = Some(justification.to_owned());
        self.timestamp = time::now_iso();
        Ok(())
    }

    /// Drop the decision.
    pub fn drop_decision(&mut self, reason: &str) {
        self.status = DecisionStatus::Dropped;
        self.drop_reason = Some(reason.to_owned());
        self.timestamp = time::now_iso();
    }

    /// Mark as superseded by another decision.
    pub fn supersede(&mut self, new_id: &str) {
        self.status = DecisionStatus::Superseded;
        self.superseded_by = Some(new_id.to_owned());
        self.timestamp = time::now_iso();
    }

    /// Mark a disagree position as addressed.
    pub fn address_disagreement(&mut self, role: &str) -> Result<(), String> {
        let pos = self
            .positions
            .iter_mut()
            .find(|p| p.role == role && p.position == PositionKind::Disagree)
            .ok_or_else(|| format!("no disagree position from {role}"))?;
        pos.addressed = Some(true);
        self.iteration += 1;
        Ok(())
    }
}

/// Serialize a decision to a single NDJSON line.
pub fn to_ndjson_line(decision: &Decision) -> String {
    serde_json::to_string(decision).expect("decision serialization")
}

/// Deserialize a decision from an NDJSON line.
pub fn from_ndjson_line(line: &str) -> Result<Decision, String> {
    serde_json::from_str(line).map_err(|e| format!("invalid decision JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_decision() -> Decision {
        Decision::new("D-001".to_owned(), "Use REST".to_owned())
    }

    // -- Creation --

    #[test]
    fn new_decision_is_proposed() {
        let d = new_decision();
        assert_eq!(d.status, DecisionStatus::Proposed);
        assert_eq!(d.iteration, 0);
        assert!(d.positions.is_empty());
    }

    // -- Positions --

    #[test]
    fn set_position_transitions_to_discussing() {
        let mut d = new_decision();
        d.set_position("analyst", PositionKind::Agree, None);
        assert_eq!(d.status, DecisionStatus::Discussing);
    }

    #[test]
    fn set_position_increments_iteration() {
        let mut d = new_decision();
        d.set_position("analyst", PositionKind::Agree, None);
        assert_eq!(d.iteration, 1);
        d.set_position("ux-designer", PositionKind::Agree, None);
        assert_eq!(d.iteration, 2);
    }

    #[test]
    fn set_position_updates_existing() {
        let mut d = new_decision();
        d.set_position("analyst", PositionKind::Disagree, Some("no".to_owned()));
        d.set_position("analyst", PositionKind::Agree, None);
        assert_eq!(d.positions.len(), 1);
        assert_eq!(d.positions[0].position, PositionKind::Agree);
    }

    // -- can_record --

    #[test]
    fn can_record_all_agree() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Agree, None);
        d.set_position("b", PositionKind::Agree, None);
        assert!(d.can_record());
    }

    #[test]
    fn can_record_with_disagree_and_commit() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Agree, None);
        d.set_position("b", PositionKind::DisagreeAndCommit, Some("ok".to_owned()));
        assert!(d.can_record());
    }

    #[test]
    fn cannot_record_with_disagree() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Agree, None);
        d.set_position("b", PositionKind::Disagree, Some("no".to_owned()));
        assert!(!d.can_record());
    }

    #[test]
    fn cannot_record_with_no_positions() {
        let d = new_decision();
        assert!(!d.can_record());
    }

    // -- record --

    #[test]
    fn record_all_agree_status_agreed() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Agree, None);
        d.record().unwrap();
        assert_eq!(d.status, DecisionStatus::Agreed);
    }

    #[test]
    fn record_with_reservations() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Agree, None);
        d.set_position("b", PositionKind::DisagreeAndCommit, Some("ok".to_owned()));
        d.record().unwrap();
        assert_eq!(d.status, DecisionStatus::AgreedWithReservations);
    }

    #[test]
    fn record_fails_with_disagree() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Disagree, Some("no".to_owned()));
        assert!(d.record().is_err());
    }

    // -- resolve --

    #[test]
    fn resolve_succeeds_after_iterations() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Disagree, Some("no".to_owned()));
        d.set_position("b", PositionKind::Agree, None);
        d.address_disagreement("a").unwrap();
        // iteration is now 3
        d.resolve("driver", "overriding for MVP", 3).unwrap();
        assert_eq!(d.status, DecisionStatus::ResolvedByOwner);
        assert_eq!(d.resolved_by.as_deref(), Some("driver"));
    }

    #[test]
    fn resolve_fails_below_min_iterations() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Disagree, Some("no".to_owned()));
        d.address_disagreement("a").unwrap();
        // iteration is 2, min is 3
        assert!(d.resolve("driver", "reason", 3).is_err());
    }

    #[test]
    fn resolve_fails_without_addressed() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Disagree, Some("no".to_owned()));
        d.set_position("b", PositionKind::Agree, None);
        d.set_position("c", PositionKind::Agree, None);
        // 3 iterations but disagreement not addressed
        assert!(d.resolve("driver", "reason", 3).is_err());
    }

    #[test]
    fn resolve_fails_without_reason_on_disagree() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Disagree, None);
        d.set_position("b", PositionKind::Agree, None);
        d.address_disagreement("a").unwrap();
        assert!(d.resolve("driver", "reason", 3).is_err());
    }

    // -- drop / supersede --

    #[test]
    fn drop_sets_status_and_reason() {
        let mut d = new_decision();
        d.drop_decision("no longer relevant");
        assert_eq!(d.status, DecisionStatus::Dropped);
        assert_eq!(d.drop_reason.as_deref(), Some("no longer relevant"));
    }

    #[test]
    fn supersede_sets_status_and_link() {
        let mut d = new_decision();
        d.supersede("D-002");
        assert_eq!(d.status, DecisionStatus::Superseded);
        assert_eq!(d.superseded_by.as_deref(), Some("D-002"));
    }

    // -- Terminal status --

    #[test]
    fn terminal_statuses() {
        assert!(DecisionStatus::Agreed.is_terminal());
        assert!(DecisionStatus::AgreedWithReservations.is_terminal());
        assert!(DecisionStatus::ResolvedByOwner.is_terminal());
        assert!(DecisionStatus::Dropped.is_terminal());
        assert!(DecisionStatus::Superseded.is_terminal());
        assert!(!DecisionStatus::Proposed.is_terminal());
        assert!(!DecisionStatus::Discussing.is_terminal());
    }

    // -- NDJSON serialization --

    #[test]
    fn ndjson_roundtrip() {
        let mut d = new_decision();
        d.set_position("analyst", PositionKind::Agree, None);
        d.record().unwrap();
        let line = to_ndjson_line(&d);
        let parsed = from_ndjson_line(&line).unwrap();
        assert_eq!(d, parsed);
    }

    #[test]
    fn ndjson_line_is_single_line() {
        let d = new_decision();
        let line = to_ndjson_line(&d);
        assert!(!line.contains('\n'));
    }

    // -- address_disagreement --

    #[test]
    fn address_disagreement_marks_addressed() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Disagree, Some("no".to_owned()));
        d.address_disagreement("a").unwrap();
        assert_eq!(d.positions[0].addressed, Some(true));
    }

    #[test]
    fn address_disagreement_fails_for_wrong_role() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Agree, None);
        assert!(d.address_disagreement("a").is_err());
    }

    #[test]
    fn address_disagreement_increments_iteration() {
        let mut d = new_decision();
        d.set_position("a", PositionKind::Disagree, Some("no".to_owned()));
        let before = d.iteration;
        d.address_disagreement("a").unwrap();
        assert_eq!(d.iteration, before + 1);
    }
}
