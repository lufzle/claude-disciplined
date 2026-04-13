use super::{CmdResult, map_store_err, meeting::active_meeting_context, workflow::status};
use crate::{
    decision::{self, Decision, PositionKind},
    id::Prefix,
    ndjson,
    store::Store,
};

fn decisions_path(store: &impl Store) -> CmdResult<String> {
    let (meetings_dir, meeting_slug) = active_meeting_context(store)?;
    Ok(format!("{meetings_dir}/{meeting_slug}/decisions.ndjson"))
}

fn load_decisions(store: &impl Store) -> CmdResult<Vec<Decision>> {
    let path = decisions_path(store)?;
    ndjson::load(store, &path, decision::from_ndjson_line).map_err(super::CmdError::Store)
}

fn save_decisions(store: &impl Store, decisions: &[Decision]) -> CmdResult<()> {
    let path = decisions_path(store)?;
    ndjson::save(store, &path, decisions, decision::to_ndjson_line).map_err(super::CmdError::Store)
}

/// Find a mutable decision by ID. Returns index.
fn find_decision(decisions: &[Decision], id: &str) -> CmdResult<usize> {
    decisions
        .iter()
        .position(|d| d.id == id)
        .ok_or_else(|| super::CmdError::Store(format!("decision {id} not found")))
}

/// Allocate a decision ID.
fn next_decision_id(store: &impl Store) -> CmdResult<String> {
    let mut counters = store.read_counters().map_err(map_store_err)?;
    let id = counters.next(Prefix::D).map_err(map_store_err)?;
    store.write_counters(&counters).map_err(map_store_err)?;
    Ok(id.to_string())
}

/// Propose a new decision in the active meeting.
pub fn propose_decision(store: &impl Store, summary: &str) -> CmdResult<String> {
    status(store)?;
    let id = next_decision_id(store)?;
    let d = Decision::new(id.clone(), summary.to_owned());
    let mut decisions = load_decisions(store)?;
    decisions.push(d);
    save_decisions(store, &decisions)?;
    Ok(id)
}

/// Register a position on a decision.
pub fn position(
    store: &impl Store,
    decision_id: &str,
    role: &str,
    kind: PositionKind,
    reason: Option<String>,
) -> CmdResult<()> {
    status(store)?;
    let mut decisions = load_decisions(store)?;
    let idx = find_decision(&decisions, decision_id)?;
    decisions[idx].set_position(role, kind, reason);
    save_decisions(store, &decisions)
}

/// Record a decision as agreed (requires all positions agree or
/// disagree-and-commit).
pub fn record_decision(store: &impl Store, decision_id: &str) -> CmdResult<()> {
    status(store)?;
    let mut decisions = load_decisions(store)?;
    let idx = find_decision(&decisions, decision_id)?;
    decisions[idx].record().map_err(super::CmdError::Store)?;
    save_decisions(store, &decisions)
}

/// Force-resolve a decision by the step owner (guarded).
pub fn resolve_decision(
    store: &impl Store,
    decision_id: &str,
    resolved_by: &str,
    justification: &str,
    min_iterations: u32,
) -> CmdResult<()> {
    status(store)?;
    let mut decisions = load_decisions(store)?;
    let idx = find_decision(&decisions, decision_id)?;
    decisions[idx]
        .resolve(resolved_by, justification, min_iterations)
        .map_err(super::CmdError::Store)?;
    save_decisions(store, &decisions)
}

/// Drop a decision.
pub fn drop_decision(store: &impl Store, decision_id: &str, reason: &str) -> CmdResult<()> {
    status(store)?;
    let mut decisions = load_decisions(store)?;
    let idx = find_decision(&decisions, decision_id)?;
    decisions[idx].drop_decision(reason);
    save_decisions(store, &decisions)
}

/// Supersede a decision with a new one.
pub fn supersede_decision(store: &impl Store, old_id: &str, new_id: &str) -> CmdResult<()> {
    status(store)?;
    let mut decisions = load_decisions(store)?;
    let idx = find_decision(&decisions, old_id)?;
    decisions[idx].supersede(new_id);
    save_decisions(store, &decisions)
}

/// Get the status of a specific decision.
pub fn decision_status(store: &impl Store, decision_id: &str) -> CmdResult<Decision> {
    let decisions = load_decisions(store)?;
    let idx = find_decision(&decisions, decision_id)?;
    Ok(decisions[idx].clone())
}

/// List all decisions in the active meeting.
pub fn list_decisions(store: &impl Store) -> CmdResult<Vec<Decision>> {
    load_decisions(store)
}

/// Mark a disagreement as addressed by the driver.
pub fn address_disagreement(store: &impl Store, decision_id: &str, role: &str) -> CmdResult<()> {
    status(store)?;
    let mut decisions = load_decisions(store)?;
    let idx = find_decision(&decisions, decision_id)?;
    decisions[idx]
        .address_disagreement(role)
        .map_err(super::CmdError::Store)?;
    save_decisions(store, &decisions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{init, meeting_start},
        decision::DecisionStatus,
        store::MemStore,
    };

    fn store_with_meeting() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        meeting_start(&store, "kickoff").unwrap();
        store
    }

    // -- propose --

    #[test]
    fn propose_returns_id() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        assert!(id.starts_with("D-"));
    }

    #[test]
    fn propose_creates_decision_in_list() {
        let store = store_with_meeting();
        propose_decision(&store, "Use REST").unwrap();
        let decisions = list_decisions(&store).unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].summary, "Use REST");
    }

    #[test]
    fn propose_multiple_increments_ids() {
        let store = store_with_meeting();
        let id1 = propose_decision(&store, "First").unwrap();
        let id2 = propose_decision(&store, "Second").unwrap();
        assert_ne!(id1, id2);
    }

    // -- position --

    #[test]
    fn position_agree() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        position(&store, &id, "analyst", PositionKind::Agree, None).unwrap();
        let d = decision_status(&store, &id).unwrap();
        assert_eq!(d.status, DecisionStatus::Discussing);
        assert_eq!(d.positions.len(), 1);
    }

    #[test]
    fn position_disagree_requires_reason() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        position(
            &store,
            &id,
            "analyst",
            PositionKind::Disagree,
            Some("prefer GraphQL".to_owned()),
        )
        .unwrap();
        let d = decision_status(&store, &id).unwrap();
        assert_eq!(d.positions[0].reason.as_deref(), Some("prefer GraphQL"));
    }

    // -- record --

    #[test]
    fn record_succeeds_when_all_agree() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        position(&store, &id, "a", PositionKind::Agree, None).unwrap();
        position(&store, &id, "b", PositionKind::Agree, None).unwrap();
        record_decision(&store, &id).unwrap();
        let d = decision_status(&store, &id).unwrap();
        assert_eq!(d.status, DecisionStatus::Agreed);
    }

    #[test]
    fn record_fails_with_disagree() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        position(&store, &id, "a", PositionKind::Agree, None).unwrap();
        position(
            &store,
            &id,
            "b",
            PositionKind::Disagree,
            Some("no".to_owned()),
        )
        .unwrap();
        assert!(record_decision(&store, &id).is_err());
    }

    // -- resolve --

    #[test]
    fn resolve_succeeds_when_guarded() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        position(
            &store,
            &id,
            "dissenter",
            PositionKind::Disagree,
            Some("no".to_owned()),
        )
        .unwrap();
        position(&store, &id, "other", PositionKind::Agree, None).unwrap();
        address_disagreement(&store, &id, "dissenter").unwrap();
        // iteration is now 3
        resolve_decision(&store, &id, "driver", "overriding", 3).unwrap();
        let d = decision_status(&store, &id).unwrap();
        assert_eq!(d.status, DecisionStatus::ResolvedByOwner);
    }

    #[test]
    fn resolve_fails_below_min_iterations() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        position(
            &store,
            &id,
            "a",
            PositionKind::Disagree,
            Some("no".to_owned()),
        )
        .unwrap();
        address_disagreement(&store, &id, "a").unwrap();
        // 2 iterations, need 3
        assert!(resolve_decision(&store, &id, "driver", "reason", 3).is_err());
    }

    // -- drop / supersede --

    #[test]
    fn drop_sets_status() {
        let store = store_with_meeting();
        let id = propose_decision(&store, "Use REST").unwrap();
        drop_decision(&store, &id, "not needed").unwrap();
        let d = decision_status(&store, &id).unwrap();
        assert_eq!(d.status, DecisionStatus::Dropped);
    }

    #[test]
    fn supersede_sets_status() {
        let store = store_with_meeting();
        let id1 = propose_decision(&store, "Old").unwrap();
        let id2 = propose_decision(&store, "New").unwrap();
        supersede_decision(&store, &id1, &id2).unwrap();
        let d = decision_status(&store, &id1).unwrap();
        assert_eq!(d.status, DecisionStatus::Superseded);
        assert_eq!(d.superseded_by.as_deref(), Some(id2.as_str()));
    }

    // -- list --

    #[test]
    fn list_empty_initially() {
        let store = store_with_meeting();
        assert!(list_decisions(&store).unwrap().is_empty());
    }
}
