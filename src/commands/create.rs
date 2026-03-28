use super::{CmdResult, map_store_err, workflow::status};
use crate::{
    artifacts,
    id::{Id, Prefix},
    store::Store,
};

/// Allocate an ID, write artifact file, persist counters.
fn create_artifact(
    store: &impl Store,
    prefix: Prefix,
    build: impl FnOnce(&Id) -> (String, String, Vec<String>),
) -> CmdResult<Id> {
    status(store)?;
    let mut counters = store.read_counters().map_err(map_store_err)?;
    let id = counters.next(prefix).map_err(map_store_err)?;
    let (path, content, dirs) = build(&id);
    for dir in &dirs {
        store.ensure_dir(dir).map_err(map_store_err)?;
    }
    store.write_file(&path, &content).map_err(map_store_err)?;
    store.write_counters(&counters).map_err(map_store_err)?;
    Ok(id)
}

pub fn create_requirement(store: &impl Store, slug: &str) -> CmdResult<Id> {
    create_artifact(store, Prefix::Req, |id| {
        let path = artifacts::requirement_path(id, slug);
        let content = artifacts::requirement_front_matter(id);
        (path, content, vec!["requirements".to_owned()])
    })
}

pub fn propose_nfr(store: &impl Store, slug: &str) -> CmdResult<Id> {
    create_artifact(store, Prefix::Nfr, |id| {
        let path = artifacts::nfr_path(id, slug);
        let content = artifacts::nfr_front_matter(id);
        (path, content, vec!["requirements".to_owned()])
    })
}

pub fn create_milestone(store: &impl Store, slug: &str) -> CmdResult<Id> {
    create_artifact(store, Prefix::M, |id| {
        let dir = artifacts::milestone_dir(id, slug);
        let path = artifacts::milestone_path(id, slug);
        let content = artifacts::milestone_front_matter(id);
        let meetings = format!("{dir}/planning/meetings");
        (path, content, vec![dir, meetings])
    })
}

pub fn create_epic(store: &impl Store, milestone_dir: &str, slug: &str) -> CmdResult<Id> {
    create_artifact(store, Prefix::E, |id| {
        let dir = artifacts::epic_dir(milestone_dir, id, slug);
        let path = artifacts::epic_path(milestone_dir, id, slug);
        let content = artifacts::epic_front_matter(id);
        (path, content, vec![dir])
    })
}

pub fn create_story(store: &impl Store, epic_dir: &str, slug: &str) -> CmdResult<Id> {
    create_artifact(store, Prefix::S, |id| {
        let dir = artifacts::story_dir(epic_dir, id, slug);
        let path = artifacts::story_path(epic_dir, id, slug);
        let content = artifacts::story_front_matter(id);
        let tasks = format!("{dir}/tasks");
        let flows = format!("{dir}/flows");
        (path, content, vec![dir, tasks, flows])
    })
}

pub fn create_task(store: &impl Store, story_dir: &str, slug: &str) -> CmdResult<Id> {
    create_artifact(store, Prefix::T, |id| {
        let path = artifacts::task_path(story_dir, id, slug);
        let content = artifacts::task_front_matter(id);
        let tasks = format!("{story_dir}/tasks");
        (path, content, vec![tasks])
    })
}

pub fn create_flow(
    store: &impl Store,
    story_dir: &str,
    slug: &str,
    flow_type: &str,
) -> CmdResult<Id> {
    create_artifact(store, Prefix::F, |id| {
        let path = artifacts::flow_path(story_dir, id, slug);
        let content = artifacts::flow_front_matter(id, flow_type);
        let flows = format!("{story_dir}/flows");
        (path, content, vec![flows])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands::init, store::MemStore};

    fn initialized_store() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        store
    }

    #[test]
    fn create_requirement_returns_id() {
        let store = initialized_store();
        assert_eq!(
            create_requirement(&store, "apps").unwrap().to_string(),
            "REQ-001"
        );
    }

    #[test]
    fn create_requirement_writes_file() {
        let store = initialized_store();
        create_requirement(&store, "apps").unwrap();
        let content = store.read_file("requirements/REQ-001-apps.md").unwrap();
        assert!(content.contains("id: REQ-001"));
    }

    #[test]
    fn create_requirement_increments() {
        let store = initialized_store();
        create_requirement(&store, "a").unwrap();
        assert_eq!(
            create_requirement(&store, "b").unwrap().to_string(),
            "REQ-002"
        );
    }

    #[test]
    fn propose_nfr_starts_draft() {
        let store = initialized_store();
        propose_nfr(&store, "fast").unwrap();
        let content = store.read_file("requirements/NFR-001-fast.md").unwrap();
        assert!(content.contains("status: draft"));
    }

    #[test]
    fn create_milestone_writes_file() {
        let store = initialized_store();
        create_milestone(&store, "mvp").unwrap();
        let content = store
            .read_file("milestones/M-001-mvp/milestone.md")
            .unwrap();
        assert!(content.contains("id: M-001"));
    }

    #[test]
    fn create_epic_writes_file() {
        let store = initialized_store();
        create_epic(&store, "milestones/M-001", "auth").unwrap();
        let content = store
            .read_file("milestones/M-001/epics/E-001-auth/epic.md")
            .unwrap();
        assert!(content.contains("id: E-001"));
    }

    #[test]
    fn create_story_has_nfrs() {
        let store = initialized_store();
        create_story(&store, "epics/E-001", "login").unwrap();
        let content = store
            .read_file("epics/E-001/stories/S-001-login/story.md")
            .unwrap();
        assert!(content.contains("nfrs: []"));
    }

    #[test]
    fn create_task_has_pending_status() {
        let store = initialized_store();
        create_task(&store, "stories/S-001", "ui").unwrap();
        let content = store.read_file("stories/S-001/tasks/T-001-ui.md").unwrap();
        assert!(content.contains("status: pending"));
    }

    #[test]
    fn create_flow_has_type() {
        let store = initialized_store();
        create_flow(&store, "stories/S-001", "happy", "happy-path").unwrap();
        let content = store
            .read_file("stories/S-001/flows/F-001-happy.md")
            .unwrap();
        assert!(content.contains("type: happy-path"));
    }
}
