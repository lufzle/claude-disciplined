use std::process::ExitCode;

use claude_disciplined::{
    commands, resolve,
    store::{FsStore, Store},
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let Some(cmd) = args.first() else {
        eprintln!("usage: workflow <command> [args...]");
        return ExitCode::FAILURE;
    };

    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let store = FsStore::new(&cwd);
    let rest = &args[1..];

    let result = match cmd.as_str() {
        "init" => run_json(&store, commands::init),
        "status" => run_json(&store, commands::status),
        "advance" => run_json(&store, commands::advance),
        "request-approval" => {
            run_with_arg(rest, "step", |arg| commands::request_approval(&store, arg))
        }
        "validate" => run_validate(&store, rest),
        "verify" => run_verify(&store, rest),
        "query" => run_query(&store, rest),
        "update" => run_update(&store, rest),
        "unresolved" => run_unresolved(&store, rest),
        "escalate" => run_escalate(&store, rest),
        "create" => run_create(&store, rest),
        "propose" => run_propose(&store, rest),
        "read" => run_read(&store, rest),
        "write" => run_write(&store, rest),
        "meeting" => run_meeting(&store, rest),
        "action" => run_action(&store, rest),
        "task" => run_task(&store, rest),
        "epic" => run_epic(&store, rest),
        _ => {
            eprintln!("unknown command: {cmd}");
            Err(())
        }
    };

    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run_json<T: serde::Serialize>(
    store: &FsStore,
    f: impl FnOnce(&FsStore) -> Result<T, commands::CmdError>,
) -> Result<(), ()> {
    match f(store) {
        Ok(value) => {
            let json = serde_json::to_string_pretty(&value).expect("serialization");
            println!("{json}");
            Ok(())
        }
        Err(e) => {
            eprintln!("{e}");
            Err(())
        }
    }
}

fn run_with_arg<T>(
    args: &[String],
    arg_name: &str,
    f: impl FnOnce(&str) -> Result<T, commands::CmdError>,
) -> Result<(), ()> {
    let arg = require_arg(args, 0, arg_name)?;
    f(arg).map(|_| ()).map_err(|e| eprintln!("{e}"))
}

fn print_id(result: commands::CmdResult<claude_disciplined::id::Id>) -> Result<(), ()> {
    match result {
        Ok(id) => {
            println!("{id}");
            Ok(())
        }
        Err(e) => {
            eprintln!("{e}");
            Err(())
        }
    }
}

fn run_create(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let kind = require_arg(args, 0, "type")?;
    match kind {
        // Context-free: requirement, milestone
        "requirement" => {
            let slug = require_arg(args, 1, "slug")?;
            print_id(commands::create_requirement(store, slug))
        }
        "milestone" => {
            let slug = require_arg(args, 1, "slug")?;
            print_id(commands::create_milestone(store, slug))
        }
        // Resolved from state: epic (needs current milestone)
        "epic" => {
            let slug = require_arg(args, 1, "slug")?;
            let m_dir = resolve::current_milestone_dir(store).map_err(|e| eprintln!("{e}"))?;
            print_id(commands::create_epic(store, &m_dir, slug))
        }
        // Resolved from state: story (needs current epic)
        "story" => {
            let slug = require_arg(args, 1, "slug")?;
            let e_dir = resolve::current_epic_dir(store).map_err(|e| eprintln!("{e}"))?;
            print_id(commands::create_story(store, &e_dir, slug))
        }
        // Needs story context: task <slug> --story <story-id>
        "task" => {
            let slug = require_arg(args, 1, "slug")?;
            let story_id = require_flag(args, "--story")?;
            let e_dir = resolve::current_epic_dir(store).map_err(|e| eprintln!("{e}"))?;
            let s_dir = store
                .find_dir(&format!("{e_dir}/stories/{story_id}-*"))
                .ok_or_else(|| eprintln!("story directory not found for {story_id}"))?;
            print_id(commands::create_task(store, &s_dir, slug))
        }
        // Needs story context: flow <slug> --story <story-id> --type <type>
        "flow" => {
            let slug = require_arg(args, 1, "slug")?;
            let story_id = require_flag(args, "--story")?;
            let flow_type = require_flag(args, "--type")?;
            let e_dir = resolve::current_epic_dir(store).map_err(|e| eprintln!("{e}"))?;
            let s_dir = store
                .find_dir(&format!("{e_dir}/stories/{story_id}-*"))
                .ok_or_else(|| eprintln!("story directory not found for {story_id}"))?;
            print_id(commands::create_flow(store, &s_dir, slug, flow_type))
        }
        _ => {
            eprintln!("unknown artifact type: {kind}");
            Err(())
        }
    }
}

fn run_propose(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let kind = require_arg(args, 0, "type")?;
    if kind != "nfr" {
        eprintln!("unknown proposal type: {kind}");
        return Err(());
    }
    let slug = require_arg(args, 1, "slug")?;
    print_id(commands::propose_nfr(store, slug))
}

fn run_read(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let artifact_type = require_arg(args, 0, "type")?;
    let id = args.get(1).map(String::as_str);
    print_ok(commands::read_artifact(store, artifact_type, id))
}

fn run_write(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let artifact_type = require_arg(args, 0, "type")?;
    let id = args.get(1).map(String::as_str);
    let mut content = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut content)
        .map_err(|e| eprintln!("{e}"))?;
    commands::write_artifact(store, artifact_type, id, &content).map_err(|e| eprintln!("{e}"))
}

fn run_meeting(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = require_arg(args, 0, "subcommand")?;
    match subcmd {
        "start" | "end" | "read" | "contribute" | "list" => {
            run_meeting_lifecycle(store, subcmd, args)
        }
        "propose-decision"
        | "position"
        | "record-decision"
        | "resolve-decision"
        | "drop-decision"
        | "supersede-decision"
        | "decision-status"
        | "list-decisions"
        | "address-disagreement" => run_meeting_decision(store, subcmd, args),
        "add-action" | "start-action" | "complete-action" | "discard-action" | "list-actions" => {
            run_meeting_action(store, subcmd, args)
        }
        _ => {
            eprintln!("unknown meeting subcommand: {subcmd}");
            Err(())
        }
    }
}

fn run_meeting_decision(store: &FsStore, subcmd: &str, args: &[String]) -> Result<(), ()> {
    match subcmd {
        "propose-decision" => {
            let summary = args[1..].join(" ");
            if summary.is_empty() {
                eprintln!("missing argument: <summary>");
                return Err(());
            }
            print_ok(commands::propose_decision(store, &summary))
        }
        "position" => {
            let decision_id = require_arg(args, 1, "decision-id")?;
            let pos = require_arg(args, 2, "position")?;
            let role = require_flag(args, "--role")?;
            let kind = parse_position_kind(pos)?;
            let reason = find_flag(args, "--reason").map(str::to_owned);
            if matches!(
                kind,
                claude_disciplined::decision::PositionKind::Disagree
                    | claude_disciplined::decision::PositionKind::DisagreeAndCommit
            ) && reason.is_none()
            {
                eprintln!("--reason is required for disagree positions");
                return Err(());
            }
            ok_or_err(commands::position(store, decision_id, role, kind, reason))
        }
        "record-decision" => {
            let id = require_arg(args, 1, "decision-id")?;
            ok_or_err(commands::record_decision(store, id))
        }
        "resolve-decision" => {
            let id = require_arg(args, 1, "decision-id")?;
            let role = require_flag(args, "--role")?;
            let justification = require_flag(args, "--justification")?;
            ok_or_err(commands::resolve_decision(
                store,
                id,
                role,
                justification,
                3,
            ))
        }
        "drop-decision" => {
            let id = require_arg(args, 1, "decision-id")?;
            let reason = require_flag(args, "--reason")?;
            ok_or_err(commands::drop_decision(store, id, reason))
        }
        "supersede-decision" => {
            let old_id = require_arg(args, 1, "decision-id")?;
            let new_id = require_flag(args, "--by")?;
            ok_or_err(commands::supersede_decision(store, old_id, new_id))
        }
        "decision-status" => {
            let id = require_arg(args, 1, "decision-id")?;
            print_json(commands::decision_status(store, id))
        }
        "list-decisions" => print_json(commands::list_decisions(store)),
        "address-disagreement" => {
            let id = require_arg(args, 1, "decision-id")?;
            let role = require_flag(args, "--role")?;
            ok_or_err(commands::address_disagreement(store, id, role))
        }
        _ => unreachable!(),
    }
}

fn run_meeting_action(store: &FsStore, subcmd: &str, args: &[String]) -> Result<(), ()> {
    match subcmd {
        "add-action" => {
            let desc = args[1..]
                .iter()
                .take_while(|a| !a.starts_with("--"))
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            if desc.is_empty() {
                eprintln!("missing argument: <description>");
                return Err(());
            }
            let action_type_str = require_flag(args, "--type")?;
            let action_type =
                commands::parse_action_type(action_type_str).map_err(|e| eprintln!("{e}"))?;
            let assignee = require_flag(args, "--assignee")?;
            let immediate = find_flag(args, "--immediate").is_some();
            print_ok(commands::add_action(
                store,
                &desc,
                action_type,
                assignee,
                immediate,
            ))
        }
        "start-action" => {
            let id = require_arg(args, 1, "action-id")?;
            ok_or_err(commands::start_action(store, id))
        }
        "complete-action" => {
            let id = require_arg(args, 1, "action-id")?;
            let summary = require_flag(args, "--summary")?;
            ok_or_err(commands::complete_action(store, id, summary))
        }
        "discard-action" => {
            let id = require_arg(args, 1, "action-id")?;
            let reason = require_flag(args, "--reason")?;
            ok_or_err(commands::discard_action(store, id, reason))
        }
        "list-actions" => print_json(commands::list_actions(store)),
        _ => unreachable!(),
    }
}

fn run_meeting_lifecycle(store: &FsStore, subcmd: &str, args: &[String]) -> Result<(), ()> {
    match subcmd {
        "start" => {
            let topic = require_arg(args, 1, "topic")?;
            print_ok(commands::meeting_start(store, topic))
        }
        "end" => ok_or_err(commands::meeting_end(store)),
        "read" => print_ok(commands::meeting_read(store)),
        "contribute" => {
            let message = args[1..].join(" ");
            if message.is_empty() {
                eprintln!("missing argument: <message>");
                return Err(());
            }
            ok_or_err(commands::meeting_contribute(store, &message))
        }
        "list" => print_json(commands::meeting_list(store)),
        _ => unreachable!(),
    }
}

fn run_validate(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = args.first().map(String::as_str);
    let result = match subcmd {
        Some("milestone") => {
            let id = require_arg(args, 1, "milestone-id")?;
            commands::validate_project_milestone(store, id)
        }
        Some("epic") => {
            let id = require_arg(args, 1, "epic-id")?;
            commands::validate_project_epic(store, id)
        }
        Some("links") => commands::validate_project_links(store),
        Some("coverage") => commands::validate_project_coverage(store),
        None => commands::validate_project(store),
        Some(other) => {
            eprintln!("unknown validate subcommand: {other}");
            return Err(());
        }
    };

    match result {
        Ok(findings) => {
            let has_errors = findings
                .iter()
                .any(|f| f.severity == claude_disciplined::commands::validate::Severity::Error);
            let json = serde_json::to_string_pretty(&findings).expect("serialization");
            println!("{json}");
            if has_errors { Err(()) } else { Ok(()) }
        }
        Err(e) => {
            eprintln!("{e}");
            Err(())
        }
    }
}

fn run_verify(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = require_arg(args, 0, "subcommand")?;
    match subcmd {
        "story" => {
            let id = require_arg(args, 1, "story-id")?;
            print_json(commands::verify_story(store, id))
        }
        "epic" => {
            let id = require_arg(args, 1, "epic-id")?;
            print_json(commands::verify_epic(store, id))
        }
        "report" => print_json(commands::verify_report(store)),
        _ => {
            eprintln!("unknown verify subcommand: {subcmd}");
            Err(())
        }
    }
}

fn run_query(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = require_arg(args, 0, "query-type")?;
    match subcmd {
        "requirements" => print_json(commands::query_requirements(store)),
        "nfrs" => {
            let status_filter = find_flag(args, "--status");
            print_json(commands::query_nfrs(store, status_filter))
        }
        "milestones" => {
            let for_filter = find_flag(args, "--for");
            print_json(commands::query_milestones(store, for_filter))
        }
        "epics" => {
            let for_filter = find_flag(args, "--for");
            print_json(commands::query_epics(store, for_filter))
        }
        "stories" => {
            let for_filter = find_flag(args, "--for");
            print_json(commands::query_stories(store, for_filter))
        }
        "tasks" => {
            let for_filter = find_flag(args, "--for");
            print_json(commands::query_tasks(store, for_filter))
        }
        "flows" => {
            let for_filter = find_flag(args, "--for");
            print_json(commands::query_flows(store, for_filter))
        }
        "decisions" => {
            let status_filter = find_flag(args, "--status");
            print_json(commands::query_decisions(store, status_filter))
        }
        "actions" => {
            let type_filter = find_flag(args, "--type");
            let status_filter = find_flag(args, "--status");
            let assignee_filter = find_flag(args, "--assignee");
            print_json(commands::query_actions(
                store,
                type_filter,
                status_filter,
                assignee_filter,
            ))
        }
        "trace" => {
            let id = require_arg(args, 1, "id")?;
            print_json(commands::query_trace(store, id))
        }
        "impact" => {
            let id = require_arg(args, 1, "id")?;
            print_json(commands::query_impact(store, id))
        }
        "rationale" => {
            let id = require_arg(args, 1, "id")?;
            print_json(commands::query_rationale(store, id))
        }
        "coverage" => print_json(commands::query_coverage(store)),
        "goal" => {
            let fragment = require_arg(args, 1, "fragment")?;
            print_json(commands::query_goal(store, fragment))
        }
        _ => {
            eprintln!("unknown query type: {subcmd}");
            Err(())
        }
    }
}

fn run_update(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let id = require_arg(args, 0, "id")?;
    let status = require_flag(args, "--status")?;
    ok_or_err(commands::update(store, id, status))
}

fn run_unresolved(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = require_arg(args, 0, "subcommand")?;
    match subcmd {
        "add" => {
            let decision_id = require_flag(args, "--decision")?;
            let epic_id = require_flag(args, "--epic")?;
            let summary = require_flag(args, "--summary")?;
            let dissenter = require_flag(args, "--dissenter")?;
            let reason = require_flag(args, "--reason")?;
            print_ok(commands::add_unresolved(
                store,
                decision_id,
                epic_id,
                summary,
                dissenter,
                reason,
            ))
        }
        "list" => {
            let status_filter = find_flag(args, "--status");
            print_json(commands::list_unresolved(store, status_filter))
        }
        "review" => {
            let id = require_arg(args, 1, "id")?;
            let outcome_str = require_flag(args, "--outcome")?;
            let outcome = claude_disciplined::unresolved::parse_outcome(outcome_str)
                .map_err(|e| eprintln!("{e}"))?;
            let notes = require_flag(args, "--notes")?;
            ok_or_err(commands::review_unresolved(store, id, outcome, notes))
        }
        "history" => {
            let id = require_arg(args, 1, "id")?;
            print_json(commands::history_unresolved(store, id))
        }
        _ => {
            eprintln!("unknown unresolved subcommand: {subcmd}");
            Err(())
        }
    }
}

fn run_escalate(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let target = require_arg(args, 0, "target-step")?;
    let reason = require_flag(args, "--reason")?;
    print_ok(commands::escalate(store, target, reason))
}

fn run_epic(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = require_arg(args, 0, "subcommand")?;
    match subcmd {
        "start" => {
            let id = require_arg(args, 1, "epic-id")?;
            let info = commands::epic_start(store, id).map_err(|e| eprintln!("{e}"))?;

            // Create git branch
            let branch_status = std::process::Command::new("git")
                .args(["branch", &info.branch_name])
                .output()
                .map_err(|e| eprintln!("git branch failed: {e}"))?;
            if !branch_status.status.success() {
                eprintln!(
                    "git branch failed: {}",
                    String::from_utf8_lossy(&branch_status.stderr)
                );
                return Err(());
            }

            // Create worktree
            let worktree_path = format!(".worktrees/{}", info.branch_name);
            let wt_status = std::process::Command::new("git")
                .args(["worktree", "add", &worktree_path, &info.branch_name])
                .output()
                .map_err(|e| eprintln!("git worktree add failed: {e}"))?;
            if !wt_status.status.success() {
                eprintln!(
                    "git worktree add failed: {}",
                    String::from_utf8_lossy(&wt_status.stderr)
                );
                return Err(());
            }

            // Write epic branch state in the worktree
            let branch_state = commands::epic_branch_state(&info.milestone_id, &info.epic_id);
            let wt_store =
                claude_disciplined::store::FsStore::new(std::path::PathBuf::from(&worktree_path));
            let yaml = serde_yaml::to_string(&branch_state).expect("yaml serialization");
            let wt_state_dir = format!("{worktree_path}/.workflow");
            std::fs::create_dir_all(&wt_state_dir).map_err(|e| eprintln!("{e}"))?;
            std::fs::write(format!("{wt_state_dir}/state.yml"), &yaml)
                .map_err(|e| eprintln!("{e}"))?;

            // Copy counters to worktree
            let counters = store.read_counters().map_err(|e| eprintln!("{e}"))?;
            let _ = wt_store.write_counters(&counters);

            let json = serde_json::to_string_pretty(&info).expect("serialization");
            println!("{json}");
            Ok(())
        }
        "complete" => {
            let id = require_arg(args, 1, "epic-id")?;
            ok_or_err(commands::epic_complete(store, id))
        }
        "list" => {
            let milestone_id = find_flag(args, "--milestone");
            let status_filter = find_flag(args, "--status");
            print_json(commands::epic_list(store, milestone_id, status_filter))
        }
        _ => {
            eprintln!("unknown epic subcommand: {subcmd}");
            Err(())
        }
    }
}

fn run_action(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = require_arg(args, 0, "subcommand")?;
    match subcmd {
        "start" => {
            let id = require_arg(args, 1, "action-id")?;
            ok_or_err(commands::action_start(store, id))
        }
        "complete" => {
            let id = require_arg(args, 1, "action-id")?;
            let summary = require_flag(args, "--summary")?;
            ok_or_err(commands::action_complete(store, id, summary))
        }
        "discard" => {
            let id = require_arg(args, 1, "action-id")?;
            let reason = require_flag(args, "--reason")?;
            ok_or_err(commands::action_discard(store, id, reason))
        }
        "read" => {
            let id = require_arg(args, 1, "action-id")?;
            print_json(commands::action_read(store, id))
        }
        "list" => {
            let type_filter = find_flag(args, "--type");
            let status_filter = find_flag(args, "--status");
            let assignee_filter = find_flag(args, "--assignee");
            print_json(commands::query_actions(
                store,
                type_filter,
                status_filter,
                assignee_filter,
            ))
        }
        _ => {
            eprintln!("unknown action subcommand: {subcmd}");
            Err(())
        }
    }
}

fn run_task(store: &FsStore, args: &[String]) -> Result<(), ()> {
    let subcmd = require_arg(args, 0, "subcommand")?;
    match subcmd {
        "start" => {
            let id = require_arg(args, 1, "task-id")?;
            ok_or_err(commands::task_start(store, id))
        }
        "complete" => {
            let id = require_arg(args, 1, "task-id")?;
            ok_or_err(commands::task_complete(store, id))
        }
        "block" => {
            let id = require_arg(args, 1, "task-id")?;
            let reason = require_flag(args, "--reason")?;
            ok_or_err(commands::task_block(store, id, reason))
        }
        "unblock" => {
            let id = require_arg(args, 1, "task-id")?;
            ok_or_err(commands::task_unblock(store, id))
        }
        "list" => {
            let epic_id = find_flag(args, "--epic");
            let story_id = find_flag(args, "--story");
            let status_filter = find_flag(args, "--status");
            print_json(commands::task_list(store, epic_id, story_id, status_filter))
        }
        _ => {
            eprintln!("unknown task subcommand: {subcmd}");
            Err(())
        }
    }
}

fn parse_position_kind(s: &str) -> Result<claude_disciplined::decision::PositionKind, ()> {
    match s {
        "agree" => Ok(claude_disciplined::decision::PositionKind::Agree),
        "disagree" => Ok(claude_disciplined::decision::PositionKind::Disagree),
        "disagree-and-commit" => Ok(claude_disciplined::decision::PositionKind::DisagreeAndCommit),
        _ => {
            eprintln!("invalid position: {s} (expected: agree, disagree, disagree-and-commit)");
            Err(())
        }
    }
}

fn find_flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            return args.get(i + 1).map(String::as_str);
        }
    }
    None
}

fn print_ok<T: std::fmt::Display>(result: commands::CmdResult<T>) -> Result<(), ()> {
    match result {
        Ok(val) => {
            println!("{val}");
            Ok(())
        }
        Err(e) => {
            eprintln!("{e}");
            Err(())
        }
    }
}

fn print_json<T: serde::Serialize>(result: commands::CmdResult<T>) -> Result<(), ()> {
    match result {
        Ok(val) => {
            let json = serde_json::to_string_pretty(&val).expect("serialization");
            println!("{json}");
            Ok(())
        }
        Err(e) => {
            eprintln!("{e}");
            Err(())
        }
    }
}

fn ok_or_err(result: commands::CmdResult<()>) -> Result<(), ()> {
    result.map_err(|e| eprintln!("{e}"))
}

fn require_arg<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str, ()> {
    args.get(index).map(String::as_str).ok_or_else(|| {
        eprintln!("missing argument: <{name}>");
    })
}

fn require_flag<'a>(args: &'a [String], flag: &str) -> Result<&'a str, ()> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            return args.get(i + 1).map(String::as_str).ok_or_else(|| {
                eprintln!("missing value for flag {flag}");
            });
        }
    }
    eprintln!("missing required flag: {flag}");
    Err(())
}
