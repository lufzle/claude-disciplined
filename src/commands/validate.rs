use serde::Serialize;

use super::{CmdResult, extract_field, extract_list_field, workflow::status};
use crate::{action_item, decision, ndjson, store::Store};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub rule: String,
    pub message: String,
}

/// Full lint of the project.
pub fn validate(store: &impl Store) -> CmdResult<Vec<Finding>> {
    status(store)?;
    let mut findings = Vec::new();

    check_link_rules(store, &mut findings);
    check_coverage_rules(store, &mut findings);
    check_structure_rules(store, &mut findings);
    check_coherence_rules(store, &mut findings);

    Ok(findings)
}

/// Validate a specific milestone.
pub fn validate_milestone(store: &impl Store, milestone_id: &str) -> CmdResult<Vec<Finding>> {
    status(store)?;
    let mut findings = Vec::new();

    let m_dir = store
        .find_dir(&format!("milestones/{milestone_id}-*"))
        .ok_or_else(|| super::CmdError::Store(format!("milestone not found: {milestone_id}")))?;

    // Check milestone has satisfies links
    let m_path = format!("{m_dir}/milestone.md");
    if let Ok(content) = store.read_file(&m_path) {
        let satisfies = extract_list_field(&content, "satisfies");
        if satisfies.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                rule: "link".to_owned(),
                message: format!("{milestone_id} has no `satisfies` links"),
            });
        }
        check_broken_links(store, &satisfies, milestone_id, &mut findings);
    }

    // Check all epics under this milestone
    let epic_paths = store.find_all_files_deep(&format!("{m_dir}/epics"), "epic.md");
    for path in &epic_paths {
        check_artifact_links(store, path, "satisfies", &mut findings);
    }

    // Check all stories
    let story_paths = store.find_all_files_deep(&m_dir, "story.md");
    for path in &story_paths {
        check_artifact_links(store, path, "satisfies", &mut findings);
        check_story_rules(store, path, &mut findings);
    }

    Ok(findings)
}

/// Validate a specific epic.
pub fn validate_epic(store: &impl Store, epic_id: &str) -> CmdResult<Vec<Finding>> {
    status(store)?;
    let mut findings = Vec::new();

    let path = crate::resolve::artifact_path(store, epic_id).map_err(super::CmdError::Store)?;
    let e_dir = path.strip_suffix("/epic.md").unwrap_or(&path);

    check_artifact_links(store, &path, "satisfies", &mut findings);

    // Check stories under this epic
    let story_paths = store.find_all_files_deep(e_dir, "story.md");
    for spath in &story_paths {
        check_artifact_links(store, spath, "satisfies", &mut findings);
        check_story_rules(store, spath, &mut findings);
    }

    Ok(findings)
}

/// Validate all links (no broken references).
pub fn validate_links(store: &impl Store) -> CmdResult<Vec<Finding>> {
    status(store)?;
    let mut findings = Vec::new();

    // Check all artifacts with satisfies or contributes fields
    for prefix in ["milestone.md", "epic.md", "story.md"] {
        let paths = store.find_all_files_deep("milestones", prefix);
        for path in &paths {
            if let Ok(content) = store.read_file(path) {
                let id = extract_field(&content, "id").unwrap_or_default();
                let satisfies = extract_list_field(&content, "satisfies");
                check_broken_links(store, &satisfies, &id, &mut findings);
            }
        }
    }

    for prefix in ["REQ-", "NFR-"] {
        let paths = store.find_all_files_deep("requirements", prefix);
        for path in &paths {
            if let Ok(content) = store.read_file(path) {
                let id = extract_field(&content, "id").unwrap_or_default();
                let contributes = extract_list_field(&content, "contributes");
                // contributes links are to product-brief fragments — just check the file exists
                for link in &contributes {
                    let file_part = link.split('#').next().unwrap_or(link);
                    if !store.file_exists(file_part) {
                        findings.push(Finding {
                            severity: Severity::Error,
                            rule: "link-integrity".to_owned(),
                            message: format!("{id} links to non-existent file: {file_part}"),
                        });
                    }
                }
            }
        }
    }

    Ok(findings)
}

/// Validate coverage (same as query coverage but as findings).
pub fn validate_coverage(store: &impl Store) -> CmdResult<Vec<Finding>> {
    status(store)?;
    let mut findings = Vec::new();
    check_coverage_rules(store, &mut findings);
    Ok(findings)
}

// ---------------------------------------------------------------------------
// Rule checks
// ---------------------------------------------------------------------------

fn check_link_rules(store: &impl Store, findings: &mut Vec<Finding>) {
    // Every milestone must have satisfies links
    let milestone_paths = store.find_all_files_deep("milestones", "milestone.md");
    for path in &milestone_paths {
        check_artifact_links(store, path, "satisfies", findings);
    }

    // Every epic must have satisfies links
    let epic_paths = store.find_all_files_deep("milestones", "epic.md");
    for path in &epic_paths {
        check_artifact_links(store, path, "satisfies", findings);
    }

    // Every story must have satisfies links
    let story_paths = store.find_all_files_deep("milestones", "story.md");
    for path in &story_paths {
        check_artifact_links(store, path, "satisfies", findings);
    }

    // Every requirement must have contributes links
    let req_paths = store.find_all_files_deep("requirements", "REQ-");
    for path in &req_paths {
        check_artifact_links(store, path, "contributes", findings);
    }
}

fn check_coverage_rules(store: &impl Store, findings: &mut Vec<Finding>) {
    // Collect all requirement IDs
    let req_paths = store.find_all_files_deep("requirements", "REQ-");
    let mut req_ids: Vec<String> = Vec::new();
    for path in &req_paths {
        if let Ok(content) = store.read_file(path) {
            if let Some(id) = extract_field(&content, "id") {
                req_ids.push(id);
            }
        }
    }

    // Check which are covered by milestones
    let milestone_paths = store.find_all_files_deep("milestones", "milestone.md");
    let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
    for path in &milestone_paths {
        if let Ok(content) = store.read_file(path) {
            let satisfies = extract_list_field(&content, "satisfies");
            for link in &satisfies {
                if let Some(id) = extract_id_from_path(link) {
                    covered.insert(id);
                }
            }
        }
    }

    for req_id in &req_ids {
        if !covered.contains(req_id) {
            findings.push(Finding {
                severity: Severity::Warning,
                rule: "coverage".to_owned(),
                message: format!("{req_id} is not satisfied by any milestone"),
            });
        }
    }
}

fn check_structure_rules(store: &impl Store, findings: &mut Vec<Finding>) {
    // Superseded artifacts must have superseded_by populated
    for prefix in ["milestone.md", "epic.md", "story.md"] {
        let paths = store.find_all_files_deep("milestones", prefix);
        for path in &paths {
            if let Ok(content) = store.read_file(path) {
                let id = extract_field(&content, "id").unwrap_or_default();
                let artifact_status = extract_field(&content, "status").unwrap_or_default();
                if artifact_status == "superseded" {
                    let superseded_by = extract_field(&content, "superseded_by");
                    if superseded_by.is_none() {
                        findings.push(Finding {
                            severity: Severity::Error,
                            rule: "structure".to_owned(),
                            message: format!(
                                "{id} is superseded but `superseded_by` is not populated"
                            ),
                        });
                    }
                }
            }
        }
    }

    // Stories should have at least one happy-path flow
    let story_paths = store.find_all_files_deep("milestones", "story.md");
    for path in &story_paths {
        check_story_rules(store, path, findings);
    }

    // Discarded action items must have a reason
    let action_paths = store.find_all_files_deep(".", "actions.ndjson");
    for path in &action_paths {
        if let Ok(actions) = ndjson::load(store, path, action_item::from_ndjson_line) {
            for action in &actions {
                if action.status == action_item::ActionStatus::Discarded
                    && action.discarded_reason.is_none()
                {
                    findings.push(Finding {
                        severity: Severity::Error,
                        rule: "structure".to_owned(),
                        message: format!("action {} is discarded without a reason", action.id),
                    });
                }
            }
        }
    }
}

fn check_coherence_rules(store: &impl Store, findings: &mut Vec<Finding>) {
    // Draft NFRs should not be referenced in story nfrs lists
    let nfr_paths = store.find_all_files_deep("requirements", "NFR-");
    let mut draft_nfr_ids: Vec<String> = Vec::new();
    for path in &nfr_paths {
        if let Ok(content) = store.read_file(path) {
            if extract_field(&content, "status").as_deref() == Some("draft") {
                if let Some(id) = extract_field(&content, "id") {
                    draft_nfr_ids.push(id);
                }
            }
        }
    }

    if !draft_nfr_ids.is_empty() {
        let story_paths = store.find_all_files_deep("milestones", "story.md");
        for path in &story_paths {
            if let Ok(content) = store.read_file(path) {
                let story_id = extract_field(&content, "id").unwrap_or_default();
                let nfrs = extract_list_field(&content, "nfrs");
                for nfr_link in &nfrs {
                    for draft_id in &draft_nfr_ids {
                        if nfr_link.contains(draft_id.as_str()) {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                rule: "coherence".to_owned(),
                                message: format!("{story_id} references draft NFR {draft_id}"),
                            });
                        }
                    }
                }
            }
        }
    }

    // Active work should not depend on obsolete/superseded artifacts
    for prefix in ["epic.md", "story.md"] {
        let paths = store.find_all_files_deep("milestones", prefix);
        for path in &paths {
            if let Ok(content) = store.read_file(path) {
                let id = extract_field(&content, "id").unwrap_or_default();
                let artifact_status = extract_field(&content, "status").unwrap_or_default();
                if artifact_status == "active" || artifact_status == "draft" {
                    let satisfies = extract_list_field(&content, "satisfies");
                    for link in &satisfies {
                        if let Ok(linked_content) = store.read_file(link) {
                            let linked_status =
                                extract_field(&linked_content, "status").unwrap_or_default();
                            if linked_status == "obsolete" || linked_status == "superseded" {
                                let linked_id =
                                    extract_field(&linked_content, "id").unwrap_or_default();
                                findings.push(Finding {
                                    severity: Severity::Error,
                                    rule: "coherence".to_owned(),
                                    message: format!(
                                        "{id} depends on {linked_status} artifact {linked_id}"
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

fn check_artifact_links(store: &impl Store, path: &str, field: &str, findings: &mut Vec<Finding>) {
    if let Ok(content) = store.read_file(path) {
        let id = extract_field(&content, "id").unwrap_or_default();
        let links = extract_list_field(&content, field);
        if links.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                rule: "link".to_owned(),
                message: format!("{id} has no `{field}` links"),
            });
        }
    }
}

fn check_broken_links(
    store: &impl Store,
    links: &[String],
    source_id: &str,
    findings: &mut Vec<Finding>,
) {
    for link in links {
        let file_part = link.split('#').next().unwrap_or(link);
        if !store.file_exists(file_part) {
            findings.push(Finding {
                severity: Severity::Error,
                rule: "link-integrity".to_owned(),
                message: format!("{source_id} links to non-existent file: {file_part}"),
            });
        }
    }
}

fn check_story_rules(store: &impl Store, story_path: &str, findings: &mut Vec<Finding>) {
    let Ok(content) = store.read_file(story_path) else {
        return;
    };
    let story_id = extract_field(&content, "id").unwrap_or_default();

    // Story dir is parent of story.md
    let story_dir = story_path.strip_suffix("/story.md").unwrap_or(story_path);

    // Check for at least one happy-path flow
    let flow_paths = store.find_all_files_deep(&format!("{story_dir}/flows"), "F-");
    let has_happy_path = flow_paths.iter().any(|fp| {
        store
            .read_file(fp)
            .ok()
            .and_then(|c| extract_field(&c, "type"))
            .is_some_and(|t| t == "happy-path")
    });

    if !has_happy_path && !flow_paths.is_empty() {
        // Only warn if there are flows but none are happy-path
        // If no flows at all, the story may not be at that stage yet
    } else if flow_paths.is_empty() {
        // No flows at all — warning since the story might not be at flow stage
        findings.push(Finding {
            severity: Severity::Warning,
            rule: "structure".to_owned(),
            message: format!("{story_id} has no user flows"),
        });
    }

    if !has_happy_path && !flow_paths.is_empty() {
        findings.push(Finding {
            severity: Severity::Error,
            rule: "structure".to_owned(),
            message: format!("{story_id} has flows but no happy-path flow"),
        });
    }

    // Check nfrs field exists
    let nfrs_field = extract_field(&content, "nfrs");
    let nfrs_list = extract_list_field(&content, "nfrs");
    if nfrs_field.is_none() && nfrs_list.is_empty() {
        // Check if the raw field line exists at all
        let has_nfrs_field = content.lines().any(|l| l.starts_with("nfrs:"));
        if !has_nfrs_field {
            findings.push(Finding {
                severity: Severity::Warning,
                rule: "structure".to_owned(),
                message: format!("{story_id} is missing `nfrs` field"),
            });
        }
    }
}

/// Extract an ID like "REQ-001" from a path like
/// "requirements/REQ-001-auth-login.md".
fn extract_id_from_path(path: &str) -> Option<String> {
    let filename = path.rsplit('/').next()?;
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    let parts: Vec<&str> = stem.splitn(3, '-').collect();
    if parts.len() >= 2 {
        Some(format!("{}-{}", parts[0], parts[1]))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Decision/action gate checks
// ---------------------------------------------------------------------------

/// Check meeting-related gate rules.
pub fn validate_meeting_gates(store: &impl Store) -> CmdResult<Vec<Finding>> {
    status(store)?;
    let mut findings = Vec::new();

    // All decisions in terminal state
    let decision_paths = store.find_all_files_deep(".", "decisions.ndjson");
    for path in &decision_paths {
        if let Ok(decisions) = ndjson::load(store, path, decision::from_ndjson_line) {
            for d in &decisions {
                if !d.status.is_terminal() {
                    findings.push(Finding {
                        severity: Severity::Error,
                        rule: "gate".to_owned(),
                        message: format!("decision {} is not in terminal state", d.id),
                    });
                }
            }
        }
    }

    // All immediate action items completed/discarded
    let action_paths = store.find_all_files_deep(".", "actions.ndjson");
    for path in &action_paths {
        if let Ok(actions) = ndjson::load(store, path, action_item::from_ndjson_line) {
            for a in &actions {
                if a.is_immediate() && !a.is_done() {
                    findings.push(Finding {
                        severity: Severity::Error,
                        rule: "gate".to_owned(),
                        message: format!("immediate action {} is not done", a.id),
                    });
                }
            }
        }
    }

    Ok(findings)
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
        create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();
        create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "ui",
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

    // -- Full validate --

    #[test]
    fn validate_finds_missing_links() {
        let store = store_with_artifacts();
        let findings = validate(&store).unwrap();
        // All artifacts start with empty satisfies/contributes
        let link_findings: Vec<_> = findings.iter().filter(|f| f.rule == "link").collect();
        assert!(
            !link_findings.is_empty(),
            "should find missing link warnings"
        );
    }

    #[test]
    fn validate_finds_uncovered_requirements() {
        let store = store_with_artifacts();
        let findings = validate(&store).unwrap();
        let coverage: Vec<_> = findings.iter().filter(|f| f.rule == "coverage").collect();
        assert!(!coverage.is_empty(), "should find uncovered requirements");
    }

    #[test]
    fn validate_no_link_warning_when_populated() {
        let store = store_with_artifacts();

        // Populate milestone satisfies
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

        let findings = validate(&store).unwrap();
        let m_link: Vec<_> = findings
            .iter()
            .filter(|f| f.rule == "link" && f.message.contains("M-001"))
            .collect();
        assert!(
            m_link.is_empty(),
            "populated milestone should not have link warning"
        );
    }

    // -- Milestone validate --

    #[test]
    fn validate_milestone_checks_epic_and_story_links() {
        let store = store_with_artifacts();
        let findings = validate_milestone(&store, "M-001").unwrap();
        // Should find warnings for milestone, epic, story without links
        assert!(!findings.is_empty());
    }

    // -- Epic validate --

    #[test]
    fn validate_epic_checks_stories() {
        let store = store_with_artifacts();
        let findings = validate_epic(&store, "E-001").unwrap();
        assert!(!findings.is_empty());
    }

    // -- Link validation --

    #[test]
    fn validate_links_detects_broken_refs() {
        let store = store_with_artifacts();

        // Add a broken satisfies link
        let content = store
            .read_file("milestones/M-001-mvp/milestone.md")
            .unwrap();
        let updated = replace_front_matter_field(
            &content,
            "satisfies",
            "\n  - requirements/REQ-999-nonexistent.md",
        );
        store
            .write_file("milestones/M-001-mvp/milestone.md", &updated)
            .unwrap();

        let findings = validate_links(&store).unwrap();
        let broken: Vec<_> = findings
            .iter()
            .filter(|f| f.rule == "link-integrity")
            .collect();
        assert!(!broken.is_empty(), "should detect broken link");
    }

    // -- Coverage validation --

    #[test]
    fn validate_coverage_finds_uncovered() {
        let store = store_with_artifacts();
        let findings = validate_coverage(&store).unwrap();
        assert!(findings.iter().any(|f| f.message.contains("REQ-001")));
    }

    // -- Structure rules --

    #[test]
    fn validate_detects_superseded_without_by() {
        let store = store_with_artifacts();

        // Set epic to superseded without superseded_by
        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/epic.md")
            .unwrap();
        let updated = replace_front_matter_field(&content, "status", "superseded");
        store
            .write_file("milestones/M-001-mvp/epics/E-001-auth/epic.md", &updated)
            .unwrap();

        let findings = validate(&store).unwrap();
        let structure: Vec<_> = findings
            .iter()
            .filter(|f| f.rule == "structure" && f.message.contains("superseded_by"))
            .collect();
        assert!(!structure.is_empty(), "should detect missing superseded_by");
    }

    // -- Story rules --

    #[test]
    fn validate_warns_story_without_flows() {
        let store = store_with_artifacts();
        let findings = validate(&store).unwrap();
        let no_flows: Vec<_> = findings
            .iter()
            .filter(|f| f.message.contains("no user flows"))
            .collect();
        assert!(!no_flows.is_empty());
    }

    #[test]
    fn validate_story_with_happy_path_flow_no_error() {
        let store = store_with_artifacts();
        create_flow(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "login-ok",
            "happy-path",
        )
        .unwrap();

        let findings = validate(&store).unwrap();
        let flow_errors: Vec<_> = findings
            .iter()
            .filter(|f| f.message.contains("happy-path") || f.message.contains("no user flows"))
            .collect();
        assert!(
            flow_errors.is_empty(),
            "story with happy-path flow should not have flow errors: {flow_errors:?}"
        );
    }

    // -- Coherence --

    #[test]
    fn validate_warns_draft_nfr_in_story() {
        let store = store_with_artifacts();
        propose_nfr(&store, "perf-latency").unwrap();

        // Add draft NFR reference to story
        let content = store
            .read_file("milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/story.md")
            .unwrap();
        let updated = replace_front_matter_field(
            &content,
            "nfrs",
            "\n  - requirements/NFR-001-perf-latency.md",
        );
        store
            .write_file(
                "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/story.md",
                &updated,
            )
            .unwrap();

        let findings = validate(&store).unwrap();
        let coherence: Vec<_> = findings
            .iter()
            .filter(|f| f.rule == "coherence" && f.message.contains("NFR-001"))
            .collect();
        assert!(
            !coherence.is_empty(),
            "should warn about draft NFR reference"
        );
    }

    // -- Requires initialization --

    #[test]
    fn validate_requires_initialization() {
        let store = MemStore::new();
        assert!(validate(&store).is_err());
    }

    // -- extract_id_from_path --

    #[test]
    fn extract_id_works() {
        assert_eq!(
            extract_id_from_path("requirements/REQ-001-auth.md"),
            Some("REQ-001".to_owned())
        );
    }
}
