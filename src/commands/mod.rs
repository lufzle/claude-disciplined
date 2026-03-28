pub(crate) mod action;
mod artifact;
mod create;
pub(crate) mod decision;
pub(crate) mod epic;
mod escalate;
pub(crate) mod meeting;
mod query;
pub(crate) mod task;
mod unresolved_cmd;
mod update;
pub mod validate;
mod verify;
mod workflow;

pub use action::{
    action_complete, action_discard, action_read, action_start, add_action, complete_action,
    discard_action, list_actions, parse_action_type, start_action,
};
pub use artifact::{read_artifact, write_artifact};
pub use create::{
    create_epic, create_flow, create_milestone, create_requirement, create_story, create_task,
    propose_nfr,
};
pub use decision::{
    address_disagreement, decision_status, drop_decision, list_decisions, position,
    propose_decision, record_decision, resolve_decision, supersede_decision,
};
pub use epic::{epic_branch_state, epic_complete, epic_list, epic_start};
pub use escalate::escalate;
pub use meeting::{meeting_contribute, meeting_end, meeting_list, meeting_read, meeting_start};
pub use query::{
    query_actions, query_coverage, query_decisions, query_epics, query_flows, query_goal,
    query_impact, query_milestones, query_nfrs, query_rationale, query_requirements, query_stories,
    query_tasks, query_trace,
};
pub use task::{task_block, task_complete, task_list, task_start, task_unblock};
pub use unresolved_cmd::{add_unresolved, history_unresolved, list_unresolved, review_unresolved};
pub use update::update;
pub use validate::{
    validate as validate_project, validate_coverage as validate_project_coverage,
    validate_epic as validate_project_epic, validate_links as validate_project_links,
    validate_milestone as validate_project_milestone,
};
pub use verify::{verify_epic, verify_report, verify_story};
pub use workflow::{advance, init, request_approval, status};

use crate::gates::Violation;

/// Result type for command operations.
pub type CmdResult<T> = Result<T, CmdError>;

#[derive(Debug)]
pub enum CmdError {
    AlreadyInitialized,
    NotInitialized,
    NoNextStep,
    GatesFailed(Vec<Violation>),
    Store(String),
}

impl std::fmt::Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => f.write_str("project is already initialized"),
            Self::NotInitialized => f.write_str("project is not initialized (run `workflow init`)"),
            Self::NoNextStep => f.write_str("no next step (end of current phase)"),
            Self::GatesFailed(violations) => {
                write!(f, "gate check failed ({} violations):", violations.len())?;
                for v in violations {
                    write!(f, "\n  - [{}] {}", v.rule, v.message)?;
                }
                Ok(())
            }
            Self::Store(msg) => write!(f, "store error: {msg}"),
        }
    }
}

impl std::error::Error for CmdError {}

pub(crate) fn map_store_err<E: std::error::Error>(e: E) -> CmdError {
    CmdError::Store(e.to_string())
}

/// Replace a field value in YAML front matter.
///
/// Handles simple `key: value` lines only (not multiline YAML).
pub fn replace_front_matter_field(content: &str, field: &str, new_value: &str) -> String {
    let prefix = format!("{field}: ");
    content
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                format!("{prefix}{new_value}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract a front matter field value from file content.
///
/// Returns `None` for missing fields or `null` values.
pub fn extract_field(content: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}: ");
    for line in content.lines() {
        if let Some(value) = line.strip_prefix(&prefix) {
            let value = value.trim();
            if value == "null" {
                return None;
            }
            return Some(value.to_owned());
        }
    }
    None
}

/// Extract a YAML list field from front matter.
///
/// Handles both inline `field: []` and multi-line:
/// ```yaml
/// field:
///   - value1
///   - value2
/// ```
pub fn extract_list_field(content: &str, field: &str) -> Vec<String> {
    let prefix = format!("{field}:");
    let mut lines = content.lines();
    let mut result = Vec::new();

    // Find the field line
    let field_line = loop {
        match lines.next() {
            Some(line) if line.starts_with(&prefix) => break line,
            Some(line) if line == "---" && !result.is_empty() => return result,
            None => return result,
            _ => {}
        }
    };

    // Check for inline value
    let after_colon = field_line[prefix.len()..].trim();
    if after_colon == "[]" || after_colon.is_empty() {
        // Empty or multi-line — collect continuation lines
    } else {
        // Inline non-empty (shouldn't happen in our YAML but handle gracefully)
        return vec![after_colon.to_owned()];
    }

    // Collect indented list items
    for line in lines {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("- ") {
            result.push(value.to_owned());
        } else {
            // No longer a list continuation
            break;
        }
    }

    result
}
