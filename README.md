# claude-disciplined

> **Alpha software.** This project is still in alpha development. APIs, commands, and on-disk formats may change without notice. **Try at your own risk.**

A workflow enforcement system for [Claude Code](https://claude.com/claude-code) that ensures disciplined, traceable, goal-aligned software development.

## Overview

claude-disciplined provides two binaries:

- **`workflow`** — companion CLI for managing workflow state, artifacts, and queries
- **`workflow-hook`** — Claude Code hook binary enforcing workflow constraints on agent tool calls

The system enforces a structured development workflow with full traceability from individual tasks up to product goals, role-based access control for agents, consensus-driven decision making, and file-based state management.

See [SPEC.md](SPEC.md) for the full specification.

## CLI Commands

```bash
workflow init                              # initialize a new project
workflow status                            # show current workflow state
workflow advance                           # move to next step (checks gates)
workflow request-approval <step>           # record stakeholder approval
workflow escalate <target-step> --reason "..."  # backward transition (logged as decision)
workflow create requirement <slug>         # create a functional requirement
workflow create milestone <slug>           # create a milestone
workflow create epic <slug>                # create an epic (resolves milestone from state)
workflow create story <slug>               # create a story (resolves epic from state)
workflow create task <slug> --story <id>   # create a task in a story
workflow create flow <slug> --story <id> --type <type>  # create a user flow
workflow propose nfr <slug>               # propose a non-functional requirement (draft)
workflow read <type> [<id>]               # read artifact content
echo "body" | workflow write <type> [<id>]  # write artifact body (preserves front matter)
workflow meeting start <topic>            # start a meeting in the current step
workflow meeting end                      # end the active meeting
workflow meeting read                     # read active meeting notes
workflow meeting contribute <message>     # append to meeting notes
workflow meeting list                     # list meetings in current step
workflow meeting propose-decision <summary>  # propose a decision
workflow meeting position <id> <agree|disagree|disagree-and-commit> --role <role> [--reason "..."]
workflow meeting record-decision <id>     # finalize agreed decision
workflow meeting resolve-decision <id> --role <role> --justification "..."
workflow meeting drop-decision <id> --reason "..."
workflow meeting supersede-decision <id> --by <new-id>
workflow meeting decision-status <id>     # check decision status
workflow meeting list-decisions           # list all decisions
workflow meeting address-disagreement <id> --role <role>
workflow meeting add-action <desc> --type <type> --assignee <role> [--immediate]
workflow meeting start-action <id>
workflow meeting complete-action <id> --summary "..."
workflow meeting discard-action <id> --reason "..."
workflow meeting list-actions
workflow task start <id>               # mark task as in-progress
workflow task complete <id>            # mark task as completed
workflow task block <id> --reason "..."  # block task with reason
workflow task unblock <id>             # unblock task (back to pending)
workflow task list [--epic <id>] [--story <id>] [--status <s>]  # list tasks
workflow epic start <id>               # create branch/worktree, activate epic
workflow epic complete <id>            # finalize after retrospective
workflow epic list [--milestone <id>] [--status <s>]  # list epics
```

## Building

```bash
cargo build --release
```

## Testing

```bash
# All tests
cargo test

# Clippy
cargo clippy --all-targets -- -D warnings

# Format (requires nightly)
rustup run nightly cargo fmt --check

# Mutation testing
cargo mutants --timeout 30
```

## License

AGPL-3.0 — see [LICENSE](LICENSE).
