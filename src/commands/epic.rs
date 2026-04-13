use serde::Serialize;

use super::{
    CmdResult, extract_field, map_store_err, replace_front_matter_field, workflow::status,
};
use crate::{resolve, state::Phase, store::Store, time};

/// Information returned by `epic_start` so the binary can create the git
/// branch and worktree.
#[derive(Debug, Serialize)]
pub struct EpicStartInfo {
    pub branch_name: String,
    pub epic_dir: String,
    pub milestone_id: String,
    pub epic_id: String,
}

/// Validate and prepare for epic start. Updates epic status to `active`.
///
/// The binary is responsible for:
/// 1. Creating the git branch (`branch_name`)
/// 2. Creating a worktree
/// 3. Writing `.workflow/state.yml` in the worktree via `epic_branch_state`
pub fn epic_start(store: &impl Store, epic_id: &str) -> CmdResult<EpicStartInfo> {
    let state = status(store)?;

    // Must be in execution/epic-planning phase
    if state.phase != Phase::Execution {
        return Err(super::CmdError::Store(
            "epic start requires execution phase (epic-planning step)".to_owned(),
        ));
    }
    if state.step.as_deref() != Some("epic-planning") {
        return Err(super::CmdError::Store(
            "epic start requires epic-planning step".to_owned(),
        ));
    }

    let milestone_id = state
        .current_milestone
        .as_deref()
        .ok_or_else(|| super::CmdError::Store("no current milestone in state".to_owned()))?
        .to_owned();

    let m_dir = resolve::current_milestone_dir(store).map_err(super::CmdError::Store)?;

    // Resolve epic directory
    let e_dir = store
        .find_dir(&format!("{m_dir}/epics/{epic_id}-*"))
        .ok_or_else(|| super::CmdError::Store(format!("epic directory not found for {epic_id}")))?;

    // Extract the directory basename for the branch name (e.g., "E-001-auth")
    let epic_dir_name = e_dir
        .rsplit_once('/')
        .map_or(e_dir.as_str(), |(_, name)| name);
    let branch_name = format!("{milestone_id}/{epic_dir_name}");

    // Update epic front matter: draft -> active
    let epic_path = format!("{e_dir}/epic.md");
    let content = store.read_file(&epic_path).map_err(map_store_err)?;
    let updated = replace_front_matter_field(&content, "status", "active");
    let updated = replace_front_matter_field(&updated, "updated", &time::today());
    store
        .write_file(&epic_path, &updated)
        .map_err(map_store_err)?;

    Ok(EpicStartInfo {
        branch_name,
        epic_dir: e_dir,
        milestone_id,
        epic_id: epic_id.to_owned(),
    })
}

/// Build the initial `.workflow/state.yml` content for an epic branch.
pub fn epic_branch_state(milestone_id: &str, epic_id: &str) -> crate::state::State {
    crate::state::State {
        phase: Phase::Epic,
        step: Some("analysis".to_owned()),
        current_milestone: Some(milestone_id.to_owned()),
        epic: Some(epic_id.to_owned()),
        active_meeting: None,
    }
}

/// Finalize an epic after retrospective.
///
/// Validates we're in the retrospective step and updates epic status to
/// `completed`.
pub fn epic_complete(store: &impl Store, epic_id: &str) -> CmdResult<()> {
    let state = status(store)?;

    if state.phase != Phase::Epic {
        return Err(super::CmdError::Store(
            "epic complete requires epic phase".to_owned(),
        ));
    }
    if state.step.as_deref() != Some("retrospective") {
        return Err(super::CmdError::Store(
            "epic complete requires retrospective step".to_owned(),
        ));
    }

    // Resolve and update epic front matter
    let path = resolve::artifact_path(store, epic_id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(map_store_err)?;
    let updated = replace_front_matter_field(&content, "status", "completed");
    let updated = replace_front_matter_field(&updated, "updated", &time::today());
    store.write_file(&path, &updated).map_err(map_store_err)
}

/// Summary of an epic for list output.
#[derive(Debug, Serialize)]
pub struct EpicSummary {
    pub id: String,
    pub status: String,
    pub path: String,
}

/// List epics, optionally filtered by milestone ID and/or status.
pub fn epic_list(
    store: &impl Store,
    milestone_filter: Option<&str>,
    status_filter: Option<&str>,
) -> CmdResult<Vec<EpicSummary>> {
    status(store)?;

    let m_dir = if let Some(m_id) = milestone_filter {
        store
            .find_dir(&format!("milestones/{m_id}-*"))
            .ok_or_else(|| {
                super::CmdError::Store(format!("milestone directory not found for {m_id}"))
            })?
    } else {
        resolve::current_milestone_dir(store).map_err(super::CmdError::Store)?
    };

    let epics_dir = format!("{m_dir}/epics");
    let paths = store.find_all_files_deep(&epics_dir, "epic.md");
    let mut epics = Vec::new();

    for path in paths {
        let content = store.read_file(&path).map_err(map_store_err)?;
        let id = extract_field(&content, "id").unwrap_or_default();
        let epic_status = extract_field(&content, "status").unwrap_or_default();

        if let Some(filter) = status_filter {
            if epic_status != filter {
                continue;
            }
        }

        epics.push(EpicSummary {
            id,
            status: epic_status,
            path,
        });
    }

    Ok(epics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{create_epic, create_milestone, init},
        state::State,
        store::MemStore,
    };

    fn store_at_epic_planning() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();

        let state = State {
            phase: Phase::Execution,
            step: Some("epic-planning".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: None,
            active_meeting: None,
        };
        store.write_state(&state).unwrap();

        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        store
    }

    fn store_at_retrospective() -> MemStore {
        let store = store_at_epic_planning();
        create_epic(&store, "milestones/M-001-mvp", "billing").unwrap();

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

    // -- epic_start --

    #[test]
    fn epic_start_returns_branch_name() {
        let store = store_at_epic_planning();
        let info = epic_start(&store, "E-001").unwrap();
        assert_eq!(info.branch_name, "M-001/E-001-auth");
    }

    #[test]
    fn epic_start_returns_epic_dir() {
        let store = store_at_epic_planning();
        let info = epic_start(&store, "E-001").unwrap();
        assert_eq!(info.epic_dir, "milestones/M-001-mvp/epics/E-001-auth");
    }

    #[test]
    fn epic_start_sets_status_active() {
        let store = store_at_epic_planning();
        epic_start(&store, "E-001").unwrap();
        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/epic.md")
            .unwrap();
        assert!(content.contains("status: active"));
    }

    #[test]
    fn epic_start_updates_timestamp() {
        let store = store_at_epic_planning();
        epic_start(&store, "E-001").unwrap();
        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/epic.md")
            .unwrap();
        assert!(content.contains(&format!("updated: {}", time::today())));
    }

    #[test]
    fn epic_start_fails_outside_epic_planning() {
        let store = MemStore::new();
        init(&store).unwrap();
        assert!(epic_start(&store, "E-001").is_err());
    }

    #[test]
    fn epic_start_fails_without_milestone() {
        let store = MemStore::new();
        init(&store).unwrap();
        let state = State {
            phase: Phase::Execution,
            step: Some("epic-planning".to_owned()),
            current_milestone: None,
            epic: None,
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        assert!(epic_start(&store, "E-001").is_err());
    }

    #[test]
    fn epic_start_fails_for_nonexistent_epic() {
        let store = store_at_epic_planning();
        assert!(epic_start(&store, "E-999").is_err());
    }

    // -- epic_branch_state --

    #[test]
    fn epic_branch_state_starts_at_analysis() {
        let state = epic_branch_state("M-001", "E-001");
        assert_eq!(state.phase, Phase::Epic);
        assert_eq!(state.step.as_deref(), Some("analysis"));
        assert_eq!(state.current_milestone.as_deref(), Some("M-001"));
        assert_eq!(state.epic.as_deref(), Some("E-001"));
    }

    // -- epic_complete --

    #[test]
    fn epic_complete_sets_status_completed() {
        let store = store_at_retrospective();
        epic_complete(&store, "E-001").unwrap();
        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/epic.md")
            .unwrap();
        assert!(content.contains("status: completed"));
    }

    #[test]
    fn epic_complete_updates_timestamp() {
        let store = store_at_retrospective();
        epic_complete(&store, "E-001").unwrap();
        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/epic.md")
            .unwrap();
        assert!(content.contains(&format!("updated: {}", time::today())));
    }

    #[test]
    fn epic_complete_fails_outside_retrospective() {
        let store = store_at_epic_planning();
        let state = State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        assert!(epic_complete(&store, "E-001").is_err());
    }

    #[test]
    fn epic_complete_fails_outside_epic_phase() {
        let store = store_at_epic_planning();
        assert!(epic_complete(&store, "E-001").is_err());
    }

    // -- epic_list --

    #[test]
    fn epic_list_returns_all_epics() {
        let store = store_at_epic_planning();
        create_epic(&store, "milestones/M-001-mvp", "billing").unwrap();
        let epics = epic_list(&store, None, None).unwrap();
        assert_eq!(epics.len(), 2);
    }

    #[test]
    fn epic_list_filters_by_status() {
        let store = store_at_epic_planning();
        epic_start(&store, "E-001").unwrap();

        // Reset state to epic-planning so we can call epic_list
        let state = State {
            phase: Phase::Execution,
            step: Some("epic-planning".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: None,
            active_meeting: None,
        };
        store.write_state(&state).unwrap();

        create_epic(&store, "milestones/M-001-mvp", "billing").unwrap();

        let active = epic_list(&store, None, Some("active")).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "E-001");

        let draft = epic_list(&store, None, Some("draft")).unwrap();
        assert_eq!(draft.len(), 1);
        assert_eq!(draft[0].id, "E-002");
    }

    #[test]
    fn epic_list_filters_by_milestone() {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_milestone(&store, "v2").unwrap();

        let state = State {
            phase: Phase::Execution,
            step: Some("epic-planning".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: None,
            active_meeting: None,
        };
        store.write_state(&state).unwrap();

        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        create_epic(&store, "milestones/M-002-v2", "billing").unwrap();

        let m1_epics = epic_list(&store, Some("M-001"), None).unwrap();
        assert_eq!(m1_epics.len(), 1);
        assert_eq!(m1_epics[0].id, "E-001");

        let m2_epics = epic_list(&store, Some("M-002"), None).unwrap();
        assert_eq!(m2_epics.len(), 1);
        assert_eq!(m2_epics[0].id, "E-002");
    }
}
