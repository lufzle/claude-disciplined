# claude-disciplined

Workflow enforcement system for Claude Code. Two binaries: `workflow` (companion CLI) and `workflow-hook` (Claude Code hook).

## Architecture

- `src/lib.rs` -- crate root, module re-exports
- `src/id.rs` -- typed entity IDs (Prefix + 3-digit hex)
- `src/state.rs` -- workflow phases, steps, transitions
- `src/role.rs` -- role definitions and step-permission matrix
- `src/counters.rs` -- auto-increment ID allocator per prefix
- `src/store.rs` -- I/O abstraction (Store trait, FsStore, MemStore)
- `src/gates.rs` -- transition gate checks (approval validation)
- `src/artifacts.rs` -- artifact path generation and front matter templates
- `src/action_item.rs` -- action item data model, lifecycle, NDJSON serialization
- `src/decision.rs` -- decision data model, consensus logic, NDJSON serialization
- `src/meeting.rs` -- meeting folder creation, counting, step-to-dir mapping
- `src/commands/mod.rs` -- shared types (CmdError, CmdResult), re-exports
- `src/commands/workflow.rs` -- init, status, advance, request-approval
- `src/commands/create.rs` -- create requirement/milestone/epic/story/task/flow, propose nfr
- `src/commands/artifact.rs` -- read/write artifact content
- `src/commands/meeting.rs` -- meeting start/end/read/contribute/list
- `src/commands/decision.rs` -- decision propose/position/record/resolve/drop/supersede
- `src/commands/action.rs` -- action item add/start/complete/discard/list
- `src/commands/task.rs` -- task lifecycle start/complete/block/unblock/list
- `src/commands/epic.rs` -- epic lifecycle start/complete/list, branch/worktree info
- `src/commands/verify.rs` -- story/epic verification and verification reports
- `src/commands/escalate.rs` -- backward transitions with decision logging
- `src/commands/query.rs` -- listing queries (requirements/nfrs/milestones/epics/stories/tasks/flows/decisions) and traceability (trace/impact/rationale/coverage/goal)
- `src/commands/validate.rs` -- structural lint: link/coverage/structure/coherence/gate rules
- `src/commands/update.rs` -- generic artifact status update by ID
- `src/commands/unresolved_cmd.rs` -- unresolved disagreements: add/list/review/history
- `src/unresolved.rs` -- unresolved disagreement data model, NDJSON serialization
- `src/ndjson.rs` -- generic NDJSON load/save for Store-backed files
- `src/bin/workflow.rs` -- companion CLI binary (thin dispatch layer)
- `src/resolve.rs` -- context resolution (milestone/epic dir from state)
- `src/time.rs` -- date/timestamp generation (no external deps)
- `src/hook.rs` -- hook enforcement logic (file/bash/role/driver rules)
- `src/bin/workflow_hook.rs` -- Claude Code hook binary (reads stdin JSON, state, evaluates rules)
- `tests/` -- integration, property-based, and mutation-targeted tests

## Conventions

- Rust 2024 edition, MSRV 1.85
- Lint config in `Cargo.toml [lints]`, not in source attributes
- Format with nightly rustfmt (`rustup run nightly cargo fmt`)
- Clippy must pass with `-D warnings`
- Unit tests inline in `#[cfg(test)] mod tests`
- Integration tests in `tests/`
- Property tests use `proptest!` macro
- Mutation-target tests document known equivalent mutants
- `test-support` feature flag enables `MemStore` for integration tests

## Testing requirements

- Every module must have unit tests
- Narrow integration tests with mocked I/O (MemStore)
- Property-based tests for invariants (proptest)
- Mutation-targeted boundary tests (deferred to final pass)

## Spec

Full specification in `SPEC.md`.
