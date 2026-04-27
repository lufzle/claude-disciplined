use super::{CmdResult, map_store_err, meeting::active_meeting_context, workflow::status};
use crate::{
    decision::{self, Decision, DecisionStatus},
    id::Prefix,
    ndjson,
    state::EpicStep,
    store::Store,
    time,
};

/// Perform a backward transition (escalation) to a target step.
///
/// Requires an active meeting to log the escalation as a decision record.
/// Validates the backward transition is allowed per
/// `EpicStep::escalation_targets`. Returns the decision ID of the logged
/// escalation record.
pub fn escalate(store: &impl Store, target: &str, reason: &str) -> CmdResult<String> {
    let state = status(store)?;

    // Parse current epic step
    let current = state.epic_step().ok_or_else(|| {
        super::CmdError::Store("escalate requires an epic phase with a valid step".to_owned())
    })?;

    // Parse target step
    let target_step = parse_epic_step(target)?;

    // Validate backward transition is allowed
    if !current.escalation_targets().contains(&target_step) {
        return Err(super::CmdError::Store(format!(
            "cannot escalate from {} to {target} (allowed targets: {})",
            current.as_str(),
            current
                .escalation_targets()
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    // Log as a decision record in the active meeting
    let (meetings_dir, meeting_slug) = active_meeting_context(store)?;
    let decisions_path = format!("{meetings_dir}/{meeting_slug}/decisions.ndjson");
    let mut decisions = ndjson::load(store, &decisions_path, decision::from_ndjson_line)
        .map_err(super::CmdError::Store)?;

    let mut counters = store.read_counters().map_err(map_store_err)?;
    let id = counters.next(Prefix::D).map_err(map_store_err)?;
    store.write_counters(&counters).map_err(map_store_err)?;
    let id_str = id.to_string();

    let mut d = Decision::new(
        id_str.clone(),
        format!("Escalation: {} -> {target} — {reason}", current.as_str()),
    );
    d.status = DecisionStatus::ResolvedByOwner;
    d.justification = Some(reason.to_owned());
    d.timestamp = time::now_iso();

    decisions.push(d);
    ndjson::save(store, &decisions_path, &decisions, decision::to_ndjson_line)
        .map_err(super::CmdError::Store)?;

    // Update state: move to target step and clear active meeting
    // (the meeting belonged to the old step)
    let mut new_state = state;
    new_state.step = Some(target.to_owned());
    new_state.active_meeting = None;
    store.write_state(&new_state).map_err(map_store_err)?;

    Ok(id_str)
}

fn parse_epic_step(s: &str) -> CmdResult<EpicStep> {
    for step in EpicStep::ALL {
        if step.as_str() == s {
            return Ok(step);
        }
    }
    Err(super::CmdError::Store(format!("unknown epic step: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{create_epic, create_milestone, init, meeting_start},
        state::{Phase, State},
        store::MemStore,
    };

    fn store_at_epic_step(step: &str) -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();

        let state = State {
            phase: Phase::Epic,
            step: Some(step.to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        store
    }

    fn with_meeting(store: &MemStore) {
        meeting_start(store, "review-session").unwrap();
    }

    // -- Valid escalations --

    #[test]
    fn ux_design_to_analysis() {
        let store = store_at_epic_step("ux-design");
        with_meeting(&store);
        let id = escalate(&store, "analysis", "stories need rework").unwrap();
        assert!(id.starts_with("D-"));

        let state = store.read_state().unwrap();
        assert_eq!(state.step.as_deref(), Some("analysis"));
    }

    #[test]
    fn technical_design_to_ux_design() {
        let store = store_at_epic_step("technical-design");
        with_meeting(&store);
        escalate(&store, "ux-design", "API reveals UX issue").unwrap();
        assert_eq!(
            store.read_state().unwrap().step.as_deref(),
            Some("ux-design")
        );
    }

    #[test]
    fn planning_to_technical_design() {
        let store = store_at_epic_step("planning");
        with_meeting(&store);
        escalate(&store, "technical-design", "missing component").unwrap();
        assert_eq!(
            store.read_state().unwrap().step.as_deref(),
            Some("technical-design")
        );
    }

    #[test]
    fn review_to_implementation() {
        let store = store_at_epic_step("review");
        with_meeting(&store);
        escalate(&store, "implementation", "acceptance criteria not met").unwrap();
        assert_eq!(
            store.read_state().unwrap().step.as_deref(),
            Some("implementation")
        );
    }

    #[test]
    fn verification_to_implementation() {
        let store = store_at_epic_step("verification");
        with_meeting(&store);
        escalate(&store, "implementation", "test failures").unwrap();
        assert_eq!(
            store.read_state().unwrap().step.as_deref(),
            Some("implementation")
        );
    }

    // -- Decision record --

    #[test]
    fn escalation_logs_decision_record() {
        let store = store_at_epic_step("ux-design");
        with_meeting(&store);
        let id = escalate(&store, "analysis", "rework needed").unwrap();

        // Decision was written to the ux-design meetings dir (old step)
        let epic_dir = "milestones/M-001-mvp/epics/E-001-auth";
        let path = format!("{epic_dir}/ux-design/meetings/001-review-session/decisions.ndjson");
        let decisions = ndjson::load(&store, &path, decision::from_ndjson_line).unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].id, id);
        assert_eq!(decisions[0].status, DecisionStatus::ResolvedByOwner);
        assert!(decisions[0].summary.contains("Escalation"));
        assert_eq!(decisions[0].justification.as_deref(), Some("rework needed"));
    }

    #[test]
    fn escalation_clears_active_meeting() {
        let store = store_at_epic_step("ux-design");
        with_meeting(&store);
        escalate(&store, "analysis", "rework needed").unwrap();
        let state = store.read_state().unwrap();
        assert!(
            state.active_meeting.is_none(),
            "meeting should be cleared after escalation"
        );
    }

    // -- Invalid escalations --

    #[test]
    fn analysis_cannot_escalate() {
        let store = store_at_epic_step("analysis");
        with_meeting(&store);
        assert!(escalate(&store, "analysis", "reason").is_err());
    }

    #[test]
    fn implementation_cannot_escalate() {
        let store = store_at_epic_step("implementation");
        with_meeting(&store);
        assert!(escalate(&store, "analysis", "reason").is_err());
    }

    #[test]
    fn ux_design_cannot_skip_to_technical_design() {
        let store = store_at_epic_step("ux-design");
        with_meeting(&store);
        // Can only go to analysis, not technical-design
        assert!(escalate(&store, "technical-design", "reason").is_err());
    }

    #[test]
    fn escalate_fails_without_epic_phase() {
        let store = MemStore::new();
        init(&store).unwrap();
        assert!(escalate(&store, "analysis", "reason").is_err());
    }

    #[test]
    fn escalate_fails_without_active_meeting() {
        let store = store_at_epic_step("ux-design");
        // No meeting started
        assert!(escalate(&store, "analysis", "reason").is_err());
    }

    #[test]
    fn escalate_to_invalid_step_fails() {
        let store = store_at_epic_step("ux-design");
        with_meeting(&store);
        assert!(escalate(&store, "nonexistent", "reason").is_err());
    }

    // -- Error messages --

    #[test]
    fn escalation_error_lists_allowed_targets() {
        let store = store_at_epic_step("ux-design");
        with_meeting(&store);
        let err = escalate(&store, "implementation", "reason").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("analysis"),
            "should list allowed target: {msg}"
        );
    }
}
