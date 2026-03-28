use super::{CmdError, CmdResult, map_store_err};
use crate::{counters::Counters, gates, state::State, store::Store};

/// Initialize a new workflow project.
pub fn init(store: &impl Store) -> CmdResult<State> {
    if store.is_initialized() {
        return Err(CmdError::AlreadyInitialized);
    }
    let state = State::init();
    let counters = Counters::new();
    store.initialize(&state, &counters).map_err(map_store_err)?;
    Ok(state)
}

/// Read the current workflow status.
pub fn status(store: &impl Store) -> CmdResult<State> {
    if !store.is_initialized() {
        return Err(CmdError::NotInitialized);
    }
    store.read_state().map_err(map_store_err)
}

/// Advance to the next workflow step.
pub fn advance(store: &impl Store) -> CmdResult<State> {
    let state = status(store)?;

    let setup_step = state
        .setup_step()
        .ok_or_else(|| CmdError::Store("invalid step in state".to_owned()))?;

    let violations = gates::check_setup_advance(setup_step, store);
    if !violations.is_empty() {
        return Err(CmdError::GatesFailed(violations));
    }

    let next = setup_step.next().ok_or(CmdError::NoNextStep)?;

    let new_state = State {
        step: Some(next.as_str().to_owned()),
        ..state
    };
    store.write_state(&new_state).map_err(map_store_err)?;
    Ok(new_state)
}

/// Record a stakeholder approval for a step.
pub fn request_approval(store: &impl Store, step_name: &str) -> CmdResult<()> {
    status(store)?;
    let record = format!(
        "{{\"step\":\"{step_name}\",\"approved_by\":\"stakeholder\",\"timestamp\":\"{}\"}}",
        crate::time::now_iso()
    );
    store.append_approval(&record).map_err(map_store_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::Phase, store::MemStore};

    fn initialized_store() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        store
    }

    #[test]
    fn init_creates_state() {
        let store = MemStore::new();
        let state = init(&store).unwrap();
        assert_eq!(state.phase, Phase::Setup);
        assert_eq!(state.step.as_deref(), Some("product-brief"));
    }

    #[test]
    fn init_marks_as_initialized() {
        let store = MemStore::new();
        init(&store).unwrap();
        assert!(store.is_initialized());
    }

    #[test]
    fn init_fails_if_already_initialized() {
        let store = MemStore::new();
        init(&store).unwrap();
        assert!(matches!(
            init(&store).unwrap_err(),
            CmdError::AlreadyInitialized
        ));
    }

    #[test]
    fn init_persists_state() {
        let store = MemStore::new();
        init(&store).unwrap();
        assert_eq!(status(&store).unwrap().phase, Phase::Setup);
    }

    #[test]
    fn status_fails_if_not_initialized() {
        let store = MemStore::new();
        assert!(matches!(
            status(&store).unwrap_err(),
            CmdError::NotInitialized
        ));
    }

    #[test]
    fn advance_fails_without_approval() {
        let store = initialized_store();
        assert!(matches!(
            advance(&store).unwrap_err(),
            CmdError::GatesFailed(_)
        ));
    }

    #[test]
    fn advance_succeeds_with_approval() {
        let store = initialized_store();
        request_approval(&store, "product-brief").unwrap();
        let state = advance(&store).unwrap();
        assert_eq!(state.step.as_deref(), Some("requirements"));
    }

    #[test]
    fn advance_persists_new_state() {
        let store = initialized_store();
        request_approval(&store, "product-brief").unwrap();
        advance(&store).unwrap();
        assert_eq!(
            status(&store).unwrap().step.as_deref(),
            Some("requirements")
        );
    }

    #[test]
    fn advance_at_end_returns_no_next_step() {
        let store = initialized_store();
        let state = State {
            phase: Phase::Setup,
            step: Some("epic-planning".to_owned()),
            current_milestone: None,
            epic: None,
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        request_approval(&store, "epic-planning").unwrap();
        assert!(matches!(advance(&store).unwrap_err(), CmdError::NoNextStep));
    }

    #[test]
    fn request_approval_appends_record() {
        let store = initialized_store();
        request_approval(&store, "product-brief").unwrap();
        let approvals = store.read_approvals().unwrap();
        assert_eq!(approvals.len(), 1);
        assert!(approvals[0].contains("product-brief"));
    }

    #[test]
    fn request_approval_fails_if_not_initialized() {
        let store = MemStore::new();
        assert!(request_approval(&store, "x").is_err());
    }
}
