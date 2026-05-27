use claude_disciplined::{
    counters::Counters,
    id::Prefix,
    state::{Phase, State},
    store::{MemStore, Store},
};
use proptest::prelude::*;

fn arb_phase() -> impl Strategy<Value = Phase> {
    prop_oneof![
        Just(Phase::Setup),
        Just(Phase::Execution),
        Just(Phase::Epic),
    ]
}

fn arb_option_string() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-z\\-]{1,20}".prop_map(Some),]
}

fn arb_state() -> impl Strategy<Value = State> {
    (
        arb_phase(),
        arb_option_string(),
        arb_option_string(),
        arb_option_string(),
        arb_option_string(),
    )
        .prop_map(|(phase, step, milestone, epic, meeting)| State {
            phase,
            step,
            current_milestone: milestone,
            epic,
            active_meeting: meeting,
        })
}

fn arb_prefix() -> impl Strategy<Value = Prefix> {
    prop_oneof![
        Just(Prefix::Req),
        Just(Prefix::Nfr),
        Just(Prefix::M),
        Just(Prefix::E),
        Just(Prefix::S),
        Just(Prefix::T),
        Just(Prefix::D),
        Just(Prefix::Ai),
        Just(Prefix::F),
        Just(Prefix::Ur),
    ]
}

proptest! {
    #[test]
    fn state_write_read_roundtrip(state in arb_state()) {
        let store = MemStore::new();
        store.write_state(&state).unwrap();
        let loaded = store.read_state().unwrap();
        prop_assert_eq!(state, loaded);
    }

    #[test]
    fn counters_write_read_roundtrip(prefix in arb_prefix(), count in 1u16..=50) {
        let store = MemStore::new();
        let mut counters = Counters::new();
        for _ in 0..count {
            counters.next(prefix).unwrap();
        }
        store.write_counters(&counters).unwrap();
        let loaded = store.read_counters().unwrap();
        prop_assert_eq!(loaded.count(prefix), count);
    }

    #[test]
    fn approvals_accumulate_in_order(entries in prop::collection::vec("[a-z]{1,10}", 1..=10)) {
        let store = MemStore::new();
        for entry in &entries {
            store.append_approval(entry).unwrap();
        }
        let loaded = store.read_approvals().unwrap();
        prop_assert_eq!(entries, loaded);
    }

    #[test]
    fn initialize_then_read_matches(state in arb_state()) {
        let store = MemStore::new();
        let counters = Counters::new();
        store.initialize(&state, &counters).unwrap();
        prop_assert!(store.is_initialized());
        prop_assert_eq!(store.read_state().unwrap(), state);
    }

    #[test]
    fn file_write_read_roundtrip(path in "[a-z]{1,5}/[a-z]{1,10}\\.md", content in "\\PC{0,100}") {
        let store = MemStore::new();
        store.write_file(&path, &content).unwrap();
        let loaded = store.read_file(&path).unwrap();
        prop_assert_eq!(content, loaded);
    }

    #[test]
    fn file_exists_after_write(path in "[a-z]{1,5}/[a-z]{1,10}\\.md") {
        let store = MemStore::new();
        prop_assert!(!store.file_exists(&path));
        store.write_file(&path, "x").unwrap();
        prop_assert!(store.file_exists(&path));
    }
}
