use serde::Serialize;

use super::{CmdResult, map_store_err, workflow::status};
use crate::{
    id::Prefix,
    ndjson,
    store::Store,
    unresolved::{self, ReviewOutcome, Unresolved, UnresolvedStatus},
};

/// Add a new unresolved disagreement to the project-wide log.
pub fn add_unresolved(
    store: &impl Store,
    origin_decision: &str,
    origin_epic: &str,
    summary: &str,
    dissenter: &str,
    reason: &str,
) -> CmdResult<String> {
    status(store)?;

    let mut items = ndjson::load(store, unresolved::PATH, unresolved::from_ndjson_line)
        .map_err(super::CmdError::Store)?;

    let mut counters = store.read_counters().map_err(map_store_err)?;
    let id = counters.next(Prefix::Ur).map_err(map_store_err)?;
    store.write_counters(&counters).map_err(map_store_err)?;
    let id_str = id.to_string();

    let item = Unresolved::new(
        id_str.clone(),
        origin_decision.to_owned(),
        origin_epic.to_owned(),
        summary.to_owned(),
        dissenter.to_owned(),
        reason.to_owned(),
    );

    items.push(item);
    ndjson::save(store, unresolved::PATH, &items, unresolved::to_ndjson_line)
        .map_err(super::CmdError::Store)?;

    Ok(id_str)
}

/// Summary for list output.
#[derive(Debug, Serialize)]
pub struct UnresolvedSummary {
    pub id: String,
    pub summary: String,
    pub dissenter: String,
    pub status: String,
    pub origin_epic: String,
    pub reviews: usize,
}

/// List unresolved disagreements, optionally filtered by status.
pub fn list_unresolved(
    store: &impl Store,
    status_filter: Option<&str>,
) -> CmdResult<Vec<UnresolvedSummary>> {
    status(store)?;

    let items = ndjson::load(store, unresolved::PATH, unresolved::from_ndjson_line)
        .map_err(super::CmdError::Store)?;

    let filter_status = status_filter.map(|s| match s {
        "resolved" => UnresolvedStatus::Resolved,
        "vindicated" => UnresolvedStatus::Vindicated,
        _ => UnresolvedStatus::Open,
    });

    let mut result = Vec::new();
    for item in &items {
        if let Some(ref filter) = filter_status {
            if &item.status != filter {
                continue;
            }
        }

        let status_str = match item.status {
            UnresolvedStatus::Open => "open",
            UnresolvedStatus::Resolved => "resolved",
            UnresolvedStatus::Vindicated => "vindicated",
        };

        result.push(UnresolvedSummary {
            id: item.id.clone(),
            summary: item.summary.clone(),
            dissenter: item.dissenter.clone(),
            status: status_str.to_owned(),
            origin_epic: item.origin_epic.clone(),
            reviews: item.history.len(),
        });
    }

    Ok(result)
}

/// Review an unresolved disagreement by its ID.
pub fn review_unresolved(
    store: &impl Store,
    ur_id: &str,
    outcome: ReviewOutcome,
    notes: &str,
) -> CmdResult<()> {
    let state = status(store)?;

    let epic_id = state.epic.as_deref().ok_or_else(|| {
        super::CmdError::Store("review requires an active epic context".to_owned())
    })?;

    let mut items = ndjson::load(store, unresolved::PATH, unresolved::from_ndjson_line)
        .map_err(super::CmdError::Store)?;

    let item = items
        .iter_mut()
        .find(|i| i.id == ur_id)
        .ok_or_else(|| super::CmdError::Store(format!("unresolved item not found: {ur_id}")))?;

    item.review(epic_id, outcome, notes);

    ndjson::save(store, unresolved::PATH, &items, unresolved::to_ndjson_line)
        .map_err(super::CmdError::Store)
}

/// History of reviews for a single unresolved item.
#[derive(Debug, Serialize)]
pub struct UnresolvedHistory {
    pub id: String,
    pub summary: String,
    pub status: String,
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub epic: String,
    pub outcome: String,
    pub notes: String,
    pub timestamp: String,
}

/// Get the full history for an unresolved disagreement.
pub fn history_unresolved(store: &impl Store, ur_id: &str) -> CmdResult<UnresolvedHistory> {
    status(store)?;

    let items = ndjson::load(store, unresolved::PATH, unresolved::from_ndjson_line)
        .map_err(super::CmdError::Store)?;

    let item = items
        .iter()
        .find(|i| i.id == ur_id)
        .ok_or_else(|| super::CmdError::Store(format!("unresolved item not found: {ur_id}")))?;

    let status_str = match item.status {
        UnresolvedStatus::Open => "open",
        UnresolvedStatus::Resolved => "resolved",
        UnresolvedStatus::Vindicated => "vindicated",
    };

    let history = item
        .history
        .iter()
        .map(|e| {
            let outcome_str = match e.outcome {
                unresolved::ReviewOutcome::Resolved => "resolved",
                unresolved::ReviewOutcome::Vindicated => "vindicated",
                unresolved::ReviewOutcome::Deferred => "deferred",
            };
            HistoryEntry {
                epic: e.epic.clone(),
                outcome: outcome_str.to_owned(),
                notes: e.notes.clone(),
                timestamp: e.timestamp.clone(),
            }
        })
        .collect();

    Ok(UnresolvedHistory {
        id: item.id.clone(),
        summary: item.summary.clone(),
        status: status_str.to_owned(),
        history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{create_epic, create_milestone, init},
        state::{Phase, State},
        store::MemStore,
    };

    fn store_with_epic() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();

        let state = State {
            phase: Phase::Epic,
            step: Some("retrospective".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        store
    }

    fn add_sample(store: &MemStore) -> String {
        add_unresolved(
            store,
            "D-003",
            "E-001",
            "GraphQL vs REST",
            "platform-engineer",
            "GraphQL better for nested data",
        )
        .unwrap()
    }

    // -- add --

    #[test]
    fn add_returns_ur_id() {
        let store = store_with_epic();
        let id = add_sample(&store);
        assert_eq!(id, "UR-001");
    }

    #[test]
    fn add_increments_id() {
        let store = store_with_epic();
        add_sample(&store);
        let id2 = add_unresolved(
            &store,
            "D-005",
            "E-001",
            "SQL vs NoSQL",
            "staff-engineer",
            "NoSQL for flexibility",
        )
        .unwrap();
        assert_eq!(id2, "UR-002");
    }

    // -- list --

    #[test]
    fn list_returns_all() {
        let store = store_with_epic();
        add_sample(&store);
        add_unresolved(
            &store,
            "D-005",
            "E-001",
            "SQL vs NoSQL",
            "staff-engineer",
            "reason",
        )
        .unwrap();

        let items = list_unresolved(&store, None).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn list_filters_by_status() {
        let store = store_with_epic();
        let id = add_sample(&store);
        add_unresolved(
            &store,
            "D-005",
            "E-001",
            "SQL vs NoSQL",
            "staff-engineer",
            "reason",
        )
        .unwrap();

        // Resolve first one
        review_unresolved(&store, &id, ReviewOutcome::Resolved, "REST confirmed").unwrap();

        let open = list_unresolved(&store, Some("open")).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, "UR-002");

        let resolved = list_unresolved(&store, Some("resolved")).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id, "UR-001");
    }

    #[test]
    fn list_empty() {
        let store = store_with_epic();
        let items = list_unresolved(&store, None).unwrap();
        assert!(items.is_empty());
    }

    // -- review --

    #[test]
    fn review_deferred_stays_open() {
        let store = store_with_epic();
        let id = add_sample(&store);
        review_unresolved(&store, &id, ReviewOutcome::Deferred, "wait for data").unwrap();

        let items = list_unresolved(&store, Some("open")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].reviews, 1);
    }

    #[test]
    fn review_resolved_closes() {
        let store = store_with_epic();
        let id = add_sample(&store);
        review_unresolved(&store, &id, ReviewOutcome::Resolved, "REST works").unwrap();

        let items = list_unresolved(&store, Some("open")).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn review_accepted_closes() {
        let store = store_with_epic();
        let id = add_sample(&store);
        review_unresolved(
            &store,
            &id,
            ReviewOutcome::Vindicated,
            "switching to GraphQL",
        )
        .unwrap();

        let items = list_unresolved(&store, Some("vindicated")).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn review_nonexistent_fails() {
        let store = store_with_epic();
        assert!(review_unresolved(&store, "UR-999", ReviewOutcome::Resolved, "notes").is_err());
    }

    #[test]
    fn review_requires_epic_context() {
        let store = MemStore::new();
        init(&store).unwrap();
        // No epic in state
        assert!(review_unresolved(&store, "UR-001", ReviewOutcome::Resolved, "notes").is_err());
    }

    // -- history --

    #[test]
    fn history_shows_all_reviews() {
        let store = store_with_epic();
        let id = add_sample(&store);
        review_unresolved(&store, &id, ReviewOutcome::Deferred, "wait").unwrap();

        // Switch to E-002 context for second review
        create_epic(&store, "milestones/M-001-mvp", "billing").unwrap();
        let state = State {
            phase: Phase::Epic,
            step: Some("retrospective".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-002".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();

        review_unresolved(&store, &id, ReviewOutcome::Resolved, "confirmed REST").unwrap();

        let hist = history_unresolved(&store, &id).unwrap();
        assert_eq!(hist.history.len(), 2);
        assert_eq!(hist.history[0].epic, "E-001");
        assert_eq!(hist.history[0].outcome, "deferred");
        assert_eq!(hist.history[1].epic, "E-002");
        assert_eq!(hist.history[1].outcome, "resolved");
        assert_eq!(hist.status, "resolved");
    }

    #[test]
    fn history_nonexistent_fails() {
        let store = store_with_epic();
        assert!(history_unresolved(&store, "UR-999").is_err());
    }

    // -- requires initialization --

    #[test]
    fn add_requires_initialization() {
        let store = MemStore::new();
        assert!(add_unresolved(&store, "D-001", "E-001", "s", "r", "reason").is_err());
    }

    #[test]
    fn list_requires_initialization() {
        let store = MemStore::new();
        assert!(list_unresolved(&store, None).is_err());
    }
}
