use serde::{Deserialize, Serialize};

use crate::{
    role::{self, Role},
    state::{Phase, State},
};

// ---------------------------------------------------------------------------
// I/O types (Claude Code hook contract)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct HookInput {
    pub tool_name: Option<String>,
    pub tool_input: Option<ToolInput>,
    pub cwd: Option<String>,
    pub agent_type: Option<String>,
}

#[derive(Deserialize)]
pub struct ToolInput {
    pub command: Option<String>,
    pub file_path: Option<String>,
    // For Edit tool
    pub old_string: Option<String>,
    pub new_string: Option<String>,
    // For Write tool
    pub content: Option<String>,
}

#[derive(Serialize)]
pub struct HookOutput {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: HookDecision,
    #[serde(rename = "systemMessage")]
    pub system_message: String,
}

#[derive(Serialize)]
pub struct HookDecision {
    #[serde(rename = "permissionDecision")]
    pub permission_decision: String,
}

impl HookOutput {
    pub fn deny(reason: String) -> Self {
        Self {
            hook_specific_output: HookDecision {
                permission_decision: "deny".to_owned(),
            },
            system_message: reason,
        }
    }
}

// ---------------------------------------------------------------------------
// Enforcement
// ---------------------------------------------------------------------------

/// Protected top-level paths that agents must not read/write directly.
const PROTECTED_PATHS: &[&str] = &[
    ".workflow/",
    "product-brief/",
    "requirements/",
    "ux-foundations/",
    "roadmap/",
    "architecture/",
    "tech-stack/",
    "milestones/",
];

/// Epic-level artifact path patterns (relative to epic dir).
const EPIC_ARTIFACT_PATTERNS: &[&str] = &[
    "analysis",
    "ux-design",
    "technical-design",
    "plan.md",
    "review",
    "release.md",
    "retrospective",
    "epic.md",
    "stories/",
];

/// State-mutating CLI subcommands that only drivers can invoke.
const DRIVER_ONLY_COMMANDS: &[&str] = &[
    "advance",
    "escalate",
    "meeting start",
    "meeting end",
    "epic start",
    "epic complete",
];

/// Evaluate hook enforcement rules. Returns `None` for allow, `Some(reason)`
/// for deny.
pub fn evaluate(input: &HookInput, state: &State) -> Option<String> {
    let tool_name = input.tool_name.as_deref()?;
    let tool_input = input.tool_input.as_ref();

    // Rule 1+2: File/Bash protection
    if let Some(reason) = check_file_protection(tool_name, tool_input) {
        return Some(reason);
    }

    // Rules 3-5 require an agent_type
    let agent_type_str = input.agent_type.as_deref()?;
    let agent_role = parse_role(agent_type_str)?;

    // Rule 3: Role enforcement
    if let Some(reason) = check_role(state, agent_role, agent_type_str) {
        return Some(reason);
    }

    // Rule 4: Step enforcement (artifact creation must match current step)
    if let Some(reason) = check_step_enforcement(tool_name, tool_input, state) {
        return Some(reason);
    }

    // Rule 5: State mutation enforcement (only drivers)
    if let Some(reason) =
        check_driver_only(tool_name, tool_input, state, agent_role, agent_type_str)
    {
        return Some(reason);
    }

    None
}

/// Rule 1: Deny Read/Edit/Write to workflow-controlled paths.
fn check_file_protection(tool_name: &str, tool_input: Option<&ToolInput>) -> Option<String> {
    match tool_name {
        "Read" | "Edit" | "Write" => {
            let file_path = tool_input.and_then(|t| t.file_path.as_deref())?;
            if is_protected_path(file_path) {
                return Some(format!(
                    "Direct {tool_name} to workflow-controlled path '{file_path}' is not allowed. \
                     Use the `workflow` CLI instead."
                ));
            }
            None
        }
        "Bash" => check_bash_protection(tool_input),
        _ => None,
    }
}

/// Rule 2: Deny Bash commands that read/modify controlled paths.
fn check_bash_protection(tool_input: Option<&ToolInput>) -> Option<String> {
    let command = tool_input.and_then(|t| t.command.as_deref())?;

    // Allow workflow CLI invocations
    if command.contains("workflow ") || command.starts_with("workflow") {
        return None;
    }

    // Check if the command references any protected path
    for path in PROTECTED_PATHS {
        if command.contains(path) {
            return Some(format!(
                "Bash command references workflow-controlled path '{path}'. \
                 Use the `workflow` CLI instead."
            ));
        }
    }

    None
}

/// Rule 3: Role enforcement — `agent_type` must be allowed for current step.
fn check_role(state: &State, role: Role, agent_type: &str) -> Option<String> {
    match state.phase {
        Phase::Setup | Phase::Execution => {
            let step = state.setup_step()?;
            if !role::is_allowed_setup(step, role) {
                let allowed = allowed_roles_setup(step);
                return Some(format!(
                    "Role '{agent_type}' is not allowed during step '{}'. Allowed roles: {}",
                    step.as_str(),
                    allowed
                ));
            }
        }
        Phase::Epic => {
            let step = state.epic_step()?;
            if !role::is_allowed_epic(step, role) {
                let allowed = allowed_roles_epic(step);
                return Some(format!(
                    "Role '{agent_type}' is not allowed during step '{}'. Allowed roles: {}",
                    step.as_str(),
                    allowed
                ));
            }
        }
    }
    None
}

/// Rule 4: Step enforcement — deny `workflow write` or `workflow create`
/// targeting artifacts that don't belong to the current step.
fn check_step_enforcement(
    tool_name: &str,
    tool_input: Option<&ToolInput>,
    state: &State,
) -> Option<String> {
    if tool_name != "Bash" {
        return None;
    }
    let command = tool_input.and_then(|t| t.command.as_deref())?;

    // Only check workflow write commands
    if !command.contains("workflow write") && !command.contains("workflow create") {
        return None;
    }

    let step_str = state.step.as_deref()?;

    // Map steps to allowed artifact types for write operations
    let allowed: &[&str] = match state.phase {
        Phase::Setup | Phase::Execution => match step_str {
            "product-brief" => &["product-brief"],
            "requirements" => &["requirement", "nfr"],
            "ux-foundations" => &["ux-foundations"],
            "roadmap" => &["roadmap", "milestone"],
            "architecture" => &["architecture"],
            "tech-stack" => &["tech-stack"],
            "epic-planning" => &["epic"],
            _ => return None,
        },
        Phase::Epic => match step_str {
            "analysis" => &["story", "flow", "analysis"],
            "ux-design" => &["ux-design", "story"],
            "technical-design" => &["technical-design"],
            "planning" => &["task", "plan"],
            "implementation" => &["task", "nfr"],
            "verification" => &["story", "task"],
            "review" => &["review"],
            "release" => &["release"],
            "retrospective" => &["retrospective"],
            _ => return None,
        },
    };

    // Extract the artifact type from the command
    // workflow write <type> [id] or workflow create <type> <slug>
    let artifact_type = if command.contains("workflow write") {
        extract_command_arg(command, "workflow write")
    } else {
        extract_command_arg(command, "workflow create")
    };

    if let Some(atype) = artifact_type {
        if !allowed.iter().any(|a| atype.contains(a)) {
            return Some(format!(
                "Cannot create/write '{atype}' during step '{step_str}'. \
                 Allowed artifact types: {}",
                allowed.join(", ")
            ));
        }
    }

    None
}

/// Extract the first argument after a command prefix.
fn extract_command_arg<'a>(command: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = command.split(prefix).nth(1)?.trim();
    rest.split_whitespace().next()
}

/// Rule 5: Only primary drivers can invoke state-mutating commands.
fn check_driver_only(
    tool_name: &str,
    tool_input: Option<&ToolInput>,
    state: &State,
    role: Role,
    agent_type: &str,
) -> Option<String> {
    if tool_name != "Bash" {
        return None;
    }
    let command = tool_input.and_then(|t| t.command.as_deref())?;

    // Check if this is a state-mutating workflow command
    let is_driver_cmd = DRIVER_ONLY_COMMANDS
        .iter()
        .any(|cmd| command.contains(&format!("workflow {cmd}")));

    if !is_driver_cmd {
        return None;
    }

    let is_driver = match state.phase {
        Phase::Setup | Phase::Execution => state
            .setup_step()
            .is_some_and(|s| role::is_driver_setup(s, role)),
        Phase::Epic => state
            .epic_step()
            .is_some_and(|s| role::is_driver_epic(s, role)),
    };

    if !is_driver {
        return Some(format!(
            "Role '{agent_type}' is not the primary driver and cannot invoke state-mutating commands"
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_protected_path(file_path: &str) -> bool {
    // Normalize: strip leading ./ or /
    let normalized = file_path.strip_prefix("./").unwrap_or(file_path);

    for protected in PROTECTED_PATHS {
        if normalized.starts_with(protected) {
            return true;
        }
    }

    // Check epic-level artifacts (relative paths within an epic dir)
    for pattern in EPIC_ARTIFACT_PATTERNS {
        if normalized.contains(pattern) {
            // Only if it's under a milestone's epic
            if normalized.contains("milestones/") {
                return true;
            }
        }
    }

    false
}

fn parse_role(agent_type: &str) -> Option<Role> {
    match agent_type {
        "stakeholder" => Some(Role::Stakeholder),
        "analyst" => Some(Role::Analyst),
        "ux-designer" => Some(Role::UxDesigner),
        "staff-engineer" => Some(Role::StaffEngineer),
        "product-engineer" => Some(Role::ProductEngineer),
        "platform-engineer" => Some(Role::PlatformEngineer),
        _ => None,
    }
}

fn allowed_roles_setup(step: crate::state::SetupStep) -> String {
    let roles = [
        Role::Stakeholder,
        Role::Analyst,
        Role::UxDesigner,
        Role::StaffEngineer,
        Role::ProductEngineer,
        Role::PlatformEngineer,
    ];
    roles
        .iter()
        .filter(|&&r| role::is_allowed_setup(step, r))
        .map(|r| r.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn allowed_roles_epic(step: crate::state::EpicStep) -> String {
    let roles = [
        Role::Stakeholder,
        Role::Analyst,
        Role::UxDesigner,
        Role::StaffEngineer,
        Role::ProductEngineer,
        Role::PlatformEngineer,
    ];
    roles
        .iter()
        .filter(|&&r| role::is_allowed_epic(step, r))
        .map(|r| r.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_state(step: &str) -> State {
        State {
            phase: Phase::Setup,
            step: Some(step.to_owned()),
            current_milestone: None,
            epic: None,
            active_meeting: None,
        }
    }

    fn epic_state(step: &str) -> State {
        State {
            phase: Phase::Epic,
            step: Some(step.to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        }
    }

    fn input_with_file(tool: &str, path: &str) -> HookInput {
        HookInput {
            tool_name: Some(tool.to_owned()),
            tool_input: Some(ToolInput {
                command: None,
                file_path: Some(path.to_owned()),
                old_string: None,
                new_string: None,
                content: None,
            }),
            cwd: None,
            agent_type: None,
        }
    }

    fn input_with_bash(command: &str, agent_type: Option<&str>) -> HookInput {
        HookInput {
            tool_name: Some("Bash".to_owned()),
            tool_input: Some(ToolInput {
                command: Some(command.to_owned()),
                file_path: None,
                old_string: None,
                new_string: None,
                content: None,
            }),
            cwd: None,
            agent_type: agent_type.map(str::to_owned),
        }
    }

    fn input_with_role(tool: &str, path: &str, agent_type: &str) -> HookInput {
        HookInput {
            tool_name: Some(tool.to_owned()),
            tool_input: Some(ToolInput {
                command: None,
                file_path: Some(path.to_owned()),
                old_string: None,
                new_string: None,
                content: None,
            }),
            cwd: None,
            agent_type: Some(agent_type.to_owned()),
        }
    }

    // -- Rule 1: File protection --

    #[test]
    fn deny_read_workflow_state() {
        let input = input_with_file("Read", ".workflow/state.yml");
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn deny_edit_requirement() {
        let input = input_with_file("Edit", "requirements/REQ-001-auth.md");
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn deny_write_product_brief() {
        let input = input_with_file("Write", "product-brief/product-brief.md");
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn deny_write_milestone() {
        let input = input_with_file("Write", "milestones/M-001-mvp/milestone.md");
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn allow_read_unprotected_file() {
        let input = input_with_file("Read", "src/main.rs");
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    // -- Rule 2: Bash protection --

    #[test]
    fn deny_bash_cat_requirements() {
        let input = input_with_bash("cat requirements/REQ-001-auth.md", None);
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn deny_bash_sed_workflow() {
        let input = input_with_bash("sed -i 's/draft/active/' .workflow/state.yml", None);
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn allow_bash_workflow_cli() {
        let input = input_with_bash("workflow status", None);
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    #[test]
    fn allow_bash_unrelated() {
        let input = input_with_bash("cargo test", None);
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    // -- Rule 3: Role enforcement --

    #[test]
    fn deny_wrong_role_for_step() {
        // Platform engineer not allowed at product-brief
        let input = input_with_role("Read", "src/main.rs", "platform-engineer");
        let state = setup_state("product-brief");
        let reason = evaluate(&input, &state);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("not allowed"));
    }

    #[test]
    fn allow_correct_role_for_step() {
        // Analyst is allowed at product-brief
        let input = input_with_role("Read", "src/main.rs", "analyst");
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    #[test]
    fn deny_wrong_role_epic_step() {
        // Analyst not allowed at review
        let input = input_with_role("Read", "src/main.rs", "analyst");
        let state = epic_state("review");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn allow_correct_role_epic_step() {
        // Product engineer allowed at implementation
        let input = input_with_role("Read", "src/main.rs", "product-engineer");
        let state = epic_state("implementation");
        assert!(evaluate(&input, &state).is_none());
    }

    #[test]
    fn unknown_agent_type_allows() {
        // Unknown agent types are not role-assigned agents -> allow
        let input = input_with_role("Read", "src/main.rs", "unknown-type");
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    // -- Rule 5: State mutation enforcement --

    #[test]
    fn deny_non_driver_state_mutation() {
        // Stakeholder is involved but not driver at product-brief
        let input = input_with_bash("workflow advance", Some("stakeholder"));
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn allow_driver_state_mutation() {
        // Analyst is driver at product-brief
        let input = input_with_bash("workflow advance", Some("analyst"));
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    #[test]
    fn deny_non_driver_escalate() {
        let input = input_with_bash("workflow escalate analysis --reason test", Some("analyst"));
        // Analyst is not driver at ux-design (UX designer is)
        let state = epic_state("ux-design");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn allow_driver_meeting_start() {
        let input = input_with_bash("workflow meeting start topic", Some("analyst"));
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    // -- No agent_type -> no role checks --

    #[test]
    fn no_agent_type_skips_role_checks() {
        let input = HookInput {
            tool_name: Some("Read".to_owned()),
            tool_input: Some(ToolInput {
                command: None,
                file_path: Some("src/main.rs".to_owned()),
                old_string: None,
                new_string: None,
                content: None,
            }),
            cwd: None,
            agent_type: None,
        };
        let state = setup_state("product-brief");
        assert!(evaluate(&input, &state).is_none());
    }

    // -- Protected paths --

    #[test]
    fn is_protected_path_checks() {
        assert!(is_protected_path(".workflow/state.yml"));
        assert!(is_protected_path("product-brief/product-brief.md"));
        assert!(is_protected_path("requirements/REQ-001.md"));
        assert!(is_protected_path("milestones/M-001/milestone.md"));
        assert!(is_protected_path("ux-foundations/ux-foundations.md"));
        assert!(is_protected_path("roadmap/roadmap.md"));
        assert!(is_protected_path("architecture/architecture.md"));
        assert!(is_protected_path("tech-stack/tech-stack.md"));
        assert!(!is_protected_path("src/main.rs"));
        assert!(!is_protected_path("Cargo.toml"));
    }

    #[test]
    fn is_protected_normalizes_dot_slash() {
        assert!(is_protected_path("./.workflow/state.yml"));
        assert!(is_protected_path("./requirements/REQ-001.md"));
    }

    // -- Error messages --

    #[test]
    fn role_error_includes_allowed_roles() {
        let input = input_with_role("Read", "src/main.rs", "platform-engineer");
        let state = setup_state("product-brief");
        let reason = evaluate(&input, &state).unwrap();
        assert!(reason.contains("analyst"), "should list analyst: {reason}");
    }

    #[test]
    fn file_protection_error_mentions_cli() {
        let input = input_with_file("Read", ".workflow/state.yml");
        let state = setup_state("product-brief");
        let reason = evaluate(&input, &state).unwrap();
        assert!(reason.contains("workflow"), "should mention CLI: {reason}");
    }

    // -- Rule 4: Step enforcement --

    #[test]
    fn deny_wrong_artifact_for_step() {
        // Writing technical-design during analysis step
        let input = input_with_bash("workflow write technical-design", Some("analyst"));
        let state = epic_state("analysis");
        let reason = evaluate(&input, &state);
        assert!(reason.is_some(), "should deny wrong artifact for step");
    }

    #[test]
    fn allow_correct_artifact_for_step() {
        let input = input_with_bash("workflow write analysis", Some("analyst"));
        let state = epic_state("analysis");
        assert!(evaluate(&input, &state).is_none());
    }

    #[test]
    fn deny_create_task_during_analysis() {
        let input = input_with_bash("workflow create task ui --story S-001", Some("analyst"));
        let state = epic_state("analysis");
        assert!(evaluate(&input, &state).is_some());
    }

    #[test]
    fn allow_create_task_during_planning() {
        let input = input_with_bash(
            "workflow create task ui --story S-001",
            Some("product-engineer"),
        );
        let state = epic_state("planning");
        assert!(evaluate(&input, &state).is_none());
    }
}
