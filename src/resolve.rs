use crate::store::Store;

/// Resolve the directory path for the current milestone.
///
/// Reads `current_milestone` from state and finds the matching directory.
pub fn current_milestone_dir(store: &impl Store) -> Result<String, String> {
    let state = store.read_state().map_err(|e| e.to_string())?;
    let milestone_id = state
        .current_milestone
        .as_deref()
        .ok_or("no current milestone set in state")?;
    store
        .find_dir(&format!("milestones/{milestone_id}-*"))
        .ok_or_else(|| format!("milestone directory not found for {milestone_id}"))
}

/// Resolve the directory path for the current epic.
///
/// Reads `milestone` and `epic` from state (epic branch) and finds the
/// matching directory.
pub fn current_epic_dir(store: &impl Store) -> Result<String, String> {
    let state = store.read_state().map_err(|e| e.to_string())?;
    let milestone_id = state
        .current_milestone
        .as_deref()
        .ok_or("no milestone in state")?;
    let milestone_dir = store
        .find_dir(&format!("milestones/{milestone_id}-*"))
        .ok_or_else(|| format!("milestone directory not found for {milestone_id}"))?;
    let epic_id = state
        .epic
        .as_deref()
        .ok_or("no current epic set in state")?;
    store
        .find_dir(&format!("{milestone_dir}/epics/{epic_id}-*"))
        .ok_or_else(|| format!("epic directory not found for {epic_id}"))
}

/// Resolve the file path for an artifact by its ID.
///
/// ID-based artifacts (REQ, NFR, M, E, S, T, F) are found by scanning
/// the appropriate directory for files/dirs matching the ID prefix.
pub fn artifact_path(store: &impl Store, id: &str) -> Result<String, String> {
    if id.starts_with("REQ-") || id.starts_with("NFR-") {
        store
            .find_file(&format!("requirements/{id}-*"))
            .ok_or_else(|| format!("file not found for {id}"))
    } else if id.starts_with("M-") {
        let m_dir = store
            .find_dir(&format!("milestones/{id}-*"))
            .ok_or_else(|| format!("milestone not found for {id}"))?;
        Ok(format!("{m_dir}/milestone.md"))
    } else if id.starts_with("E-") {
        let state = store.read_state().map_err(|e| e.to_string())?;
        let m_id = state
            .current_milestone
            .as_deref()
            .ok_or("no milestone in state")?;
        let m_dir = store
            .find_dir(&format!("milestones/{m_id}-*"))
            .ok_or_else(|| format!("milestone dir not found for {m_id}"))?;
        let e_dir = store
            .find_dir(&format!("{m_dir}/epics/{id}-*"))
            .ok_or_else(|| format!("epic not found for {id}"))?;
        Ok(format!("{e_dir}/epic.md"))
    } else if id.starts_with("S-") {
        let e_dir = current_epic_dir(store)?;
        let s_dir = store
            .find_dir(&format!("{e_dir}/stories/{id}-*"))
            .ok_or_else(|| format!("story not found for {id}"))?;
        Ok(format!("{s_dir}/story.md"))
    } else if id.starts_with("T-") || id.starts_with("F-") {
        let e_dir = current_epic_dir(store)?;
        store
            .find_file_deep(&e_dir, &format!("{id}-"))
            .ok_or_else(|| format!("{id} not found under {e_dir}"))
    } else {
        Err(format!("unknown ID prefix: {id}"))
    }
}

/// Resolve the path for a singleton artifact scoped to the current epic.
pub fn epic_artifact_path(store: &impl Store, name: &str) -> Result<String, String> {
    let e_dir = current_epic_dir(store)?;
    Ok(format!("{e_dir}/{name}.md"))
}

/// Resolve the path for a project-level singleton artifact.
pub fn singleton_path(name: &str) -> String {
    match name {
        "product-brief" => "product-brief/product-brief.md".to_owned(),
        "architecture" => "architecture/architecture.md".to_owned(),
        "ux-foundations" => "ux-foundations/ux-foundations.md".to_owned(),
        "tech-stack" => "tech-stack/tech-stack.md".to_owned(),
        "roadmap" => "roadmap/roadmap.md".to_owned(),
        _ => format!("{name}.md"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands,
        state::{Phase, State},
        store::MemStore,
    };

    #[test]
    fn current_milestone_dir_resolves() {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::create_milestone(&store, "mvp").unwrap();
        // Set current_milestone in state
        let mut state = store.read_state().unwrap();
        state.current_milestone = Some("M-001".to_owned());
        store.write_state(&state).unwrap();

        let dir = current_milestone_dir(&store).unwrap();
        assert_eq!(dir, "milestones/M-001-mvp");
    }

    #[test]
    fn current_milestone_dir_fails_without_milestone() {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        assert!(current_milestone_dir(&store).is_err());
    }

    #[test]
    fn current_epic_dir_resolves() {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::create_milestone(&store, "mvp").unwrap();
        commands::create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        // Set state to epic branch
        let state = State {
            phase: Phase::Epic,
            step: Some("analysis".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();

        let dir = current_epic_dir(&store).unwrap();
        assert_eq!(dir, "milestones/M-001-mvp/epics/E-001-auth");
    }

    #[test]
    fn current_epic_dir_fails_without_epic() {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::create_milestone(&store, "mvp").unwrap();
        let mut state = store.read_state().unwrap();
        state.current_milestone = Some("M-001".to_owned());
        store.write_state(&state).unwrap();
        assert!(current_epic_dir(&store).is_err());
    }
}
