use serde::{Deserialize, Serialize};

use crate::time;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionType {
    Research,
    ProofOfConcept,
    StakeholderQuestion,
    Draft,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionStatus {
    Pending,
    InProgress,
    Completed,
    Discarded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionTiming {
    Immediate,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionItem {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: ActionType,
    pub description: String,
    pub assignee: String,
    pub timing: ActionTiming,
    pub status: ActionStatus,
    pub created: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discarded_reason: Option<String>,
}

impl ActionItem {
    pub fn new(
        id: String,
        action_type: ActionType,
        description: String,
        assignee: String,
        timing: ActionTiming,
    ) -> Self {
        Self {
            id,
            action_type,
            description,
            assignee,
            timing,
            status: ActionStatus::Pending,
            created: time::now_iso(),
            completed: None,
            summary: None,
            discarded_reason: None,
        }
    }

    pub fn start(&mut self) {
        self.status = ActionStatus::InProgress;
    }

    pub fn complete(&mut self, summary: &str) {
        self.status = ActionStatus::Completed;
        self.summary = Some(summary.to_owned());
        self.completed = Some(time::now_iso());
    }

    pub fn discard(&mut self, reason: &str) {
        self.status = ActionStatus::Discarded;
        self.discarded_reason = Some(reason.to_owned());
        self.completed = Some(time::now_iso());
    }

    pub fn is_done(&self) -> bool {
        matches!(
            self.status,
            ActionStatus::Completed | ActionStatus::Discarded
        )
    }

    pub fn is_immediate(&self) -> bool {
        self.timing == ActionTiming::Immediate
    }
}

pub fn to_ndjson_line(item: &ActionItem) -> String {
    serde_json::to_string(item).expect("action item serialization")
}

pub fn from_ndjson_line(line: &str) -> Result<ActionItem, String> {
    serde_json::from_str(line).map_err(|e| format!("invalid action item JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_item() -> ActionItem {
        ActionItem::new(
            "AI-001".to_owned(),
            ActionType::Research,
            "Research event sourcing".to_owned(),
            "platform-engineer".to_owned(),
            ActionTiming::Deferred,
        )
    }

    #[test]
    fn new_is_pending() {
        let item = new_item();
        assert_eq!(item.status, ActionStatus::Pending);
        assert!(!item.is_done());
    }

    #[test]
    fn start_sets_in_progress() {
        let mut item = new_item();
        item.start();
        assert_eq!(item.status, ActionStatus::InProgress);
    }

    #[test]
    fn complete_sets_completed_with_summary() {
        let mut item = new_item();
        item.complete("Event sourcing not needed for MVP");
        assert_eq!(item.status, ActionStatus::Completed);
        assert!(item.is_done());
        assert!(item.summary.is_some());
        assert!(item.completed.is_some());
    }

    #[test]
    fn discard_sets_discarded_with_reason() {
        let mut item = new_item();
        item.discard("no longer relevant");
        assert_eq!(item.status, ActionStatus::Discarded);
        assert!(item.is_done());
        assert_eq!(item.discarded_reason.as_deref(), Some("no longer relevant"));
    }

    #[test]
    fn is_immediate_reflects_timing() {
        let deferred = new_item();
        assert!(!deferred.is_immediate());

        let immediate = ActionItem::new(
            "AI-002".to_owned(),
            ActionType::Research,
            "Quick check".to_owned(),
            "analyst".to_owned(),
            ActionTiming::Immediate,
        );
        assert!(immediate.is_immediate());
    }

    #[test]
    fn ndjson_roundtrip() {
        let mut item = new_item();
        item.complete("findings here");
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
}
