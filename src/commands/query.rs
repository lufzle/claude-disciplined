use serde::Serialize;

use super::{CmdResult, extract_field, extract_list_field, map_store_err, workflow::status};
use crate::{action_item, decision, ndjson, resolve, store::Store};

// ---------------------------------------------------------------------------
// Shared summary types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ArtifactSummary {
    pub id: String,
    pub status: String,
    pub path: String,
}

// ---------------------------------------------------------------------------
// Listing queries
// ---------------------------------------------------------------------------

/// List all requirements.
pub fn query_requirements(store: &impl Store) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;
    Ok(scan_artifacts(store, "requirements", "REQ-"))
}

/// List all NFRs, optionally filtered by status.
pub fn query_nfrs(
    store: &impl Store,
    status_filter: Option<&str>,
) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;
    let all = scan_artifacts(store, "requirements", "NFR-");
    Ok(filter_by_status(all, status_filter))
}

/// List all milestones, optionally filtered by a requirement they satisfy.
pub fn query_milestones(
    store: &impl Store,
    for_req: Option<&str>,
) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;
    let all = scan_artifacts(store, "milestones", "milestone.md");
    Ok(filter_by_link(store, all, "satisfies", for_req))
}

/// List all epics, optionally filtered by milestone or requirement.
pub fn query_epics(
    store: &impl Store,
    for_filter: Option<&str>,
) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;
    let all = scan_artifacts(store, "milestones", "epic.md");

    if let Some(filter_id) = for_filter {
        if filter_id.starts_with("M-") {
            // Filter by milestone: epic path must contain the milestone dir
            let m_dir = store
                .find_dir(&format!("milestones/{filter_id}-*"))
                .ok_or_else(|| {
                    super::CmdError::Store(format!("milestone not found: {filter_id}"))
                })?;
            return Ok(all
                .into_iter()
                .filter(|a| a.path.starts_with(&m_dir))
                .collect());
        }
        // Filter by requirement via satisfies link
        return Ok(filter_by_link(store, all, "satisfies", Some(filter_id)));
    }

    Ok(all)
}

/// List all stories, optionally filtered by epic or requirement.
pub fn query_stories(
    store: &impl Store,
    for_filter: Option<&str>,
) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;
    let all = scan_artifacts(store, "milestones", "story.md");

    if let Some(filter_id) = for_filter {
        if filter_id.starts_with("E-") {
            let e_dir = find_epic_dir(store, filter_id)?;
            return Ok(all
                .into_iter()
                .filter(|a| a.path.starts_with(&e_dir))
                .collect());
        }
        return Ok(filter_by_link(store, all, "satisfies", Some(filter_id)));
    }

    Ok(all)
}

/// List all tasks, optionally filtered by story.
pub fn query_tasks(store: &impl Store, for_story: Option<&str>) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;

    if let Some(story_id) = for_story {
        let e_dir = resolve::current_epic_dir(store).map_err(super::CmdError::Store)?;
        let s_dir = store
            .find_dir(&format!("{e_dir}/stories/{story_id}-*"))
            .ok_or_else(|| super::CmdError::Store(format!("story not found: {story_id}")))?;
        return Ok(scan_artifacts(store, &format!("{s_dir}/tasks"), "T-"));
    }

    Ok(scan_artifacts(store, "milestones", "T-"))
}

/// List all flows, optionally filtered by story.
pub fn query_flows(store: &impl Store, for_story: Option<&str>) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;

    if let Some(story_id) = for_story {
        let e_dir = resolve::current_epic_dir(store).map_err(super::CmdError::Store)?;
        let s_dir = store
            .find_dir(&format!("{e_dir}/stories/{story_id}-*"))
            .ok_or_else(|| super::CmdError::Store(format!("story not found: {story_id}")))?;
        return Ok(scan_artifacts(store, &format!("{s_dir}/flows"), "F-"));
    }

    Ok(scan_artifacts(store, "milestones", "F-"))
}

/// List all decisions across all meetings, optionally filtered by status.
pub fn query_decisions(
    store: &impl Store,
    status_filter: Option<&str>,
) -> CmdResult<Vec<DecisionSummary>> {
    status(store)?;

    let paths = store.find_all_files_deep(".", "decisions.ndjson");
    let mut result = Vec::new();

    for path in paths {
        let decisions = ndjson::load(store, &path, decision::from_ndjson_line).unwrap_or_default();
        for d in decisions {
            let status_str = format!("{:?}", d.status).to_lowercase();
            if let Some(filter) = status_filter {
                if !status_str.contains(filter) {
                    continue;
                }
            }
            result.push(DecisionSummary {
                id: d.id,
                summary: d.summary,
                status: status_str,
                path: path.clone(),
            });
        }
    }

    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct DecisionSummary {
    pub id: String,
    pub summary: String,
    pub status: String,
    pub path: String,
}

/// List all action items across all meetings, with optional filters.
pub fn query_actions(
    store: &impl Store,
    type_filter: Option<&str>,
    status_filter: Option<&str>,
    assignee_filter: Option<&str>,
) -> CmdResult<Vec<ActionSummary>> {
    status(store)?;

    let paths = store.find_all_files_deep(".", "action-items.ndjson");
    let mut result = Vec::new();

    for path in &paths {
        let actions = ndjson::load(store, path, action_item::from_ndjson_line).unwrap_or_default();
        for a in actions {
            let type_str = serde_json::to_value(&a.action_type)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let status_str = serde_json::to_value(a.status)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();

            if let Some(tf) = type_filter {
                if type_str != tf {
                    continue;
                }
            }
            if let Some(sf) = status_filter {
                if status_str != sf {
                    continue;
                }
            }
            if let Some(af) = assignee_filter {
                if a.assignee != af {
                    continue;
                }
            }

            result.push(ActionSummary {
                id: a.id,
                description: a.description,
                action_type: type_str,
                status: status_str,
                assignee: a.assignee,
                path: path.clone(),
            });
        }
    }

    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct ActionSummary {
    pub id: String,
    pub description: String,
    pub action_type: String,
    pub status: String,
    pub assignee: String,
    pub path: String,
}

// ---------------------------------------------------------------------------
// Traceability queries
// ---------------------------------------------------------------------------

/// Trace an artifact up through the full chain to goals/outcomes.
#[derive(Debug, Serialize)]
pub struct TraceResult {
    pub id: String,
    pub chain: Vec<TraceLink>,
}

#[derive(Debug, Serialize)]
pub struct TraceLink {
    pub level: String,
    pub id: String,
    pub path: String,
}

pub fn query_trace(store: &impl Store, id: &str) -> CmdResult<TraceResult> {
    status(store)?;

    let mut chain = Vec::new();

    // Resolve the artifact path
    let path = resolve::artifact_path(store, id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(map_store_err)?;

    chain.push(TraceLink {
        level: id_level(id),
        id: id.to_owned(),
        path: path.clone(),
    });

    // Walk up the hierarchy based on filesystem structure
    // task -> story -> epic -> milestone
    if id.starts_with("T-") || id.starts_with("F-") {
        // Parent story: go up from tasks/T-NNN.md or flows/F-NNN.md to story.md
        if let Some(story_dir) = parent_dir(&path, 2) {
            if let Ok(story_content) = store.read_file(&format!("{story_dir}/story.md")) {
                if let Some(story_id) = extract_field(&story_content, "id") {
                    chain.push(TraceLink {
                        level: "story".to_owned(),
                        id: story_id,
                        path: format!("{story_dir}/story.md"),
                    });
                    // Story -> requirements via satisfies
                    add_satisfies_links(store, &story_content, &mut chain);
                }
            }
        }
        // Parent epic
        if let Some(epic_dir) = parent_dir(&path, 4) {
            add_epic_and_up(store, epic_dir, &mut chain);
        }
    } else if id.starts_with("S-") {
        // Story: path is .../stories/S-NNN-slug/story.md
        add_satisfies_links(store, &content, &mut chain);
        if let Some(epic_dir) = parent_dir(&path, 3) {
            add_epic_and_up(store, epic_dir, &mut chain);
        }
    } else if id.starts_with("E-") {
        // Epic: path is .../epics/E-NNN-slug/epic.md
        add_satisfies_links(store, &content, &mut chain);
        if let Some(m_dir) = parent_dir(&path, 3) {
            add_milestone_and_up(store, m_dir, &mut chain);
        }
    } else if id.starts_with("M-") {
        add_satisfies_links(store, &content, &mut chain);
        add_requirement_goals(store, &content, &mut chain);
    } else if id.starts_with("REQ-") || id.starts_with("NFR-") {
        add_contributes_links(store, &content, &mut chain);
    }

    Ok(TraceResult {
        id: id.to_owned(),
        chain,
    })
}

/// Find all downstream work impacted by a decision.
pub fn query_impact(store: &impl Store, decision_id: &str) -> CmdResult<Vec<ArtifactSummary>> {
    status(store)?;

    // Find all tasks/flows that reference this decision in informed_by
    let all_tasks = store.find_all_files_deep("milestones", "T-");
    let mut result = Vec::new();

    for path in all_tasks {
        if let Ok(content) = store.read_file(&path) {
            let informed = extract_list_field(&content, "informed_by");
            if informed.iter().any(|link| link.contains(decision_id)) {
                let id = extract_field(&content, "id").unwrap_or_default();
                let status = extract_field(&content, "status").unwrap_or_default();
                result.push(ArtifactSummary { id, status, path });
            }
        }
    }

    Ok(result)
}

/// Find decisions informing a task (via `informed_by`).
pub fn query_rationale(store: &impl Store, id: &str) -> CmdResult<Vec<DecisionSummary>> {
    status(store)?;

    let path = resolve::artifact_path(store, id).map_err(super::CmdError::Store)?;
    let content = store.read_file(&path).map_err(map_store_err)?;
    let informed = extract_list_field(&content, "informed_by");

    let mut result = Vec::new();
    for link in &informed {
        // Links are like: path/to/decisions.ndjson#D-001
        if let Some((file_path, d_id)) = link.rsplit_once('#') {
            if let Ok(decisions) = ndjson::load(store, file_path, decision::from_ndjson_line) {
                if let Some(d) = decisions.iter().find(|d| d.id == d_id) {
                    result.push(DecisionSummary {
                        id: d.id.clone(),
                        summary: d.summary.clone(),
                        status: format!("{:?}", d.status).to_lowercase(),
                        path: file_path.to_owned(),
                    });
                }
            }
        }
    }

    Ok(result)
}

/// Find orphaned artifacts and uncovered requirements/goals.
#[derive(Debug, Serialize)]
pub struct CoverageResult {
    pub uncovered_requirements: Vec<String>,
    pub orphaned_milestones: Vec<String>,
    pub orphaned_epics: Vec<String>,
    pub orphaned_stories: Vec<String>,
}

pub fn query_coverage(store: &impl Store) -> CmdResult<CoverageResult> {
    status(store)?;

    // Gather all requirement IDs
    let req_paths = store.find_all_files_deep("requirements", "REQ-");
    let mut req_ids: Vec<String> = Vec::new();
    for path in &req_paths {
        if let Ok(content) = store.read_file(path) {
            if let Some(id) = extract_field(&content, "id") {
                req_ids.push(id);
            }
        }
    }

    // Check which requirements are satisfied by at least one milestone
    let milestone_paths = store.find_all_files_deep("milestones", "milestone.md");
    let mut covered_reqs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut orphaned_milestones = Vec::new();

    for path in &milestone_paths {
        if let Ok(content) = store.read_file(path) {
            let satisfies = extract_list_field(&content, "satisfies");
            if satisfies.is_empty() {
                if let Some(id) = extract_field(&content, "id") {
                    orphaned_milestones.push(id);
                }
            }
            for link in &satisfies {
                // Extract REQ-NNN from path like "requirements/REQ-001-slug.md"
                if let Some(req_id) = extract_id_from_path(link) {
                    covered_reqs.insert(req_id);
                }
            }
        }
    }

    let uncovered_requirements: Vec<String> = req_ids
        .into_iter()
        .filter(|id| !covered_reqs.contains(id))
        .collect();

    // Check epics without satisfies links
    let epic_paths = store.find_all_files_deep("milestones", "epic.md");
    let mut orphaned_epics = Vec::new();
    for path in &epic_paths {
        if let Ok(content) = store.read_file(path) {
            let satisfies = extract_list_field(&content, "satisfies");
            if satisfies.is_empty() {
                if let Some(id) = extract_field(&content, "id") {
                    orphaned_epics.push(id);
                }
            }
        }
    }

    // Check stories without satisfies links
    let story_paths = store.find_all_files_deep("milestones", "story.md");
    let mut orphaned_stories = Vec::new();
    for path in &story_paths {
        if let Ok(content) = store.read_file(path) {
            let satisfies = extract_list_field(&content, "satisfies");
            if satisfies.is_empty() {
                if let Some(id) = extract_field(&content, "id") {
                    orphaned_stories.push(id);
                }
            }
        }
    }

    Ok(CoverageResult {
        uncovered_requirements,
        orphaned_milestones,
        orphaned_epics,
        orphaned_stories,
    })
}

/// Find all work tracing to a goal fragment.
#[derive(Debug, Serialize)]
pub struct GoalTraceEntry {
    pub artifact_type: String,
    pub id: String,
    pub status: String,
    pub path: String,
}

pub fn query_goal(store: &impl Store, fragment: &str) -> CmdResult<Vec<GoalTraceEntry>> {
    status(store)?;

    let goal_target = format!("product-brief/product-brief.md#{fragment}");
    let mut result = Vec::new();

    // Find requirements that contribute to this goal
    let req_paths = store.find_all_files_deep("requirements", "REQ-");
    let nfr_paths = store.find_all_files_deep("requirements", "NFR-");

    let mut contributing_req_ids = Vec::new();

    for path in req_paths.iter().chain(nfr_paths.iter()) {
        if let Ok(content) = store.read_file(path) {
            let contributes = extract_list_field(&content, "contributes");
            if contributes.iter().any(|c| c == &goal_target) {
                let id = extract_field(&content, "id").unwrap_or_default();
                let status = extract_field(&content, "status").unwrap_or_default();
                let artifact_type = if id.starts_with("REQ") {
                    "requirement"
                } else {
                    "nfr"
                };
                contributing_req_ids.push(id.clone());
                result.push(GoalTraceEntry {
                    artifact_type: artifact_type.to_owned(),
                    id,
                    status,
                    path: path.clone(),
                });
            }
        }
    }

    // Find milestones/epics/stories that satisfy those requirements
    if !contributing_req_ids.is_empty() {
        let milestone_paths = store.find_all_files_deep("milestones", "milestone.md");
        let epic_paths = store.find_all_files_deep("milestones", "epic.md");
        let story_paths = store.find_all_files_deep("milestones", "story.md");

        for (paths, artifact_type) in [
            (&milestone_paths, "milestone"),
            (&epic_paths, "epic"),
            (&story_paths, "story"),
        ] {
            for path in paths {
                if let Ok(content) = store.read_file(path) {
                    let satisfies = extract_list_field(&content, "satisfies");
                    let links_to_contributing = satisfies.iter().any(|link| {
                        contributing_req_ids
                            .iter()
                            .any(|req_id| link.contains(req_id.as_str()))
                    });
                    if links_to_contributing {
                        let id = extract_field(&content, "id").unwrap_or_default();
                        let status = extract_field(&content, "status").unwrap_or_default();
                        result.push(GoalTraceEntry {
                            artifact_type: artifact_type.to_owned(),
                            id,
                            status,
                            path: path.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scan_artifacts(store: &impl Store, root: &str, prefix: &str) -> Vec<ArtifactSummary> {
    let paths = store.find_all_files_deep(root, prefix);
    let mut result = Vec::new();
    for path in paths {
        if let Ok(content) = store.read_file(&path) {
            let id = extract_field(&content, "id").unwrap_or_default();
            let artifact_status = extract_field(&content, "status").unwrap_or_default();
            result.push(ArtifactSummary {
                id,
                status: artifact_status,
                path,
            });
        }
    }
    result
}

fn filter_by_status(items: Vec<ArtifactSummary>, filter: Option<&str>) -> Vec<ArtifactSummary> {
    match filter {
        Some(s) => items.into_iter().filter(|a| a.status == s).collect(),
        None => items,
    }
}

fn filter_by_link(
    store: &impl Store,
    items: Vec<ArtifactSummary>,
    field: &str,
    filter_id: Option<&str>,
) -> Vec<ArtifactSummary> {
    let Some(target_id) = filter_id else {
        return items;
    };
    items
        .into_iter()
        .filter(|a| {
            if let Ok(content) = store.read_file(&a.path) {
                let links = extract_list_field(&content, field);
                links.iter().any(|link| link.contains(target_id))
            } else {
                false
            }
        })
        .collect()
}

fn find_epic_dir(store: &impl Store, epic_id: &str) -> CmdResult<String> {
    let m_dir = resolve::current_milestone_dir(store).map_err(super::CmdError::Store)?;
    store
        .find_dir(&format!("{m_dir}/epics/{epic_id}-*"))
        .ok_or_else(|| super::CmdError::Store(format!("epic not found: {epic_id}")))
}

/// Go N directories up from a file path.
fn parent_dir(path: &str, levels: usize) -> Option<&str> {
    let mut end = path.len();
    for _ in 0..levels {
        end = path[..end].rfind('/')?;
    }
    Some(&path[..end])
}

fn id_level(id: &str) -> String {
    match id.split('-').next() {
        Some("T") => "task",
        Some("F") => "flow",
        Some("S") => "story",
        Some("E") => "epic",
        Some("M") => "milestone",
        Some("REQ") => "requirement",
        Some("NFR") => "nfr",
        _ => "unknown",
    }
    .to_owned()
}

fn add_satisfies_links(store: &impl Store, content: &str, chain: &mut Vec<TraceLink>) {
    let satisfies = extract_list_field(content, "satisfies");
    for link in &satisfies {
        if let Some(req_id) = extract_id_from_path(link) {
            if let Ok(req_content) = store.read_file(link) {
                chain.push(TraceLink {
                    level: "requirement".to_owned(),
                    id: req_id,
                    path: link.clone(),
                });
                add_contributes_links(store, &req_content, chain);
            }
        }
    }
}

fn add_contributes_links(_store: &impl Store, content: &str, chain: &mut Vec<TraceLink>) {
    let contributes = extract_list_field(content, "contributes");
    for link in &contributes {
        chain.push(TraceLink {
            level: "goal".to_owned(),
            id: link.clone(),
            path: link.clone(),
        });
    }
}

fn add_epic_and_up(store: &impl Store, epic_dir: &str, chain: &mut Vec<TraceLink>) {
    let epic_path = format!("{epic_dir}/epic.md");
    if let Ok(content) = store.read_file(&epic_path) {
        if let Some(epic_id) = extract_field(&content, "id") {
            chain.push(TraceLink {
                level: "epic".to_owned(),
                id: epic_id,
                path: epic_path,
            });
            add_satisfies_links(store, &content, chain);
        }
    }
    // Epic dir -> epics -> milestone dir
    if let Some(m_dir) = parent_dir(epic_dir, 2) {
        add_milestone_and_up(store, m_dir, chain);
    }
}

fn add_milestone_and_up(store: &impl Store, m_dir: &str, chain: &mut Vec<TraceLink>) {
    let m_path = format!("{m_dir}/milestone.md");
    if let Ok(content) = store.read_file(&m_path) {
        if let Some(m_id) = extract_field(&content, "id") {
            chain.push(TraceLink {
                level: "milestone".to_owned(),
                id: m_id,
                path: m_path,
            });
            add_satisfies_links(store, &content, chain);
            add_requirement_goals(store, &content, chain);
        }
    }
}

fn add_requirement_goals(store: &impl Store, content: &str, chain: &mut Vec<TraceLink>) {
    let satisfies = extract_list_field(content, "satisfies");
    for link in &satisfies {
        if let Ok(req_content) = store.read_file(link) {
            add_contributes_links(store, &req_content, chain);
        }
    }
}

/// Extract an ID like "REQ-001" from a path like
/// "requirements/REQ-001-auth-login.md".
fn extract_id_from_path(path: &str) -> Option<String> {
    let filename = path.rsplit('/').next()?;
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    // ID is everything up to the second dash-separated segment (e.g., REQ-001)
    let parts: Vec<&str> = stem.splitn(3, '-').collect();
    if parts.len() >= 2 {
        Some(format!("{}-{}", parts[0], parts[1]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{
            create_epic, create_flow, create_milestone, create_requirement, create_story,
            create_task, init, propose_nfr, replace_front_matter_field,
        },
        state::{Phase, State},
        store::MemStore,
    };

    fn store_with_artifacts() -> MemStore {
        let store = MemStore::new();
        init(&store).unwrap();
        create_milestone(&store, "mvp").unwrap();
        create_requirement(&store, "auth-login").unwrap();
        create_requirement(&store, "billing").unwrap();
        propose_nfr(&store, "perf-latency").unwrap();
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "ui",
        )
        .unwrap();
        create_flow(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "happy",
            "happy-path",
        )
        .unwrap();

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

    // -- Listing queries --

    #[test]
    fn query_requirements_returns_all() {
        let store = store_with_artifacts();
        let reqs = query_requirements(&store).unwrap();
        assert_eq!(reqs.len(), 2);
    }

    #[test]
    fn query_nfrs_returns_all() {
        let store = store_with_artifacts();
        let nfrs = query_nfrs(&store, None).unwrap();
        assert_eq!(nfrs.len(), 1);
    }

    #[test]
    fn query_nfrs_filters_by_status() {
        let store = store_with_artifacts();
        let draft = query_nfrs(&store, Some("draft")).unwrap();
        assert_eq!(draft.len(), 1);
        let active = query_nfrs(&store, Some("active")).unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn query_milestones_returns_all() {
        let store = store_with_artifacts();
        let milestones = query_milestones(&store, None).unwrap();
        assert_eq!(milestones.len(), 1);
        assert_eq!(milestones[0].id, "M-001");
    }

    #[test]
    fn query_epics_returns_all() {
        let store = store_with_artifacts();
        let epics = query_epics(&store, None).unwrap();
        assert_eq!(epics.len(), 1);
        assert_eq!(epics[0].id, "E-001");
    }

    #[test]
    fn query_stories_returns_all() {
        let store = store_with_artifacts();
        let stories = query_stories(&store, None).unwrap();
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].id, "S-001");
    }

    #[test]
    fn query_tasks_returns_all() {
        let store = store_with_artifacts();
        let tasks = query_tasks(&store, None).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "T-001");
    }

    #[test]
    fn query_flows_returns_all() {
        let store = store_with_artifacts();
        let flows = query_flows(&store, None).unwrap();
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].id, "F-001");
    }

    #[test]
    fn query_tasks_filtered_by_story() {
        let store = store_with_artifacts();
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "signup").unwrap();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-002-signup",
            "form",
        )
        .unwrap();

        let s1_tasks = query_tasks(&store, Some("S-001")).unwrap();
        assert_eq!(s1_tasks.len(), 1);
        assert_eq!(s1_tasks[0].id, "T-001");

        let s2_tasks = query_tasks(&store, Some("S-002")).unwrap();
        assert_eq!(s2_tasks.len(), 1);
        assert_eq!(s2_tasks[0].id, "T-002");
    }

    // -- Traceability --

    #[test]
    fn query_trace_task_includes_chain() {
        let store = store_with_artifacts();
        let trace = query_trace(&store, "T-001").unwrap();
        assert_eq!(trace.id, "T-001");
        // Chain should include: task, story, epic, milestone at minimum
        let levels: Vec<&str> = trace.chain.iter().map(|l| l.level.as_str()).collect();
        assert!(levels.contains(&"task"), "missing task level: {levels:?}");
        assert!(levels.contains(&"story"), "missing story level: {levels:?}");
        assert!(levels.contains(&"epic"), "missing epic level: {levels:?}");
        assert!(
            levels.contains(&"milestone"),
            "missing milestone level: {levels:?}"
        );
    }

    #[test]
    fn query_trace_requirement_includes_self() {
        let store = store_with_artifacts();
        let trace = query_trace(&store, "REQ-001").unwrap();
        assert_eq!(trace.chain[0].id, "REQ-001");
        assert_eq!(trace.chain[0].level, "requirement");
    }

    // -- Coverage --

    #[test]
    fn query_coverage_finds_uncovered_requirements() {
        let store = store_with_artifacts();
        let cov = query_coverage(&store).unwrap();
        // Milestones have empty satisfies initially, so all reqs are uncovered
        assert_eq!(cov.uncovered_requirements.len(), 2);
    }

    #[test]
    fn query_coverage_covered_requirement_not_reported() {
        let store = store_with_artifacts();
        // Add satisfies link to milestone
        let content = store
            .read_file("milestones/M-001-mvp/milestone.md")
            .unwrap();
        let updated = replace_front_matter_field(
            &content,
            "satisfies",
            "\n  - requirements/REQ-001-auth-login.md",
        );
        store
            .write_file("milestones/M-001-mvp/milestone.md", &updated)
            .unwrap();

        let cov = query_coverage(&store).unwrap();
        assert_eq!(cov.uncovered_requirements.len(), 1);
        assert_eq!(cov.uncovered_requirements[0], "REQ-002");
    }

    // -- Impact --

    #[test]
    fn query_impact_finds_tasks_referencing_decision() {
        let store = store_with_artifacts();
        // Add informed_by link to task
        let task_path =
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/tasks/T-001-ui.md";
        let content = store.read_file(task_path).unwrap();
        let updated = replace_front_matter_field(
            &content,
            "informed_by",
            "\n  - some/path/decisions.ndjson#D-001",
        );
        store.write_file(task_path, &updated).unwrap();

        let impact = query_impact(&store, "D-001").unwrap();
        assert_eq!(impact.len(), 1);
        assert_eq!(impact[0].id, "T-001");
    }

    #[test]
    fn query_impact_empty_when_no_references() {
        let store = store_with_artifacts();
        let impact = query_impact(&store, "D-999").unwrap();
        assert!(impact.is_empty());
    }

    // -- helpers --

    #[test]
    fn extract_id_from_path_works() {
        assert_eq!(
            extract_id_from_path("requirements/REQ-001-auth-login.md"),
            Some("REQ-001".to_owned())
        );
        assert_eq!(
            extract_id_from_path("requirements/NFR-002-perf.md"),
            Some("NFR-002".to_owned())
        );
    }

    #[test]
    fn parent_dir_levels() {
        assert_eq!(parent_dir("a/b/c/d.md", 1), Some("a/b/c"));
        assert_eq!(parent_dir("a/b/c/d.md", 2), Some("a/b"));
        assert_eq!(parent_dir("a/b/c/d.md", 3), Some("a"));
        assert_eq!(parent_dir("a/b/c/d.md", 4), None);
    }

    // -- requires initialization --

    #[test]
    fn queries_require_initialization() {
        let store = MemStore::new();
        assert!(query_requirements(&store).is_err());
        assert!(query_nfrs(&store, None).is_err());
        assert!(query_milestones(&store, None).is_err());
        assert!(query_trace(&store, "T-001").is_err());
        assert!(query_coverage(&store).is_err());
    }
}
