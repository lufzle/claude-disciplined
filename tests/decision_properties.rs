use claude_disciplined::decision::{Decision, DecisionStatus, PositionKind};
use proptest::prelude::*;

fn arb_role() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("analyst".to_owned()),
        Just("ux-designer".to_owned()),
        Just("staff-engineer".to_owned()),
        Just("product-engineer".to_owned()),
        Just("platform-engineer".to_owned()),
    ]
}

proptest! {
    #[test]
    fn new_is_always_proposed(summary in "[a-z ]{1,30}") {
        let d = Decision::new("D-001".to_owned(), summary);
        prop_assert_eq!(d.status, DecisionStatus::Proposed);
        prop_assert_eq!(d.iteration, 0);
        prop_assert!(d.positions.is_empty());
    }

    #[test]
    fn set_position_always_increments_iteration(role in arb_role()) {
        let mut d = Decision::new("D-001".to_owned(), "test".to_owned());
        let before = d.iteration;
        d.set_position(&role, PositionKind::Agree, None);
        prop_assert_eq!(d.iteration, before + 1);
    }

    #[test]
    fn all_agree_always_recordable(count in 1usize..=5) {
        let mut d = Decision::new("D-001".to_owned(), "test".to_owned());
        for i in 0..count {
            d.set_position(&format!("role-{i}"), PositionKind::Agree, None);
        }
        prop_assert!(d.can_record());
    }

    #[test]
    fn any_disagree_blocks_record(agree_count in 1usize..=3) {
        let mut d = Decision::new("D-001".to_owned(), "test".to_owned());
        for i in 0..agree_count {
            d.set_position(&format!("role-{i}"), PositionKind::Agree, None);
        }
        d.set_position("dissenter", PositionKind::Disagree, Some("no".to_owned()));
        prop_assert!(!d.can_record());
    }

    #[test]
    fn record_all_agree_produces_agreed(count in 1usize..=5) {
        let mut d = Decision::new("D-001".to_owned(), "test".to_owned());
        for i in 0..count {
            d.set_position(&format!("role-{i}"), PositionKind::Agree, None);
        }
        d.record().unwrap();
        prop_assert_eq!(d.status, DecisionStatus::Agreed);
    }

    #[test]
    fn record_with_commit_produces_reservations(agree_count in 1usize..=3) {
        let mut d = Decision::new("D-001".to_owned(), "test".to_owned());
        for i in 0..agree_count {
            d.set_position(&format!("role-{i}"), PositionKind::Agree, None);
        }
        d.set_position("reluctant", PositionKind::DisagreeAndCommit, Some("ok".to_owned()));
        d.record().unwrap();
        prop_assert_eq!(d.status, DecisionStatus::AgreedWithReservations);
    }

    #[test]
    fn ndjson_roundtrips(summary in "[a-z ]{1,20}", role in arb_role()) {
        let mut d = Decision::new("D-001".to_owned(), summary);
        d.set_position(&role, PositionKind::Agree, None);
        let line = claude_disciplined::decision::to_ndjson_line(&d);
        let parsed = claude_disciplined::decision::from_ndjson_line(&line).unwrap();
        prop_assert_eq!(d, parsed);
    }

    #[test]
    fn terminal_statuses_stay_terminal(
        status in prop_oneof![
            Just(DecisionStatus::Agreed),
            Just(DecisionStatus::AgreedWithReservations),
            Just(DecisionStatus::ResolvedByOwner),
            Just(DecisionStatus::Dropped),
            Just(DecisionStatus::Superseded),
        ]
    ) {
        prop_assert!(status.is_terminal());
    }

    #[test]
    fn non_terminal_statuses(
        status in prop_oneof![
            Just(DecisionStatus::Proposed),
            Just(DecisionStatus::Discussing),
        ]
    ) {
        prop_assert!(!status.is_terminal());
    }
}
