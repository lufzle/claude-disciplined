# claude-disciplined

[![CI](https://github.com/lufzle/claude-disciplined/actions/workflows/ci.yml/badge.svg)](https://github.com/lufzle/claude-disciplined/actions/workflows/ci.yml)
[![License: AGPL-3.0-only](https://img.shields.io/badge/license-AGPL--3.0--only-blue.svg)](LICENSE)

> **Alpha software.** This project is still in alpha development. Commands, APIs, and on-disk formats may change without notice. **Try at your own risk.**

Workflow enforcement for [Claude Code](https://claude.com/claude-code): structured phases, role-gated agents, and file-based traceability from tasks up to product goals.

Two binaries:

| Binary | Role |
| --- | --- |
| **`workflow`** | Companion CLI for state, artifacts, meetings, and queries |
| **`workflow-hook`** | Claude Code `PreToolUse` hook that enforces those rules |

The full contract is in [SPEC.md](SPEC.md). Architecture notes for agents working in this repo are in [CLAUDE.md](CLAUDE.md).

## Status

Alpha. Not production-ready. Not published on crates.io. Build from source.

## Install

Requires [Rust](https://rustup.rs/) 1.85 or newer.

```bash
git clone https://github.com/lufzle/claude-disciplined.git
cd claude-disciplined
cargo install --path . --locked
```

That installs `workflow` and `workflow-hook` into `~/.cargo/bin`.

### Claude Code hook

Point a `PreToolUse` hook at `workflow-hook` (stdin JSON in, allow on exit 0, deny on exit 2). See [SPEC.md](SPEC.md#hook-binary-contract) for the I/O contract.

## Quick start

```bash
workflow init                 # create .workflow/ in the current project
workflow status               # current phase and step
workflow request-approval product-brief
workflow advance              # gates must pass
```

Artifacts live as Markdown with YAML front matter. Prefer the CLI over editing `.workflow/` by hand — the hook denies direct reads/writes to workflow-controlled paths.

## Usage

### Workflow lifecycle

```bash
workflow init
workflow status
workflow advance
workflow request-approval <step>
workflow escalate <target-step> --reason "..."
```

### Artifacts

```bash
workflow create requirement <slug>
workflow create milestone <slug>
workflow create epic <slug>
workflow create story <slug>
workflow create task <slug> --story <id>
workflow create flow <slug> --story <id> --type <type>
workflow propose nfr <slug>
workflow read <type> [<id>]
echo "body" | workflow write <type> [<id>]
workflow update <id> --status <status>
```

### Meetings, decisions, actions

```bash
workflow meeting start <topic>
workflow meeting end
workflow meeting read
workflow meeting contribute <message>
workflow meeting list
workflow meeting propose-decision <summary>
workflow meeting position <id> <agree|disagree|disagree-and-commit> --role <role> [--reason "..."]
workflow meeting record-decision <id>
workflow meeting resolve-decision <id> --role <role> --justification "..."
workflow meeting drop-decision <id> --reason "..."
workflow meeting supersede-decision <id> --by <new-id>
workflow meeting decision-status <id>
workflow meeting list-decisions
workflow meeting address-disagreement <id> --role <role>
workflow meeting add-action <desc> --type <type> --assignee <role> [--immediate]
workflow meeting start-action <id>
workflow meeting complete-action <id> --summary "..."
workflow meeting discard-action <id> --reason "..."
workflow meeting list-actions
```

### Tasks and epics

```bash
workflow task start <id>
workflow task complete <id>
workflow task block <id> --reason "..."
workflow task unblock <id>
workflow task list [--epic <id>] [--story <id>] [--status <s>]
workflow epic start <id>
workflow epic complete <id>
workflow epic list [--milestone <id>] [--status <s>]
```

### Query, validate, verify

```bash
workflow query requirements|nfrs|milestones|epics|stories|tasks|flows|decisions
workflow query trace|impact|rationale|coverage|goal ...
workflow validate links|coverage|structure|coherence|gates
workflow verify story <id>
workflow verify epic <id>
```

## Development

```bash
cargo build --release
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
rustup run nightly cargo fmt --all -- --check   # rustfmt.toml uses nightly options
cargo deny check
```

CI runs the same checks on every push to `main` and on pull requests. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[GNU Affero General Public License v3.0 only](LICENSE) (`AGPL-3.0-only`).

If you run a modified version as a network service, you must offer the corresponding source to its users (AGPL §13).
