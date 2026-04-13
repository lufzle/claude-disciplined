use super::{CmdResult, map_store_err, meeting::active_meeting_context, workflow::status};
use crate::{
    action_item::{self, ActionItem, ActionTiming, ActionType},
    id::Prefix,
    ndjson,
    store::Store,
};

fn actions_path(store: &impl Store) -> CmdResult<String> {
    let (meetings_dir, meeting_slug) = active_meeting_context(store)?;
    Ok(format!(
        "{meetings_dir}/{meeting_slug}/actions/action-items.ndjson"
    ))
}

fn load_actions(store: &impl Store) -> CmdResult<Vec<ActionItem>> {
    let path = actions_path(store)?;
    ndjson::load(store, &path, action_item::from_ndjson_line).map_err(super::CmdError::Store)
}

fn save_actions(store: &impl Store, actions: &[ActionItem]) -> CmdResult<()> {
    let path = actions_path(store)?;
    ndjson::save(store, &path, actions, action_item::to_ndjson_line).map_err(super::CmdError::Store)
}

fn find_action(actions: &[ActionItem], id: &str) -> CmdResult<usize> {
    actions
        .iter()
        .position(|a| a.id == id)
        .ok_or_else(|| super::CmdError::Store(format!("action item {id} not found")))
}

fn next_action_id(store: &impl Store) -> CmdResult<String> {
    let mut counters = store.read_counters().map_err(map_store_err)?;
    let id = counters.next(Prefix::Ai).map_err(map_store_err)?;
    store.write_counters(&counters).map_err(map_store_err)?;
    Ok(id.to_string())
}

pub fn parse_action_type(s: &str) -> Result<ActionType, String> {
    match s {
        "research" => Ok(ActionType::Research),
        "proof-of-concept" => Ok(ActionType::ProofOfConcept),
        "stakeholder-question" => Ok(ActionType::StakeholderQuestion),
        "draft" => Ok(ActionType::Draft),
        "review" => Ok(ActionType::Review),
        _ => Err(format!("invalid action type: {s}")),
    }
}

pub fn add_action(
    store: &impl Store,
    description: &str,
    action_type: ActionType,
    assignee: &str,
    immediate: bool,
) -> CmdResult<String> {
    status(store)?;
    let id = next_action_id(store)?;
    let timing = if immediate {
        ActionTiming::Immediate
    } else {
        ActionTiming::Deferred
    };
    let item = ActionItem::new(
        id.clone(),
        action_type,
        description.to_owned(),
        assignee.to_owned(),
        timing,
    );
    let mut actions = load_actions(store)?;
    actions.push(item);
    save_actions(store, &actions)?;
    Ok(id)
}

pub fn start_action(store: &impl Store, action_id: &str) -> CmdResult<()> {
    status(store)?;
    let mut actions = load_actions(store)?;
    let idx = find_action(&actions, action_id)?;
    actions[idx].start();
    save_actions(store, &actions)
}

pub fn complete_action(store: &impl Store, action_id: &str, summary: &str) -> CmdResult<()> {
    status(store)?;
    let mut actions = load_actions(store)?;
    let idx = find_action(&actions, action_id)?;
    actions[idx].complete(summary);
    save_actions(store, &actions)
}

pub fn discard_action(store: &impl Store, action_id: &str, reason: &str) -> CmdResult<()> {
    status(store)?;
    let mut actions = load_actions(store)?;
    let idx = find_action(&actions, action_id)?;
    actions[idx].discard(reason);
    save_actions(store, &actions)
}

pub fn list_actions(store: &impl Store) -> CmdResult<Vec<ActionItem>> {
    load_actions(store)
}

// ---------------------------------------------------------------------------
// Standalone action commands (resolve by ID across all meetings)
// ---------------------------------------------------------------------------

fn find_action_globally(
    store: &impl Store,
    action_id: &str,
) -> CmdResult<(String, Vec<ActionItem>, usize)> {
    let paths = store.find_all_files_deep(".", "action-items.ndjson");
    for path in paths {
        let actions = ndjson::load(store, &path, action_item::from_ndjson_line)
            .map_err(super::CmdError::Store)?;
        if let Some(idx) = actions.iter().position(|a| a.id == action_id) {
            return Ok((path, actions, idx));
        }
    }
    Err(super::CmdError::Store(format!(
        "action item {action_id} not found"
    )))
}

/// Start an action item by ID (standalone, not meeting-scoped).
pub fn action_start(store: &impl Store, action_id: &str) -> CmdResult<()> {
    status(store)?;
    let (path, mut actions, idx) = find_action_globally(store, action_id)?;
    actions[idx].start();
    ndjson::save(store, &path, &actions, action_item::to_ndjson_line)
        .map_err(super::CmdError::Store)
}

/// Complete an action item by ID (standalone).
pub fn action_complete(store: &impl Store, action_id: &str, summary: &str) -> CmdResult<()> {
    status(store)?;
    let (path, mut actions, idx) = find_action_globally(store, action_id)?;
    actions[idx].complete(summary);
    ndjson::save(store, &path, &actions, action_item::to_ndjson_line)
        .map_err(super::CmdError::Store)
}

/// Discard an action item by ID (standalone).
pub fn action_discard(store: &impl Store, action_id: &str, reason: &str) -> CmdResult<()> {
    status(store)?;
    let (path, mut actions, idx) = find_action_globally(store, action_id)?;
    actions[idx].discard(reason);
    ndjson::save(store, &path, &actions, action_item::to_ndjson_line)
        .map_err(super::CmdError::Store)
}

/// Read an action item by ID (standalone).
pub fn action_read(store: &impl Store, action_id: &str) -> CmdResult<ActionItem> {
    status(store)?;
    let (_path, actions, idx) = find_action_globally(store, action_id)?;
    Ok(actions[idx].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{init, meeting_start},
        store::MemStore,
    };

    fn store_with_meeting() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        meeting_start(&store, "kickoff").unwrap();
        store
    }

    #[test]
    fn add_action_returns_id() {
        let store = store_with_meeting();
        let id = add_action(&store, "Research X", ActionType::Research, "analyst", false).unwrap();
        assert!(id.starts_with("AI-"));
    }

    #[test]
    fn add_action_appears_in_list() {
        let store = store_with_meeting();
        add_action(&store, "Research X", ActionType::Research, "analyst", false).unwrap();
        let actions = list_actions(&store).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].description, "Research X");
    }

    #[test]
    fn start_action_sets_in_progress() {
        let store = store_with_meeting();
        let id = add_action(&store, "Research X", ActionType::Research, "analyst", false).unwrap();
        start_action(&store, &id).unwrap();
        let actions = list_actions(&store).unwrap();
        assert_eq!(
            actions[0].status,
            crate::action_item::ActionStatus::InProgress
        );
    }

    #[test]
    fn complete_action_sets_completed() {
        let store = store_with_meeting();
        let id = add_action(&store, "Research X", ActionType::Research, "analyst", false).unwrap();
        complete_action(&store, &id, "Found that X is good").unwrap();
        let actions = list_actions(&store).unwrap();
        assert!(actions[0].is_done());
        assert_eq!(actions[0].summary.as_deref(), Some("Found that X is good"));
    }

    #[test]
    fn discard_action_sets_discarded() {
        let store = store_with_meeting();
        let id = add_action(&store, "Research X", ActionType::Research, "analyst", false).unwrap();
        discard_action(&store, &id, "no longer needed").unwrap();
        let actions = list_actions(&store).unwrap();
        assert!(actions[0].is_done());
    }

    #[test]
    fn immediate_action_flag() {
        let store = store_with_meeting();
        add_action(&store, "Quick check", ActionType::Research, "analyst", true).unwrap();
        let actions = list_actions(&store).unwrap();
        assert!(actions[0].is_immediate());
    }
}
