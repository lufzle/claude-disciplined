use crate::{store::Store, time};

/// Create the meeting folder structure and initial files.
///
/// Meeting folder layout:
/// ```text
/// NNN-topic-slug/
///   notes.md
///   decisions.ndjson
///   actions/
///     action-items.ndjson
/// ```
pub fn create_meeting_folder(
    store: &impl Store,
    parent: &str,
    sequence: u32,
    topic: &str,
) -> Result<String, String> {
    let folder_name = format!("{sequence:03}-{topic}");
    let folder_path = format!("{parent}/{folder_name}");

    store.ensure_dir(&folder_path).map_err(|e| e.to_string())?;
    store
        .ensure_dir(&format!("{folder_path}/actions"))
        .map_err(|e| e.to_string())?;

    let notes = format!(
        "# {topic}\n\n\
         - **Date**: {}\n\
         - **Participants**: \n\
         - **Topic**: {topic}\n\n\
         ## Discussion\n\n\
         ## Decisions\n\n\
         ## Action Items\n",
        time::today()
    );

    store
        .write_file(&format!("{folder_path}/notes.md"), &notes)
        .map_err(|e| e.to_string())?;
    store
        .write_file(&format!("{folder_path}/decisions.ndjson"), "")
        .map_err(|e| e.to_string())?;
    store
        .write_file(&format!("{folder_path}/actions/action-items.ndjson"), "")
        .map_err(|e| e.to_string())?;

    Ok(folder_name)
}

/// Count existing meetings in a meetings directory.
pub fn count_meetings(store: &impl Store, meetings_dir: &str) -> u32 {
    // Try reading the action-items file for sequential meeting numbers.
    // For MemStore, count distinct meeting folders by scanning file keys.
    // For simplicity, count files matching NNN-*/notes.md pattern.
    let mut count = 0u32;
    loop {
        let candidate = format!("{meetings_dir}/{:03}-", count + 1);
        // Check if any file exists under this prefix
        if store.find_dir(&format!("{candidate}*")).is_some() {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Resolve the meetings directory for the current workflow step.
pub fn meetings_dir_for_step(step: &str, epic_dir: Option<&str>) -> Result<String, String> {
    match step {
        "product-brief" => Ok("product-brief/meetings".to_owned()),
        "requirements" => Ok("requirements/meetings".to_owned()),
        "ux-foundations" => Ok("ux-foundations/meetings".to_owned()),
        "roadmap" => Ok("roadmap/meetings".to_owned()),
        "architecture" => Ok("architecture/meetings".to_owned()),
        "tech-stack" => Ok("tech-stack/meetings".to_owned()),
        // Epic planning meetings live under the milestone
        "epic-planning" => Err("epic-planning meetings require milestone context".to_owned()),
        // Epic execution steps
        "analysis" | "ux-design" | "technical-design" | "planning" | "implementation"
        | "verification" | "review" | "release" | "retrospective" => {
            let dir = epic_dir.ok_or("epic step requires epic context")?;
            Ok(format!("{dir}/{step}/meetings"))
        }
        _ => Err(format!("no meetings directory for step: {step}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemStore;

    // -- create_meeting_folder --

    #[test]
    fn creates_notes_file() {
        let store = MemStore::new();
        create_meeting_folder(&store, "product-brief/meetings", 1, "vision-interview").unwrap();
        let content = store
            .read_file("product-brief/meetings/001-vision-interview/notes.md")
            .unwrap();
        assert!(content.contains("# vision-interview"));
        assert!(content.contains("## Discussion"));
    }

    #[test]
    fn creates_decisions_ndjson() {
        let store = MemStore::new();
        create_meeting_folder(&store, "product-brief/meetings", 1, "kickoff").unwrap();
        assert!(store.file_exists("product-brief/meetings/001-kickoff/decisions.ndjson"));
    }

    #[test]
    fn creates_action_items_ndjson() {
        let store = MemStore::new();
        create_meeting_folder(&store, "product-brief/meetings", 1, "kickoff").unwrap();
        assert!(
            store.file_exists("product-brief/meetings/001-kickoff/actions/action-items.ndjson")
        );
    }

    #[test]
    fn returns_folder_name() {
        let store = MemStore::new();
        let name = create_meeting_folder(&store, "product-brief/meetings", 3, "followup").unwrap();
        assert_eq!(name, "003-followup");
    }

    #[test]
    fn notes_contain_date() {
        let store = MemStore::new();
        create_meeting_folder(&store, "test/meetings", 1, "topic").unwrap();
        let content = store.read_file("test/meetings/001-topic/notes.md").unwrap();
        assert!(content.contains("**Date**:"));
        // Date should be a real date, not placeholder
        assert!(!content.contains("2026-01-01"));
    }

    // -- count_meetings --

    #[test]
    fn count_zero_when_empty() {
        let store = MemStore::new();
        assert_eq!(count_meetings(&store, "test/meetings"), 0);
    }

    #[test]
    fn count_after_creating_meetings() {
        let store = MemStore::new();
        create_meeting_folder(&store, "test/meetings", 1, "first").unwrap();
        assert_eq!(count_meetings(&store, "test/meetings"), 1);
        create_meeting_folder(&store, "test/meetings", 2, "second").unwrap();
        assert_eq!(count_meetings(&store, "test/meetings"), 2);
    }

    // -- meetings_dir_for_step --

    #[test]
    fn setup_steps_resolve() {
        assert_eq!(
            meetings_dir_for_step("product-brief", None).unwrap(),
            "product-brief/meetings"
        );
        assert_eq!(
            meetings_dir_for_step("architecture", None).unwrap(),
            "architecture/meetings"
        );
    }

    #[test]
    fn epic_steps_resolve_with_context() {
        assert_eq!(
            meetings_dir_for_step("analysis", Some("epics/E-001")).unwrap(),
            "epics/E-001/analysis/meetings"
        );
    }

    #[test]
    fn epic_steps_fail_without_context() {
        assert!(meetings_dir_for_step("analysis", None).is_err());
    }

    #[test]
    fn unknown_step_fails() {
        assert!(meetings_dir_for_step("bogus", None).is_err());
    }
}
