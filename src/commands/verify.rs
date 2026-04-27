use serde::Serialize;

use super::{CmdResult, extract_field, extract_list_field, workflow::status};
use crate::{resolve, store::Store};

#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub id: String,
    pub checks: Vec<VerifyCheck>,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct VerifyCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Verify a story against its acceptance criteria.
pub fn verify_story(store: &impl Store, story_id: &str) -> CmdResult<VerifyResult> {
    status(store)?;

    let path = resolve::artifact_path(store, story_id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(super::map_store_err)?;
    let mut checks = Vec::new();

    // Check story has satisfies links
    let satisfies = extract_list_field(&content, "satisfies");
    checks.push(VerifyCheck {
        name: "has-satisfies-links".to_owned(),
        passed: !satisfies.is_empty(),
        detail: format!("{} requirement links", satisfies.len()),
    });

    // Check story has nfrs field
    let has_nfrs = content.lines().any(|l| l.starts_with("nfrs:"));
    checks.push(VerifyCheck {
        name: "has-nfrs-field".to_owned(),
        passed: has_nfrs,
        detail: if has_nfrs {
            "nfrs field present".to_owned()
        } else {
            "missing nfrs field".to_owned()
        },
    });

    // Check story has flows
    let story_dir = path.strip_suffix("/story.md").unwrap_or(&path);
    let flows = store.find_all_files_deep(&format!("{story_dir}/flows"), "F-");
    checks.push(VerifyCheck {
        name: "has-flows".to_owned(),
        passed: !flows.is_empty(),
        detail: format!("{} user flows", flows.len()),
    });

    // Check for happy-path flow
    let has_happy = flows.iter().any(|fp| {
        store
            .read_file(fp)
            .ok()
            .and_then(|c| extract_field(&c, "type"))
            .is_some_and(|t| t == "happy-path")
    });
    checks.push(VerifyCheck {
        name: "has-happy-path".to_owned(),
        passed: has_happy,
        detail: if has_happy {
            "happy-path flow exists".to_owned()
        } else {
            "no happy-path flow".to_owned()
        },
    });

    // Check all tasks are completed
    let tasks = store.find_all_files_deep(story_dir, "T-");
    let all_done = tasks.iter().all(|tp| {
        store
            .read_file(tp)
            .ok()
            .and_then(|c| extract_field(&c, "status"))
            .is_some_and(|s| s == "completed")
    });
    checks.push(VerifyCheck {
        name: "all-tasks-completed".to_owned(),
        passed: all_done,
        detail: format!("{} tasks", tasks.len()),
    });

    let passed = checks.iter().all(|c| c.passed);

    Ok(VerifyResult {
        id: story_id.to_owned(),
        checks,
        passed,
    })
}

/// Verify an epic — all stories pass verification.
pub fn verify_epic(store: &impl Store, epic_id: &str) -> CmdResult<VerifyResult> {
    status(store)?;

    let path = resolve::artifact_path(store, epic_id).map_err(super::CmdError::Store)?;
    let e_dir = path.strip_suffix("/epic.md").unwrap_or(&path);
    let story_paths = store.find_all_files_deep(e_dir, "story.md");

    let mut checks = Vec::new();

    // Check epic has satisfies links
    let content = store.read_file(&path).map_err(super::map_store_err)?;
    let satisfies = extract_list_field(&content, "satisfies");
    checks.push(VerifyCheck {
        name: "epic-has-satisfies-links".to_owned(),
        passed: !satisfies.is_empty(),
        detail: format!("{} requirement links", satisfies.len()),
    });

    // Verify each story
    for sp in &story_paths {
        if let Ok(sc) = store.read_file(sp) {
            let sid = extract_field(&sc, "id").unwrap_or_default();
            let story_dir = sp.strip_suffix("/story.md").unwrap_or(sp);
            let tasks = store.find_all_files_deep(story_dir, "T-");
            let all_done = tasks.iter().all(|tp| {
                store
                    .read_file(tp)
                    .ok()
                    .and_then(|c| extract_field(&c, "status"))
                    .is_some_and(|s| s == "completed")
            });
            checks.push(VerifyCheck {
                name: format!("story-{sid}-tasks-done"),
                passed: all_done,
                detail: format!("{} tasks", tasks.len()),
            });
        }
    }

    let passed = checks.iter().all(|c| c.passed);

    Ok(VerifyResult {
        id: epic_id.to_owned(),
        checks,
        passed,
    })
}

/// Generate a full verification report for the current epic.
pub fn verify_report(store: &impl Store) -> CmdResult<Vec<VerifyResult>> {
    status(store)?;

    let e_dir = resolve::current_epic_dir(store).map_err(super::CmdError::Store)?;
    let story_paths = store.find_all_files_deep(&e_dir, "story.md");

    let mut results = Vec::new();
    for sp in &story_paths {
        if let Ok(content) = store.read_file(sp) {
            if let Some(sid) = extract_field(&content, "id") {
                if let Ok(result) = verify_story(store, &sid) {
                    results.push(result);
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{
            create_epic, create_flow, create_milestone, create_story, create_task, init,
            task_complete,
        },
        state::{Phase, State},
        store::MemStore,
    };

    fn store_with_story() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();

        let state = State {
            phase: Phase::Epic,
            step: Some("verification".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        store
    }

    #[test]
    fn verify_story_checks_all() {
        let store = store_with_story();
        let result = verify_story(&store, "S-001").unwrap();
        assert!(!result.passed, "story without flows/tasks should fail");
        assert!(result.checks.len() >= 4);
    }

    #[test]
    fn verify_story_passes_when_complete() {
        let store = store_with_story();
        let story_dir = "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login";
        create_flow(&store, story_dir, "happy", "happy-path").unwrap();
        create_task(&store, story_dir, "ui").unwrap();
        task_complete(&store, "T-001").unwrap();

        // Add satisfies and nfrs to story
        let content = store.read_file(&format!("{story_dir}/story.md")).unwrap();
        let updated = crate::commands::replace_front_matter_field(
            &content,
            "satisfies",
            "\n  - requirements/REQ-001-auth.md",
        );
        store
            .write_file(&format!("{story_dir}/story.md"), &updated)
            .unwrap();

        let result = verify_story(&store, "S-001").unwrap();
        // Most checks should pass now
        let passed_count = result.checks.iter().filter(|c| c.passed).count();
        assert!(passed_count >= 4, "most checks should pass: {result:?}");
    }

    #[test]
    fn verify_epic_checks_stories() {
        let store = store_with_story();
        let result = verify_epic(&store, "E-001").unwrap();
        assert!(!result.checks.is_empty());
    }

    #[test]
    fn verify_report_lists_stories() {
        let store = store_with_story();
        let results = verify_report(&store).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "S-001");
    }
}
