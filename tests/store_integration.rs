use claude_disciplined::{
    counters::Counters,
    id::Prefix,
    state::{Phase, State},
    store::{MemStore, Store},
};

// -- Initialize --

#[test]
fn initialize_sets_state_and_counters() {
    let store = MemStore::new();
    let state = State::init();
    let counters = Counters::new();
    store.initialize(&state, &counters).unwrap();

    assert!(store.is_initialized());
    assert_eq!(store.read_state().unwrap(), state);
    assert_eq!(store.read_counters().unwrap(), counters);
}

#[test]
fn not_initialized_before_init() {
    let store = MemStore::new();
    assert!(!store.is_initialized());
}

#[test]
fn read_state_before_init_returns_error() {
    let store = MemStore::new();
    assert!(store.read_state().is_err());
}

// -- State read/write --

#[test]
fn write_then_read_state_roundtrips() {
    let store = MemStore::new();
    let state = State {
        phase: Phase::Epic,
        step: Some("implementation".to_owned()),
        current_milestone: Some("M-001".to_owned()),
        epic: Some("E-002".to_owned()),
        active_meeting: None,
    };
    store.write_state(&state).unwrap();
    assert_eq!(store.read_state().unwrap(), state);
}

#[test]
fn write_state_overwrites_previous() {
    let store = MemStore::new();
    let first = State::init();
    store.write_state(&first).unwrap();

    let second = State {
        phase: Phase::Setup,
        step: Some("requirements".to_owned()),
        current_milestone: None,
        epic: None,
        active_meeting: None,
    };
    store.write_state(&second).unwrap();
    assert_eq!(store.read_state().unwrap(), second);
}

// -- Counters read/write --

#[test]
fn write_then_read_counters_roundtrips() {
    let store = MemStore::new();
    let mut counters = Counters::new();
    counters.next(Prefix::Req).unwrap();
    counters.next(Prefix::Req).unwrap();
    counters.next(Prefix::T).unwrap();

    store.write_counters(&counters).unwrap();
    let loaded = store.read_counters().unwrap();
    assert_eq!(loaded.count(Prefix::Req), 2);
    assert_eq!(loaded.count(Prefix::T), 1);
}

// -- Approvals --

#[test]
fn approvals_start_empty() {
    let store = MemStore::new();
    assert!(store.read_approvals().unwrap().is_empty());
}

#[test]
fn append_approval_accumulates() {
    let store = MemStore::new();
    store
        .append_approval(r#"{"step":"product-brief","approved_by":"stakeholder"}"#)
        .unwrap();
    store
        .append_approval(r#"{"step":"requirements","approved_by":"stakeholder"}"#)
        .unwrap();

    let approvals = store.read_approvals().unwrap();
    assert_eq!(approvals.len(), 2);
    assert!(approvals[0].contains("product-brief"));
    assert!(approvals[1].contains("requirements"));
}

// -- FsStore with temp directory --

#[test]
fn fs_store_initialize_and_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());

    let state = State::init();
    let counters = Counters::new();

    assert!(!store.is_initialized());
    store.initialize(&state, &counters).unwrap();
    assert!(store.is_initialized());

    assert_eq!(store.read_state().unwrap(), state);
    assert_eq!(store.read_counters().unwrap(), counters);
}

#[test]
fn fs_store_approval_append_and_read() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    store.initialize(&State::init(), &Counters::new()).unwrap();

    store
        .append_approval(r#"{"step":"product-brief"}"#)
        .unwrap();
    store.append_approval(r#"{"step":"requirements"}"#).unwrap();

    let approvals = store.read_approvals().unwrap();
    assert_eq!(approvals.len(), 2);
}

// -- MemStore file operations --

#[test]
fn memstore_write_then_read_file() {
    let store = MemStore::new();
    store.write_file("test/file.md", "hello").unwrap();
    assert_eq!(store.read_file("test/file.md").unwrap(), "hello");
}

#[test]
fn memstore_read_nonexistent_file_errors() {
    let store = MemStore::new();
    assert!(store.read_file("nope.md").is_err());
}

#[test]
fn memstore_file_exists_reflects_writes() {
    let store = MemStore::new();
    assert!(!store.file_exists("x.md"));
    store.write_file("x.md", "").unwrap();
    assert!(store.file_exists("x.md"));
}

#[test]
fn memstore_ensure_dir_is_noop() {
    let store = MemStore::new();
    assert!(store.ensure_dir("some/deep/path").is_ok());
}

// -- FsStore file operations --

#[test]
fn fs_store_write_then_read_file() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    store.write_file("subdir/test.md", "content").unwrap();
    assert_eq!(store.read_file("subdir/test.md").unwrap(), "content");
}

#[test]
fn fs_store_write_file_creates_parent_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    store.write_file("a/b/c/file.md", "deep").unwrap();
    assert!(tmp.path().join("a/b/c/file.md").exists());
}

#[test]
fn fs_store_file_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    assert!(!store.file_exists("nope.md"));
    store.write_file("yes.md", "").unwrap();
    assert!(store.file_exists("yes.md"));
}

#[test]
fn fs_store_ensure_dir_creates_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    store.ensure_dir("deep/nested/dir").unwrap();
    assert!(tmp.path().join("deep/nested/dir").is_dir());
}

// -- MemStore find_file --

#[test]
fn memstore_find_file_by_prefix() {
    let store = MemStore::new();
    store
        .write_file("requirements/REQ-001-build-apps.md", "content")
        .unwrap();
    let found = store.find_file("requirements/REQ-001-*").unwrap();
    assert_eq!(found, "requirements/REQ-001-build-apps.md");
}

#[test]
fn memstore_find_file_returns_none_for_missing() {
    let store = MemStore::new();
    assert!(store.find_file("requirements/REQ-999-*").is_none());
}

// -- MemStore find_dir --

#[test]
fn memstore_find_dir_by_prefix() {
    let store = MemStore::new();
    store
        .write_file("milestones/M-001-mvp/milestone.md", "content")
        .unwrap();
    let found = store.find_dir("milestones/M-001-*").unwrap();
    assert_eq!(found, "milestones/M-001-mvp");
}

// -- MemStore find_file_deep --

#[test]
fn memstore_find_file_deep_finds_nested() {
    let store = MemStore::new();
    store
        .write_file("epics/E-001/stories/S-001/tasks/T-001-ui.md", "content")
        .unwrap();
    let found = store.find_file_deep("epics/E-001", "T-001-").unwrap();
    assert_eq!(found, "epics/E-001/stories/S-001/tasks/T-001-ui.md");
}

#[test]
fn memstore_find_file_deep_returns_none_for_missing() {
    let store = MemStore::new();
    assert!(store.find_file_deep("epics/E-001", "T-999-").is_none());
}

// -- FsStore find_file --

#[test]
fn fs_store_find_file_by_prefix() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    store
        .write_file("requirements/REQ-001-apps.md", "x")
        .unwrap();
    let found = store.find_file("requirements/REQ-001-*").unwrap();
    assert_eq!(found, "requirements/REQ-001-apps.md");
}

// -- FsStore find_file_deep --

#[test]
fn fs_store_find_file_deep_finds_nested() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    store
        .write_file("epics/E-001/stories/S-001/tasks/T-001-ui.md", "x")
        .unwrap();
    let found = store.find_file_deep("epics/E-001", "T-001-").unwrap();
    assert_eq!(found, "epics/E-001/stories/S-001/tasks/T-001-ui.md");
}
