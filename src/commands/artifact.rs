use super::{CmdError, CmdResult, map_store_err, workflow::status};
use crate::store::Store;

/// Read the content of an artifact by type and optional ID.
pub fn read_artifact(
    store: &impl Store,
    artifact_type: &str,
    id: Option<&str>,
) -> CmdResult<String> {
    status(store)?;
    let path = resolve_artifact_path(store, artifact_type, id)?;
    store.read_file(&path).map_err(map_store_err)
}

/// Write content to an artifact by type and optional ID.
/// Content below the front matter is replaced; front matter is preserved.
pub fn write_artifact(
    store: &impl Store,
    artifact_type: &str,
    id: Option<&str>,
    content: &str,
) -> CmdResult<()> {
    status(store)?;
    let path = resolve_artifact_path(store, artifact_type, id)?;

    let existing = store.read_file(&path).map_err(map_store_err)?;
    let new_content = if let Some(end) = find_front_matter_end(&existing) {
        format!("{}{content}", &existing[..end])
    } else {
        content.to_owned()
    };
    store.write_file(&path, &new_content).map_err(map_store_err)
}

/// Find the byte offset of the end of YAML front matter (after the closing
/// "---\n").
fn find_front_matter_end(content: &str) -> Option<usize> {
    if !content.starts_with("---\n") {
        return None;
    }
    let after_first = &content[4..];
    let close_pos = after_first.find("---\n")?;
    Some(4 + close_pos + 4)
}

fn resolve_artifact_path(
    store: &impl Store,
    artifact_type: &str,
    id: Option<&str>,
) -> CmdResult<String> {
    match artifact_type {
        "product-brief" | "architecture" | "ux-foundations" | "tech-stack" | "roadmap" => {
            Ok(crate::resolve::singleton_path(artifact_type))
        }
        "analysis" | "ux-design" | "technical-design" | "plan" | "review" | "release"
        | "retrospective" => {
            crate::resolve::epic_artifact_path(store, artifact_type).map_err(CmdError::Store)
        }
        "requirement" | "nfr" | "milestone" | "epic" | "story" | "task" | "flow" => {
            let id =
                id.ok_or_else(|| CmdError::Store(format!("{artifact_type} requires an ID")))?;
            crate::resolve::artifact_path(store, id).map_err(CmdError::Store)
        }
        _ => Err(CmdError::Store(format!(
            "unknown artifact type: {artifact_type}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{create_requirement, init},
        store::MemStore,
    };

    fn initialized_store() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        store
    }

    #[test]
    fn read_requirement_by_id() {
        let store = initialized_store();
        create_requirement(&store, "apps").unwrap();
        let content = read_artifact(&store, "requirement", Some("REQ-001")).unwrap();
        assert!(content.contains("id: REQ-001"));
    }

    #[test]
    fn read_singleton() {
        let store = initialized_store();
        store
            .write_file("product-brief/product-brief.md", "---\n---\nVision.")
            .unwrap();
        let content = read_artifact(&store, "product-brief", None).unwrap();
        assert!(content.contains("Vision."));
    }

    #[test]
    fn read_requires_id_for_typed_artifacts() {
        let store = initialized_store();
        assert!(read_artifact(&store, "requirement", None).is_err());
    }

    #[test]
    fn write_preserves_front_matter() {
        let store = initialized_store();
        create_requirement(&store, "apps").unwrap();
        write_artifact(&store, "requirement", Some("REQ-001"), "Body.\n").unwrap();
        let content = read_artifact(&store, "requirement", Some("REQ-001")).unwrap();
        assert!(content.contains("id: REQ-001"));
        assert!(content.contains("Body."));
    }

    #[test]
    fn write_replaces_body() {
        let store = initialized_store();
        create_requirement(&store, "apps").unwrap();
        write_artifact(&store, "requirement", Some("REQ-001"), "First.\n").unwrap();
        write_artifact(&store, "requirement", Some("REQ-001"), "Second.\n").unwrap();
        let content = read_artifact(&store, "requirement", Some("REQ-001")).unwrap();
        assert!(content.contains("Second."));
        assert!(!content.contains("First."));
    }

    #[test]
    fn front_matter_end_finds_closing() {
        assert_eq!(find_front_matter_end("---\nid: X\n---\nbody"), Some(14));
    }

    #[test]
    fn front_matter_end_none_without() {
        assert_eq!(find_front_matter_end("no front matter"), None);
    }
}
