use serde::Serialize;

use super::{
    CmdResult, extract_field, map_store_err, replace_front_matter_field, workflow::status,
};
use crate::{resolve, store::Store, time};

/// Update a task's front matter status field and the `updated` timestamp.
fn update_task_status(store: &impl Store, task_id: &str, new_status: &str) -> CmdResult<()> {
    status(store)?;
    let path = resolve::artifact_path(store, task_id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(map_store_err)?;
    let updated = replace_front_matter_field(&content, "status", new_status);
    let updated = replace_front_matter_field(&updated, "updated", &time::today());
    store.write_file(&path, &updated).map_err(map_store_err)
}

/// Mark a task as in-progress.
pub fn task_start(store: &impl Store, task_id: &str) -> CmdResult<()> {
    update_task_status(store, task_id, "in-progress")
}

/// Mark a task as completed.
pub fn task_complete(store: &impl Store, task_id: &str) -> CmdResult<()> {
    update_task_status(store, task_id, "completed")
}

/// Mark a task as blocked with a reason.
pub fn task_block(store: &impl Store, task_id: &str, reason: &str) -> CmdResult<()> {
    status(store)?;
    let path = resolve::artifact_path(store, task_id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(map_store_err)?;
    let updated = replace_front_matter_field(&content, "status", "blocked");
    let updated = replace_front_matter_field(&updated, "blocked_reason", reason);
    let updated = replace_front_matter_field(&updated, "updated", &time::today());
    store.write_file(&path, &updated).map_err(map_store_err)
}

/// Unblock a task (set back to pending).
pub fn task_unblock(store: &impl Store, task_id: &str) -> CmdResult<()> {
    status(store)?;
    let path = resolve::artifact_path(store, task_id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(map_store_err)?;
    let updated = replace_front_matter_field(&content, "status", "pending");
    let updated = replace_front_matter_field(&updated, "blocked_reason", "null");
    let updated = replace_front_matter_field(&updated, "updated", &time::today());
    store.write_file(&path, &updated).map_err(map_store_err)
}

/// Summary of a task for list output.
#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub status: String,
    pub blocked_reason: Option<String>,
    pub path: String,
}

/// List tasks, optionally filtered by epic ID, story ID, and/or status.
///
/// - `epic_filter`: if provided, resolves that epic's directory under the
///   current milestone instead of using the current epic from state.
/// - `story_filter`: narrows to a specific story within the resolved epic.
/// - `status_filter`: only includes tasks matching this status value.
pub fn task_list(
    store: &impl Store,
    epic_filter: Option<&str>,
    story_filter: Option<&str>,
    status_filter: Option<&str>,
) -> CmdResult<Vec<TaskSummary>> {
    status(store)?;

    let e_dir = if let Some(epic_id) = epic_filter {
        let m_dir = resolve::current_milestone_dir(store).map_err(super::CmdError::Store)?;
        store
            .find_dir(&format!("{m_dir}/epics/{epic_id}-*"))
            .ok_or_else(|| {
                super::CmdError::Store(format!("epic directory not found for {epic_id}"))
            })?
    } else {
        resolve::current_epic_dir(store).map_err(super::CmdError::Store)?
    };

    let root = if let Some(story_id) = story_filter {
        store
            .find_dir(&format!("{e_dir}/stories/{story_id}-*"))
            .ok_or_else(|| {
                super::CmdError::Store(format!("story directory not found for {story_id}"))
            })?
    } else {
        e_dir
    };

    let paths = store.find_all_files_deep(&root, "T-");
    let mut tasks = Vec::new();

    for path in paths {
        let content = store.read_file(&path).map_err(map_store_err)?;
        let id = extract_field(&content, "id").unwrap_or_default();
        let task_status = extract_field(&content, "status").unwrap_or_default();
        let blocked_reason = extract_field(&content, "blocked_reason");

        if let Some(filter) = status_filter {
            if task_status != filter {
                continue;
            }
        }

        tasks.push(TaskSummary {
            id,
            status: task_status,
            blocked_reason,
            path,
        });
    }

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{create_epic, create_milestone, create_story, create_task, init},
        state::{Phase, State},
        store::MemStore,
    };

    /// Set up a store with an initialized project, a milestone, epic, story,
    /// and a single task — with state pointing at the epic.
    fn store_with_task() -> (MemStore, String) {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();

        // Set state to epic branch so artifact_path can resolve T- IDs
        let state = State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();

        let id = create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "ui",
        )
        .unwrap();
        (store, id.to_string())
    }

    fn task_content(store: &MemStore) -> String {
        store
            .read_file(
                "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/tasks/T-001-ui.md",
            )
            .unwrap()
    }

    // -- Status transitions --

    #[test]
    fn task_start_sets_in_progress() {
        let (store, id) = store_with_task();
        task_start(&store, &id).unwrap();
        assert!(task_content(&store).contains("status: in-progress"));
    }

    #[test]
    fn task_complete_sets_completed() {
        let (store, id) = store_with_task();
        task_complete(&store, &id).unwrap();
        assert!(task_content(&store).contains("status: completed"));
    }

    #[test]
    fn task_block_sets_blocked_with_reason() {
        let (store, id) = store_with_task();
        task_block(&store, &id, "waiting on API").unwrap();
        let content = task_content(&store);
        assert!(content.contains("status: blocked"));
        assert!(content.contains("blocked_reason: waiting on API"));
    }

    #[test]
    fn task_unblock_sets_pending_and_clears_reason() {
        let (store, id) = store_with_task();
        task_block(&store, &id, "waiting").unwrap();
        task_unblock(&store, &id).unwrap();
        let content = task_content(&store);
        assert!(content.contains("status: pending"));
        assert!(content.contains("blocked_reason: null"));
    }

    // -- Timestamp updates --

    #[test]
    fn status_change_updates_timestamp() {
        let (store, id) = store_with_task();
        task_start(&store, &id).unwrap();
        let today = time::today();
        assert!(task_content(&store).contains(&format!("updated: {today}")));
    }

    #[test]
    fn block_updates_timestamp() {
        let (store, id) = store_with_task();
        task_block(&store, &id, "reason").unwrap();
        let today = time::today();
        assert!(task_content(&store).contains(&format!("updated: {today}")));
    }

    #[test]
    fn unblock_updates_timestamp() {
        let (store, id) = store_with_task();
        task_block(&store, &id, "reason").unwrap();
        task_unblock(&store, &id).unwrap();
        let today = time::today();
        assert!(task_content(&store).contains(&format!("updated: {today}")));
    }

    // -- replace_front_matter_field --

    #[test]
    fn replace_field_changes_value() {
        let content = "---\nstatus: pending\nid: T-001\n---\n";
        let result = replace_front_matter_field(content, "status", "completed");
        assert!(result.contains("status: completed"));
        assert!(result.contains("id: T-001"));
    }

    #[test]
    fn replace_field_preserves_other_fields() {
        let content = "---\nid: T-001\nstatus: pending\nblocked_reason: null\n---\n";
        let result = replace_front_matter_field(content, "status", "blocked");
        assert!(result.contains("id: T-001"));
        assert!(result.contains("blocked_reason: null"));
        assert!(result.contains("status: blocked"));
    }

    // -- extract_field --

    #[test]
    fn extract_field_returns_value() {
        let content = "---\nid: T-001\nstatus: pending\n---\n";
        assert_eq!(extract_field(content, "id"), Some("T-001".to_owned()));
        assert_eq!(extract_field(content, "status"), Some("pending".to_owned()));
    }

    #[test]
    fn extract_field_returns_none_for_null() {
        let content = "---\nblocked_reason: null\n---\n";
        assert_eq!(extract_field(content, "blocked_reason"), None);
    }

    #[test]
    fn extract_field_returns_none_for_missing() {
        let content = "---\nid: T-001\n---\n";
        assert_eq!(extract_field(content, "status"), None);
    }

    // -- task_list --

    #[test]
    fn task_list_returns_all_tasks() {
        let (store, _) = store_with_task();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "api",
        )
        .unwrap();

        let tasks = task_list(&store, None, None, None).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn task_list_filters_by_status() {
        let (store, id) = store_with_task();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "api",
        )
        .unwrap();
        task_start(&store, &id).unwrap();

        let pending = task_list(&store, None, None, Some("pending")).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "T-002");

        let in_progress = task_list(&store, None, None, Some("in-progress")).unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].id, "T-001");
    }

    #[test]
    fn task_list_filters_by_story() {
        let (store, _) = store_with_task();
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "signup").unwrap();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-002-signup",
            "form",
        )
        .unwrap();

        let login_tasks = task_list(&store, None, Some("S-001"), None).unwrap();
        assert_eq!(login_tasks.len(), 1);
        assert_eq!(login_tasks[0].id, "T-001");

        let signup_tasks = task_list(&store, None, Some("S-002"), None).unwrap();
        assert_eq!(signup_tasks.len(), 1);
        assert_eq!(signup_tasks[0].id, "T-002");
    }

    #[test]
    fn task_list_blocked_includes_reason() {
        let (store, id) = store_with_task();
        task_block(&store, &id, "dependency missing").unwrap();

        let tasks = task_list(&store, None, None, Some("blocked")).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].blocked_reason.as_deref(),
            Some("dependency missing")
        );
    }

    #[test]
    fn task_list_filters_by_epic() {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "billing").unwrap();

        // Create stories and tasks in both epics
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "ui",
        )
        .unwrap();

        create_story(&store, "milestones/M-001-mvp/epics/E-002-billing", "pay").unwrap();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-002-billing/stories/S-002-pay",
            "stripe",
        )
        .unwrap();

        // State points at E-001
        let state = State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();

        // Default (no filter) → current epic (E-001)
        let tasks = task_list(&store, None, None, None).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "T-001");

        // Explicit --epic E-002 → billing tasks
        let tasks = task_list(&store, Some("E-002"), None, None).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "T-002");
    }
}
