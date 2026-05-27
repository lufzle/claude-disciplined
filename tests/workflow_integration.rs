use std::process::Command;

fn workflow_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workflow"))
}

fn in_temp_dir(f: impl FnOnce(&std::path::Path)) {
    let tmp = tempfile::tempdir().unwrap();
    f(tmp.path());
}

// -- init --

#[test]
fn init_succeeds_in_empty_dir() {
    in_temp_dir(|dir| {
        let output = workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"phase\": \"setup\""));
        assert!(stdout.contains("\"step\": \"product-brief\""));
    });
}

#[test]
fn init_creates_workflow_directory() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(dir.join(".workflow/state.yml").exists());
        assert!(dir.join(".workflow/counters.yml").exists());
        assert!(dir.join(".workflow/approvals.ndjson").exists());
        assert!(dir.join(".workflow/unresolved.ndjson").exists());
    });
}

#[test]
fn init_twice_fails() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("already initialized"));
    });
}

// -- status --

#[test]
fn status_after_init_shows_product_brief() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .arg("status")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"phase\": \"setup\""));
        assert!(stdout.contains("\"step\": \"product-brief\""));
    });
}

#[test]
fn status_without_init_fails() {
    in_temp_dir(|dir| {
        let output = workflow_bin()
            .arg("status")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not initialized"));
    });
}

// -- no args --

#[test]
fn no_args_shows_usage() {
    in_temp_dir(|dir| {
        let output = workflow_bin().current_dir(dir).output().unwrap();
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("usage"));
    });
}

// -- advance --

#[test]
fn advance_fails_without_approval() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .arg("advance")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("gate check failed"));
    });
}

#[test]
fn advance_succeeds_after_approval() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["request-approval", "product-brief"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .arg("advance")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"step\": \"requirements\""));
    });
}

// -- request-approval --

#[test]
fn request_approval_succeeds() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["request-approval", "product-brief"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
    });
}

#[test]
fn request_approval_without_step_arg_fails() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .arg("request-approval")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("missing argument"));
    });
}

// -- create requirement --

#[test]
fn create_requirement_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["create", "requirement", "build-apps"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-001"));
    });
}

#[test]
fn create_requirement_creates_file() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "build-apps"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(dir.join("requirements/REQ-001-build-apps.md").exists());
    });
}

// -- create milestone --

#[test]
fn create_milestone_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["create", "milestone", "basic-app"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("M-001"));
    });
}

// -- propose nfr --

#[test]
fn propose_nfr_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["propose", "nfr", "page-load-under-2s"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("NFR-001"));
    });
}

// -- create missing args --

#[test]
fn create_without_type_fails() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .arg("create")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

// -- create epic (needs current_milestone in state) --

fn set_state_yaml(dir: &std::path::Path, content: &str) {
    std::fs::write(dir.join(".workflow/state.yml"), content).unwrap();
}

#[test]
fn create_epic_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        let output = workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("E-001"));
    });
}

// -- create story (needs milestone + epic in state) --

#[test]
fn create_story_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: analysis\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        let output = workflow_bin()
            .args(["create", "story", "login"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("S-001"));
    });
}

// -- create task (needs story context via --story flag) --

#[test]
fn create_task_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: analysis\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        workflow_bin()
            .args(["create", "story", "login"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: planning\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        let output = workflow_bin()
            .args(["create", "task", "ui-form", "--story", "S-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("T-001"));
    });
}

// -- create flow (needs story context + --type flag) --

#[test]
fn create_flow_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: analysis\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        workflow_bin()
            .args(["create", "story", "login"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args([
                "create",
                "flow",
                "happy",
                "--story",
                "S-001",
                "--type",
                "happy-path",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("F-001"));
    });
}

// -- create missing args --

#[test]
fn create_epic_without_milestone_fails() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

// -- read --

#[test]
fn read_requirement_after_create() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "build-apps"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["read", "requirement", "REQ-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("id: REQ-001"));
    });
}

// -- write --

#[test]
fn write_then_read_preserves_front_matter_and_body() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "build-apps"])
            .current_dir(dir)
            .output()
            .unwrap();
        // Write body via stdin
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_workflow"))
            .args(["write", "requirement", "REQ-001"])
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        {
            use std::io::Write;
            let stdin = child.stdin.as_mut().unwrap();
            stdin.write_all(b"Users can build apps.\n").unwrap();
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Read back
        let output = workflow_bin()
            .args(["read", "requirement", "REQ-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("id: REQ-001"));
        assert!(stdout.contains("Users can build apps."));
    });
}

// -- meeting start/end/read/contribute/list --

#[test]
fn meeting_start_outputs_path() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "start", "vision-interview"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("001-vision-interview"));
    });
}

#[test]
fn meeting_end_succeeds_after_start() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "end"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
    });
}

#[test]
fn meeting_read_returns_notes() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "read"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("# kickoff"));
    });
}

#[test]
fn meeting_contribute_appends() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "contribute", "The", "product", "is", "simple."])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "read"])
            .current_dir(dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("The product is simple."));
    });
}

#[test]
fn meeting_list_returns_json() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "first"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "end"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "list"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("001-first"));
    });
}

// -- meeting decision commands --

#[test]
fn propose_decision_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "propose-decision", "Use", "REST", "for", "API"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("D-001"));
    });
}

#[test]
fn decision_lifecycle_propose_position_record() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "propose-decision", "Use", "REST"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "position", "D-001", "agree", "--role", "analyst"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "record-decision", "D-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

#[test]
fn decision_status_returns_json() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "propose-decision", "Use", "REST"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "decision-status", "D-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"summary\""));
        assert!(stdout.contains("Use REST"));
    });
}

#[test]
fn drop_decision_succeeds() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "propose-decision", "Use", "REST"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args([
                "meeting",
                "drop-decision",
                "D-001",
                "--reason",
                "not needed",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

#[test]
fn list_decisions_returns_json() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "propose-decision", "Use", "REST"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "list-decisions"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Use REST"));
    });
}

// -- position --reason enforcement --

#[test]
fn position_disagree_without_reason_fails() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "propose-decision", "Use", "REST"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args([
                "meeting", "position", "D-001", "disagree", "--role", "analyst",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--reason is required"));
    });
}

// -- meeting action items --

#[test]
fn add_action_outputs_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args([
                "meeting",
                "add-action",
                "Research",
                "event",
                "sourcing",
                "--type",
                "research",
                "--assignee",
                "platform-engineer",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("AI-001"));
    });
}

#[test]
fn complete_action_succeeds() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args([
                "meeting",
                "add-action",
                "Research",
                "X",
                "--type",
                "research",
                "--assignee",
                "analyst",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args([
                "meeting",
                "complete-action",
                "AI-001",
                "--summary",
                "X is good",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

#[test]
fn list_actions_returns_json() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["meeting", "start", "kickoff"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args([
                "meeting",
                "add-action",
                "Research",
                "X",
                "--type",
                "research",
                "--assignee",
                "analyst",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["meeting", "list-actions"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Research X"));
    });
}

// -- task lifecycle --

/// Set up a temp dir with init -> milestone -> epic -> story -> task,
/// with state pointing at the epic's implementation step.
fn setup_with_task(dir: &std::path::Path) {
    workflow_bin()
        .arg("init")
        .current_dir(dir)
        .output()
        .unwrap();
    workflow_bin()
        .args(["create", "milestone", "mvp"])
        .current_dir(dir)
        .output()
        .unwrap();
    set_state_yaml(
        dir,
        "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
    );
    workflow_bin()
        .args(["create", "epic", "auth"])
        .current_dir(dir)
        .output()
        .unwrap();
    set_state_yaml(
        dir,
        "phase: epic\nstep: implementation\ncurrent_milestone: M-001\nepic: E-001\n",
    );
    workflow_bin()
        .args(["create", "story", "login"])
        .current_dir(dir)
        .output()
        .unwrap();
    workflow_bin()
        .args(["create", "task", "ui-form", "--story", "S-001"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn task_start_succeeds() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        let output = workflow_bin()
            .args(["task", "start", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

#[test]
fn task_start_changes_status() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        workflow_bin()
            .args(["task", "start", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["read", "task", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("status: in-progress"));
    });
}

#[test]
fn task_complete_succeeds() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        let output = workflow_bin()
            .args(["task", "complete", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

#[test]
fn task_block_sets_reason() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        workflow_bin()
            .args(["task", "block", "T-001", "--reason", "waiting on API"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["read", "task", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("status: blocked"));
        assert!(stdout.contains("blocked_reason: waiting on API"));
    });
}

#[test]
fn task_block_without_reason_fails() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        let output = workflow_bin()
            .args(["task", "block", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--reason"));
    });
}

#[test]
fn task_unblock_clears_reason() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        workflow_bin()
            .args(["task", "block", "T-001", "--reason", "dep"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["task", "unblock", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["read", "task", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("status: pending"));
        assert!(stdout.contains("blocked_reason: null"));
    });
}

#[test]
fn task_list_returns_json() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        let output = workflow_bin()
            .args(["task", "list"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("T-001"));
        assert!(stdout.contains("pending"));
    });
}

#[test]
fn task_list_filters_by_status() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        workflow_bin()
            .args(["create", "task", "api-call", "--story", "S-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["task", "start", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["task", "list", "--status", "pending"])
            .current_dir(dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("T-002"));
        assert!(!stdout.contains("T-001"));
    });
}

#[test]
fn task_list_filters_by_epic() {
    in_temp_dir(|dir| {
        // Init + milestone + two epics
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "epic", "billing"])
            .current_dir(dir)
            .output()
            .unwrap();

        // Story + task in E-001
        set_state_yaml(
            dir,
            "phase: epic\nstep: implementation\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        workflow_bin()
            .args(["create", "story", "login"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "task", "ui", "--story", "S-001"])
            .current_dir(dir)
            .output()
            .unwrap();

        // Story + task in E-002
        set_state_yaml(
            dir,
            "phase: epic\nstep: implementation\ncurrent_milestone: M-001\nepic: E-002\n",
        );
        workflow_bin()
            .args(["create", "story", "pay"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "task", "stripe", "--story", "S-002"])
            .current_dir(dir)
            .output()
            .unwrap();

        // List tasks for E-001 (not the current epic)
        let output = workflow_bin()
            .args(["task", "list", "--epic", "E-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("T-001"));
        assert!(!stdout.contains("T-002"));
    });
}

#[test]
fn task_unknown_subcommand_fails() {
    in_temp_dir(|dir| {
        setup_with_task(dir);
        let output = workflow_bin()
            .args(["task", "bogus"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unknown task subcommand"));
    });
}

// -- epic lifecycle --

fn setup_for_epic_start(dir: &std::path::Path) {
    // Need a git repo for branch/worktree operations
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .unwrap();

    workflow_bin()
        .arg("init")
        .current_dir(dir)
        .output()
        .unwrap();
    workflow_bin()
        .args(["create", "milestone", "mvp"])
        .current_dir(dir)
        .output()
        .unwrap();
    set_state_yaml(
        dir,
        "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
    );
    workflow_bin()
        .args(["create", "epic", "auth"])
        .current_dir(dir)
        .output()
        .unwrap();

    // Need an initial commit for worktree to work
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn epic_start_creates_branch_and_worktree() {
    in_temp_dir(|dir| {
        setup_for_epic_start(dir);
        let output = workflow_bin()
            .args(["epic", "start", "E-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("M-001/E-001-auth"));

        // Verify worktree was created
        assert!(dir.join(".worktrees/M-001/E-001-auth").exists());

        // Verify state.yml in worktree has epic phase
        let wt_state =
            std::fs::read_to_string(dir.join(".worktrees/M-001/E-001-auth/.workflow/state.yml"))
                .unwrap();
        assert!(wt_state.contains("epic"));
        assert!(wt_state.contains("analysis"));
    });
}

#[test]
fn epic_start_sets_status_active() {
    in_temp_dir(|dir| {
        setup_for_epic_start(dir);
        workflow_bin()
            .args(["epic", "start", "E-001"])
            .current_dir(dir)
            .output()
            .unwrap();

        // Read epic.md on main — should be active
        let epic_md =
            std::fs::read_to_string(dir.join("milestones/M-001-mvp/epics/E-001-auth/epic.md"))
                .unwrap();
        assert!(epic_md.contains("status: active"));
    });
}

#[test]
fn epic_start_fails_outside_epic_planning() {
    in_temp_dir(|dir| {
        setup_for_epic_start(dir);
        set_state_yaml(dir, "phase: setup\nstep: product-brief\n");
        let output = workflow_bin()
            .args(["epic", "start", "E-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

#[test]
fn epic_complete_succeeds_in_retrospective() {
    in_temp_dir(|dir| {
        setup_for_epic_start(dir);
        workflow_bin()
            .args(["epic", "start", "E-001"])
            .current_dir(dir)
            .output()
            .unwrap();

        // Switch to the worktree and set retrospective step
        let wt = dir.join(".worktrees/M-001/E-001-auth");
        set_state_yaml(
            &wt,
            "phase: epic\nstep: retrospective\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        let output = workflow_bin()
            .args(["epic", "complete", "E-001"])
            .current_dir(&wt)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

#[test]
fn epic_list_returns_json() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["epic", "list"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("E-001"));
        assert!(stdout.contains("draft"));
    });
}

#[test]
fn epic_list_filters_by_status() {
    in_temp_dir(|dir| {
        setup_for_epic_start(dir);
        workflow_bin()
            .args(["epic", "start", "E-001"])
            .current_dir(dir)
            .output()
            .unwrap();

        // Reset state so we can create another epic
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "billing"])
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["epic", "list", "--status", "draft"])
            .current_dir(dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("E-002"));
        assert!(!stdout.contains("E-001"));
    });
}

#[test]
fn epic_unknown_subcommand_fails() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        let output = workflow_bin()
            .args(["epic", "bogus"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unknown epic subcommand"));
    });
}

// -- escalate --

fn setup_for_escalation(dir: &std::path::Path, step: &str) {
    workflow_bin()
        .arg("init")
        .current_dir(dir)
        .output()
        .unwrap();
    workflow_bin()
        .args(["create", "milestone", "mvp"])
        .current_dir(dir)
        .output()
        .unwrap();
    set_state_yaml(
        dir,
        "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
    );
    workflow_bin()
        .args(["create", "epic", "auth"])
        .current_dir(dir)
        .output()
        .unwrap();
    set_state_yaml(
        dir,
        &format!("phase: epic\nstep: {step}\ncurrent_milestone: M-001\nepic: E-001\n"),
    );
    // Start a meeting in the current step
    workflow_bin()
        .args(["meeting", "start", "review"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn escalate_changes_step() {
    in_temp_dir(|dir| {
        setup_for_escalation(dir, "ux-design");
        let output = workflow_bin()
            .args(["escalate", "analysis", "--reason", "stories need rework"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("D-001"));

        // Verify step changed
        let status = workflow_bin()
            .arg("status")
            .current_dir(dir)
            .output()
            .unwrap();
        let status_out = String::from_utf8_lossy(&status.stdout);
        assert!(status_out.contains("analysis"));
    });
}

#[test]
fn escalate_without_reason_fails() {
    in_temp_dir(|dir| {
        setup_for_escalation(dir, "ux-design");
        let output = workflow_bin()
            .args(["escalate", "analysis"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--reason"));
    });
}

#[test]
fn escalate_invalid_transition_fails() {
    in_temp_dir(|dir| {
        setup_for_escalation(dir, "ux-design");
        let output = workflow_bin()
            .args(["escalate", "implementation", "--reason", "invalid"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

#[test]
fn escalate_without_meeting_fails() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: ux-design\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        // No meeting started
        let output = workflow_bin()
            .args(["escalate", "analysis", "--reason", "reason"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

// -- workflow-hook --

fn hook_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workflow-hook"))
}

#[test]
fn hook_allows_no_state_file() {
    in_temp_dir(|dir| {
        // No .workflow/ -> allow
        let mut child = hook_bin()
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let input = serde_json::json!({
            "tool_name": "Read",
            "tool_input": {"file_path": "requirements/REQ-001.md"},
            "cwd": dir.to_str().unwrap()
        });
        std::io::Write::write_all(
            &mut child.stdin.take().unwrap(),
            serde_json::to_string(&input).unwrap().as_bytes(),
        )
        .unwrap();

        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "should allow when no state file exists"
        );
    });
}

#[test]
fn hook_denies_direct_file_access() {
    in_temp_dir(|dir| {
        // Create state file
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        let mut child = hook_bin()
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let input = serde_json::json!({
            "tool_name": "Read",
            "tool_input": {"file_path": "requirements/REQ-001.md"},
            "cwd": dir.to_str().unwrap()
        });
        std::io::Write::write_all(
            &mut child.stdin.take().unwrap(),
            serde_json::to_string(&input).unwrap().as_bytes(),
        )
        .unwrap();

        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(2), "should deny");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("permissionDecision"));
        assert!(stderr.contains("deny"));
    });
}

#[test]
fn hook_denies_wrong_role() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        let mut child = hook_bin()
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let input = serde_json::json!({
            "tool_name": "Read",
            "tool_input": {"file_path": "src/main.rs"},
            "cwd": dir.to_str().unwrap(),
            "agent_type": "platform-engineer"
        });
        std::io::Write::write_all(
            &mut child.stdin.take().unwrap(),
            serde_json::to_string(&input).unwrap().as_bytes(),
        )
        .unwrap();

        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(2), "should deny wrong role");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not allowed"));
    });
}

#[test]
fn hook_allows_correct_role() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        let mut child = hook_bin()
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let input = serde_json::json!({
            "tool_name": "Read",
            "tool_input": {"file_path": "src/main.rs"},
            "cwd": dir.to_str().unwrap(),
            "agent_type": "analyst"
        });
        std::io::Write::write_all(
            &mut child.stdin.take().unwrap(),
            serde_json::to_string(&input).unwrap().as_bytes(),
        )
        .unwrap();

        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "should allow correct role");
    });
}

#[test]
fn hook_allows_malformed_input() {
    in_temp_dir(|dir| {
        let mut child = hook_bin()
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        std::io::Write::write_all(&mut child.stdin.take().unwrap(), b"not json").unwrap();

        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "should allow malformed input");
    });
}

// -- validate --

#[test]
fn validate_outputs_findings_json() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["validate"])
            .current_dir(dir)
            .output()
            .unwrap();
        // Should have findings (warnings at minimum for empty links)
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"severity\""));
        assert!(stdout.contains("\"rule\""));
    });
}

#[test]
fn validate_links_succeeds_when_no_broken() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["validate", "links"])
            .current_dir(dir)
            .output()
            .unwrap();
        // No artifacts = no broken links
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

// -- query --

#[test]
fn query_requirements_lists_all() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "billing"])
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["query", "requirements"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-001"));
        assert!(stdout.contains("REQ-002"));
    });
}

#[test]
fn query_coverage_reports_uncovered() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["query", "coverage"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("REQ-001"),
            "should report uncovered requirement"
        );
    });
}

#[test]
fn query_trace_shows_chain() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "milestone", "mvp"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: execution\nstep: epic-planning\ncurrent_milestone: M-001\n",
        );
        workflow_bin()
            .args(["create", "epic", "auth"])
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: implementation\ncurrent_milestone: M-001\nepic: E-001\n",
        );
        workflow_bin()
            .args(["create", "story", "login"])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "task", "ui", "--story", "S-001"])
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["query", "trace", "T-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("task"), "should include task level");
        assert!(stdout.contains("story"), "should include story level");
        assert!(stdout.contains("epic"), "should include epic level");
        assert!(
            stdout.contains("milestone"),
            "should include milestone level"
        );
    });
}

// -- unresolved --

#[test]
fn unresolved_add_and_list() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        // Need epic context for state
        set_state_yaml(
            dir,
            "phase: epic\nstep: retrospective\ncurrent_milestone: M-001\nepic: E-001\n",
        );

        let add_output = workflow_bin()
            .args([
                "unresolved",
                "add",
                "--decision",
                "D-003",
                "--epic",
                "E-001",
                "--summary",
                "GraphQL vs REST",
                "--dissenter",
                "platform-engineer",
                "--reason",
                "GraphQL better",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            add_output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&add_output.stderr)
        );
        let stdout = String::from_utf8_lossy(&add_output.stdout);
        assert!(stdout.contains("UR-001"));

        let list_output = workflow_bin()
            .args(["unresolved", "list"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(list_output.status.success());
        let list_stdout = String::from_utf8_lossy(&list_output.stdout);
        assert!(list_stdout.contains("GraphQL vs REST"));
    });
}

#[test]
fn unresolved_review_and_history() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: retrospective\ncurrent_milestone: M-001\nepic: E-001\n",
        );

        workflow_bin()
            .args([
                "unresolved",
                "add",
                "--decision",
                "D-001",
                "--epic",
                "E-001",
                "--summary",
                "test issue",
                "--dissenter",
                "analyst",
                "--reason",
                "reason",
            ])
            .current_dir(dir)
            .output()
            .unwrap();

        let review_output = workflow_bin()
            .args([
                "unresolved",
                "review",
                "UR-001",
                "--outcome",
                "deferred",
                "--notes",
                "need more data",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            review_output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&review_output.stderr)
        );

        let history_output = workflow_bin()
            .args(["unresolved", "history", "UR-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(history_output.status.success());
        let history_stdout = String::from_utf8_lossy(&history_output.stdout);
        assert!(history_stdout.contains("deferred"));
        assert!(history_stdout.contains("need more data"));
    });
}

#[test]
fn unresolved_list_filters_by_status() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        set_state_yaml(
            dir,
            "phase: epic\nstep: retrospective\ncurrent_milestone: M-001\nepic: E-001\n",
        );

        // Add two items
        workflow_bin()
            .args([
                "unresolved",
                "add",
                "--decision",
                "D-001",
                "--epic",
                "E-001",
                "--summary",
                "issue one",
                "--dissenter",
                "analyst",
                "--reason",
                "r1",
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args([
                "unresolved",
                "add",
                "--decision",
                "D-002",
                "--epic",
                "E-001",
                "--summary",
                "issue two",
                "--dissenter",
                "analyst",
                "--reason",
                "r2",
            ])
            .current_dir(dir)
            .output()
            .unwrap();

        // Resolve the first
        workflow_bin()
            .args([
                "unresolved",
                "review",
                "UR-001",
                "--outcome",
                "resolved",
                "--notes",
                "done",
            ])
            .current_dir(dir)
            .output()
            .unwrap();

        let open_output = workflow_bin()
            .args(["unresolved", "list", "--status", "open"])
            .current_dir(dir)
            .output()
            .unwrap();
        let open_stdout = String::from_utf8_lossy(&open_output.stdout);
        assert!(open_stdout.contains("issue two"));
        assert!(!open_stdout.contains("issue one"));
    });
}

// -- update --

#[test]
fn update_changes_status_and_timestamp() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();
        workflow_bin()
            .args(["create", "requirement", "auth-login"])
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["update", "REQ-001", "--status", "superseded"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let content =
            std::fs::read_to_string(dir.join("requirements/REQ-001-auth-login.md")).unwrap();
        assert!(content.contains("status: superseded"));
    });
}

#[test]
fn update_fails_without_status_flag() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["update", "REQ-001"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

#[test]
fn update_fails_without_id() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["update"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

#[test]
fn update_fails_for_nonexistent_artifact() {
    in_temp_dir(|dir| {
        workflow_bin()
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap();

        let output = workflow_bin()
            .args(["update", "REQ-999", "--status", "active"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
    });
}

// -- unknown command --

#[test]
fn unknown_command_fails() {
    in_temp_dir(|dir| {
        let output = workflow_bin()
            .arg("bogus")
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unknown command"));
    });
}
