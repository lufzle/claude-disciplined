use serde::{Deserialize, Serialize};

use crate::time;

/// Possible outcomes when reviewing an unresolved disagreement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewOutcome {
    Resolved,
    Vindicated,
    Deferred,
}

/// A single review entry in an unresolved item's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewEntry {
    pub epic: String,
    pub outcome: ReviewOutcome,
    pub notes: String,
    pub timestamp: String,
}

/// Status of an unresolved disagreement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnresolvedStatus {
    Open,
    Resolved,
    Vindicated,
}

/// A project-wide unresolved disagreement record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unresolved {
    pub id: String,
    pub origin_decision: String,
    pub origin_epic: String,
    pub summary: String,
    pub dissenter: String,
    pub reason: String,
    pub status: UnresolvedStatus,
    pub history: Vec<ReviewEntry>,
    pub created: String,
}

impl Unresolved {
    pub fn new(
        id: String,
        origin_decision: String,
        origin_epic: String,
        summary: String,
        dissenter: String,
        reason: String,
    ) -> Self {
        Self {
            id,
            origin_decision,
            origin_epic,
            summary,
            dissenter,
            reason,
            status: UnresolvedStatus::Open,
            history: Vec::new(),
            created: time::now_iso(),
        }
    }

    /// Add a review entry and update status based on the outcome.
    pub fn review(&mut self, epic: &str, outcome: ReviewOutcome, notes: &str) {
        let new_status = match outcome {
            ReviewOutcome::Resolved => UnresolvedStatus::Resolved,
            ReviewOutcome::Vindicated => UnresolvedStatus::Vindicated,
            ReviewOutcome::Deferred => UnresolvedStatus::Open,
        };
        self.history.push(ReviewEntry {
            epic: epic.to_owned(),
            outcome,
            notes: notes.to_owned(),
            timestamp: time::now_iso(),
        });
        self.status = new_status;
    }

    pub fn is_open(&self) -> bool {
        self.status == UnresolvedStatus::Open
    }
}

pub fn to_ndjson_line(item: &Unresolved) -> String {
    serde_json::to_string(item).expect("unresolved serialization")
}

pub fn from_ndjson_line(line: &str) -> Result<Unresolved, String> {
    serde_json::from_str(line).map_err(|e| format!("invalid unresolved JSON: {e}"))
}

/// Parse a review outcome string.
pub fn parse_outcome(s: &str) -> Result<ReviewOutcome, String> {
    match s {
        "resolved" => Ok(ReviewOutcome::Resolved),
        "vindicated" => Ok(ReviewOutcome::Vindicated),
        "deferred" => Ok(ReviewOutcome::Deferred),
        _ => Err(format!(
            "invalid outcome: {s} (expected: resolved, accepted, deferred)"
        )),
    }
}

/// The well-known path for the project-wide unresolved file.
pub const PATH: &str = ".workflow/unresolved.ndjson";

#[cfg(test)]
mod tests {
    use super::*;

    fn new_item() -> Unresolved {
        Unresolved::new(
            "UR-001".to_owned(),
            "D-003".to_owned(),
            "E-001".to_owned(),
            "GraphQL vs REST".to_owned(),
            "platform-engineer".to_owned(),
            "GraphQL better for nested data".to_owned(),
        )
    }

    #[test]
    fn new_is_open() {
        let item = new_item();
        assert_eq!(item.status, UnresolvedStatus::Open);
        assert!(item.is_open());
        assert!(item.history.is_empty());
    }

    #[test]
    fn review_deferred_stays_open() {
        let mut item = new_item();
        item.review("E-002", ReviewOutcome::Deferred, "not enough evidence yet");
        assert!(item.is_open());
        assert_eq!(item.history.len(), 1);
        assert_eq!(item.history[0].outcome, ReviewOutcome::Deferred);
    }

    #[test]
    fn review_resolved_closes() {
        let mut item = new_item();
        item.review("E-003", ReviewOutcome::Resolved, "REST works fine");
        assert!(!item.is_open());
        assert_eq!(item.status, UnresolvedStatus::Resolved);
    }

    #[test]
    fn review_accepted_closes() {
        let mut item = new_item();
        item.review(
            "E-003",
            ReviewOutcome::Vindicated,
            "switching to GraphQL next milestone",
        );
        assert!(!item.is_open());
        assert_eq!(item.status, UnresolvedStatus::Vindicated);
    }

    #[test]
    fn multiple_reviews_accumulate() {
        let mut item = new_item();
        item.review("E-002", ReviewOutcome::Deferred, "wait for more data");
        item.review(
            "E-003",
            ReviewOutcome::Resolved,
            "REST confirmed sufficient",
        );
        assert_eq!(item.history.len(), 2);
        assert_eq!(item.status, UnresolvedStatus::Resolved);
    }

    #[test]
    fn ndjson_roundtrip() {
        let mut item = new_item();
        item.review("E-002", ReviewOutcome::Deferred, "notes");
        let line = to_ndjson_line(&item);
        let parsed = from_ndjson_line(&line).unwrap();
        assert_eq!(item, parsed);
    }

    #[test]
    fn ndjson_line_is_single_line() {
        let item = new_item();
        let line = to_ndjson_line(&item);
        assert!(!line.contains('\n'));
    }

    #[test]
    fn parse_outcome_valid() {
        assert_eq!(parse_outcome("resolved").unwrap(), ReviewOutcome::Resolved);
        assert_eq!(
            parse_outcome("vindicated").unwrap(),
            ReviewOutcome::Vindicated
        );
        assert_eq!(parse_outcome("deferred").unwrap(), ReviewOutcome::Deferred);
    }

    #[test]
    fn parse_outcome_invalid() {
        assert!(parse_outcome("invalid").is_err());
    }
}
