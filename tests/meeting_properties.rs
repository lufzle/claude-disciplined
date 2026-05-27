use claude_disciplined::{commands, meeting, store::MemStore};
use proptest::prelude::*;

fn arb_topic() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9\\-]{0,14}"
}

proptest! {
    #[test]
    fn create_meeting_folder_name_has_sequence(seq in 1u32..=999, topic in arb_topic()) {
        let store = MemStore::new();
        let name = meeting::create_meeting_folder(&store, "test/meetings", seq, &topic).unwrap();
        let expected_prefix = format!("{seq:03}-");
        prop_assert!(name.starts_with(&expected_prefix));
        prop_assert!(name.ends_with(&topic));
    }

    #[test]
    fn meeting_start_end_cycle_clears_active(topic in arb_topic()) {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::meeting_start(&store, &topic).unwrap();
        let state = commands::status(&store).unwrap();
        prop_assert!(state.active_meeting.is_some());
        commands::meeting_end(&store).unwrap();
        let state = commands::status(&store).unwrap();
        prop_assert!(state.active_meeting.is_none());
    }

    #[test]
    fn meeting_contribute_accumulates(topic in arb_topic(), msg1 in "[a-z ]{1,20}", msg2 in "[a-z ]{1,20}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::meeting_start(&store, &topic).unwrap();
        commands::meeting_contribute(&store, &msg1).unwrap();
        commands::meeting_contribute(&store, &msg2).unwrap();
        let notes = commands::meeting_read(&store).unwrap();
        prop_assert!(notes.contains(&msg1));
        prop_assert!(notes.contains(&msg2));
    }

    #[test]
    fn meeting_start_idempotency_guard(topic in arb_topic()) {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::meeting_start(&store, &topic).unwrap();
        let result = commands::meeting_start(&store, "another");
        prop_assert!(result.is_err());
    }
}
