use super::{CmdResult, map_store_err, replace_front_matter_field, workflow::status};
use crate::{resolve, store::Store, time};

/// Update an artifact's status by ID.
///
/// Resolves the artifact via `artifact_path`, sets its `status` field, and
/// refreshes the `updated` timestamp.
pub fn update(store: &impl Store, id: &str, new_status: &str) -> CmdResult<()> {
    status(store)?;
    let path = resolve::artifact_path(store, id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(map_store_err)?;
    let updated = replace_front_matter_field(&content, "status", new_status);
    let updated = replace_front_matter_field(&updated, "updated", &time::today());
    store.write_file(&path, &updated).map_err(map_store_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{
            create_epic, create_flow, create_milestone, create_requirement, create_story,
            create_task, extract_field, init, propose_nfr,
        },
        state::{Phase, State},
        store::MemStore,
    };

    fn store_with_epic() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();

        let state = State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        store
    }

    // -- Requirement --

    #[test]
    fn update_requirement_status() {
        let store = store_with_epic();
        create_requirement(&store, "auth-login").unwrap();
        update(&store, "REQ-001", "superseded").unwrap();

        let content = store
            .read_file("requirements/REQ-001-auth-login.md")
            .unwrap();
        assert_eq!(
            extract_field(&content, "status"),
            Some("superseded".to_owned())
        );
    }

    // -- NFR --

    #[test]
    fn update_nfr_status() {
        let store = store_with_epic();
        propose_nfr(&store, "perf-latency").unwrap();
        update(&store, "NFR-001", "active").unwrap();

        let content = store
            .read_file("requirements/NFR-001-perf-latency.md")
            .unwrap();
        assert_eq!(extract_field(&content, "status"), Some("active".to_owned()));
    }

    // -- Milestone --

    #[test]
    fn update_milestone_status() {
        let store = store_with_epic();
        update(&store, "M-001", "active").unwrap();

        let content = store
            .read_file("milestones/M-001-mvp/milestone.md")
            .unwrap();
        assert_eq!(extract_field(&content, "status"), Some("active".to_owned()));
    }

    // -- Epic --

    #[test]
    fn update_epic_status() {
        let store = store_with_epic();
        update(&store, "E-001", "active").unwrap();

        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/epic.md")
            .unwrap();
        assert_eq!(extract_field(&content, "status"), Some("active".to_owned()));
    }

    // -- Story --

    #[test]
    fn update_story_status() {
        let store = store_with_epic();
        update(&store, "S-001", "completed").unwrap();

        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/story.md")
            .unwrap();
        assert_eq!(
            extract_field(&content, "status"),
            Some("completed".to_owned())
        );
    }

    // -- Task --

    #[test]
    fn update_task_status() {
        let store = store_with_epic();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "ui",
        )
        .unwrap();
        update(&store, "T-001", "in-progress").unwrap();

        let content = store
            .read_file(
                "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/tasks/T-001-ui.md",
            )
            .unwrap();
        assert_eq!(
            extract_field(&content, "status"),
            Some("in-progress".to_owned())
        );
    }

    // -- Flow --

    #[test]
    fn update_flow_status() {
        let store = store_with_epic();
        create_flow(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "login-ok",
            "happy-path",
        )
        .unwrap();
        update(&store, "F-001", "superseded").unwrap();

        let content = store
            .read_file(
                "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/flows/F-001-login-ok.md",
            )
            .unwrap();
        assert_eq!(
            extract_field(&content, "status"),
            Some("superseded".to_owned())
        );
    }

    // -- Timestamp --

    #[test]
    fn update_refreshes_timestamp() {
        let store = store_with_epic();
        update(&store, "M-001", "active").unwrap();

        let content = store
            .read_file("milestones/M-001-mvp/milestone.md")
            .unwrap();
        assert_eq!(extract_field(&content, "updated"), Some(time::today()));
    }

    // -- Preserves other fields --

    #[test]
    fn update_preserves_other_fields() {
        let store = store_with_epic();
        update(&store, "M-001", "active").unwrap();

        let content = store
            .read_file("milestones/M-001-mvp/milestone.md")
            .unwrap();
        assert_eq!(extract_field(&content, "id"), Some("M-001".to_owned()));
        assert!(content.contains("superseded_by: null"));
    }

    // -- Error cases --

    #[test]
    fn update_unknown_id_fails() {
        let store = store_with_epic();
        assert!(update(&store, "REQ-999", "active").is_err());
    }

    #[test]
    fn update_invalid_prefix_fails() {
        let store = store_with_epic();
        assert!(update(&store, "NOPE-001", "active").is_err());
    }

    #[test]
    fn update_requires_initialization() {
        let store = MemStore::new();
        assert!(update(&store, "REQ-001", "active").is_err());
    }
}
