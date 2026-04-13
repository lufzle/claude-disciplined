use super::{CmdError, CmdResult, map_store_err, workflow::status};
use crate::{meeting as meeting_ops, store::Store};

/// Start a new meeting in the current step.
pub fn meeting_start(store: &impl Store, topic: &str) -> CmdResult<String> {
    let mut state = status(store)?;
    if state.active_meeting.is_some() {
        return Err(CmdError::Store(
            "a meeting is already active (end it first)".to_owned(),
        ));
    }

    let step = state.step.as_deref().unwrap_or("");
    let epic_dir = crate::resolve::current_epic_dir(store).ok();
    let meetings_dir =
        meeting_ops::meetings_dir_for_step(step, epic_dir.as_deref()).map_err(CmdError::Store)?;

    let sequence = meeting_ops::count_meetings(store, &meetings_dir) + 1;
    let folder_name = meeting_ops::create_meeting_folder(store, &meetings_dir, sequence, topic)
        .map_err(CmdError::Store)?;

    state.active_meeting = Some(folder_name.clone());
    store.write_state(&state).map_err(map_store_err)?;

    Ok(format!("{meetings_dir}/{folder_name}"))
}

/// End the currently active meeting.
pub fn meeting_end(store: &impl Store) -> CmdResult<()> {
    let mut state = status(store)?;
    if state.active_meeting.is_none() {
        return Err(CmdError::Store("no active meeting to end".to_owned()));
    }
    state.active_meeting = None;
    store.write_state(&state).map_err(map_store_err)?;
    Ok(())
}

/// List meetings in the current step.
pub fn meeting_list(store: &impl Store) -> CmdResult<Vec<String>> {
    let state = status(store)?;
    let step = state.step.as_deref().unwrap_or("");
    let epic_dir = crate::resolve::current_epic_dir(store).ok();
    let meetings_dir =
        meeting_ops::meetings_dir_for_step(step, epic_dir.as_deref()).map_err(CmdError::Store)?;

    let count = meeting_ops::count_meetings(store, &meetings_dir);
    let mut meetings = Vec::new();
    for i in 1..=count {
        if let Some(dir) = store.find_dir(&format!("{meetings_dir}/{i:03}-*")) {
            meetings.push(dir);
        }
    }
    Ok(meetings)
}

/// Read the current active meeting's notes.
pub fn meeting_read(store: &impl Store) -> CmdResult<String> {
    let (meetings_dir, meeting_slug) = active_meeting_context(store)?;
    let notes_path = format!("{meetings_dir}/{meeting_slug}/notes.md");
    store.read_file(&notes_path).map_err(map_store_err)
}

/// Add a contribution to the current active meeting's notes.
pub fn meeting_contribute(store: &impl Store, message: &str) -> CmdResult<()> {
    let (meetings_dir, meeting_slug) = active_meeting_context(store)?;
    let notes_path = format!("{meetings_dir}/{meeting_slug}/notes.md");
    let existing = store.read_file(&notes_path).map_err(map_store_err)?;
    let updated = format!("{existing}\n{message}\n");
    store
        .write_file(&notes_path, &updated)
        .map_err(map_store_err)
}

/// Resolve the meetings directory and active meeting slug.
pub(crate) fn active_meeting_context(store: &impl Store) -> CmdResult<(String, String)> {
    let state = status(store)?;
    let meeting_slug = state
        .active_meeting
        .as_deref()
        .ok_or_else(|| CmdError::Store("no active meeting".to_owned()))?
        .to_owned();

    let step = state.step.as_deref().unwrap_or("");
    let epic_dir = crate::resolve::current_epic_dir(store).ok();
    let meetings_dir =
        meeting_ops::meetings_dir_for_step(step, epic_dir.as_deref()).map_err(CmdError::Store)?;

    Ok((meetings_dir, meeting_slug))
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
    fn start_returns_path() {
        let store = initialized_store();
        let path = meeting_start(&store, "vision").unwrap();
        assert!(path.contains("001-vision"));
    }

    #[test]
    fn start_sets_active() {
        let store = initialized_store();
        meeting_start(&store, "kickoff").unwrap();
        let state = status(&store).unwrap();
        assert_eq!(state.active_meeting.as_deref(), Some("001-kickoff"));
    }

    #[test]
    fn start_fails_if_already_active() {
        let store = initialized_store();
        meeting_start(&store, "first").unwrap();
        assert!(meeting_start(&store, "second").is_err());
    }

    #[test]
    fn end_clears_active() {
        let store = initialized_store();
        meeting_start(&store, "kickoff").unwrap();
        meeting_end(&store).unwrap();
        assert!(status(&store).unwrap().active_meeting.is_none());
    }

    #[test]
    fn end_fails_without_active() {
        let store = initialized_store();
        assert!(meeting_end(&store).is_err());
    }

    #[test]
    fn read_returns_notes() {
        let store = initialized_store();
        meeting_start(&store, "kickoff").unwrap();
        assert!(meeting_read(&store).unwrap().contains("# kickoff"));
    }

    #[test]
    fn read_fails_without_active() {
        let store = initialized_store();
        assert!(meeting_read(&store).is_err());
    }

    #[test]
    fn contribute_appends() {
        let store = initialized_store();
        meeting_start(&store, "kickoff").unwrap();
        meeting_contribute(&store, "Point one.").unwrap();
        assert!(meeting_read(&store).unwrap().contains("Point one."));
    }

    #[test]
    fn contribute_preserves_existing() {
        let store = initialized_store();
        meeting_start(&store, "kickoff").unwrap();
        meeting_contribute(&store, "First.").unwrap();
        meeting_contribute(&store, "Second.").unwrap();
        let notes = meeting_read(&store).unwrap();
        assert!(notes.contains("First."));
        assert!(notes.contains("Second."));
    }

    #[test]
    fn list_returns_meetings() {
        let store = initialized_store();
        meeting_start(&store, "first").unwrap();
        meeting_end(&store).unwrap();
        meeting_start(&store, "second").unwrap();
        meeting_end(&store).unwrap();
        assert_eq!(meeting_list(&store).unwrap().len(), 2);
    }

    #[test]
    fn list_empty_when_none() {
        let store = initialized_store();
        assert!(meeting_list(&store).unwrap().is_empty());
    }
}
