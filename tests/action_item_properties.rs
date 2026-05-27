use claude_disciplined::action_item::{ActionItem, ActionStatus, ActionTiming, ActionType};
use proptest::prelude::*;

fn arb_action_type() -> impl Strategy<Value = ActionType> {
    prop_oneof![
        Just(ActionType::Research),
        Just(ActionType::ProofOfConcept),
        Just(ActionType::StakeholderQuestion),
        Just(ActionType::Draft),
        Just(ActionType::Review),
    ]
}

fn arb_timing() -> impl Strategy<Value = ActionTiming> {
    prop_oneof![Just(ActionTiming::Immediate), Just(ActionTiming::Deferred),]
}

proptest! {
    #[test]
    fn new_is_always_pending(
        action_type in arb_action_type(),
        timing in arb_timing(),
        desc in "[a-z ]{1,20}",
        assignee in "[a-z\\-]{1,15}"
    ) {
        let item = ActionItem::new("AI-001".to_owned(), action_type, desc, assignee, timing);
        prop_assert_eq!(item.status, ActionStatus::Pending);
        prop_assert!(!item.is_done());
    }

    #[test]
    fn complete_is_always_done(desc in "[a-z ]{1,20}", summary in "[a-z ]{1,20}") {
        let mut item = ActionItem::new(
            "AI-001".to_owned(),
            ActionType::Research,
            desc,
            "analyst".to_owned(),
            ActionTiming::Deferred,
        );
        item.complete(&summary);
        prop_assert!(item.is_done());
        prop_assert_eq!(item.summary.as_deref(), Some(summary.as_str()));
    }

    #[test]
    fn discard_is_always_done(desc in "[a-z ]{1,20}", reason in "[a-z ]{1,20}") {
        let mut item = ActionItem::new(
            "AI-001".to_owned(),
            ActionType::Research,
            desc,
            "analyst".to_owned(),
            ActionTiming::Deferred,
        );
        item.discard(&reason);
        prop_assert!(item.is_done());
        prop_assert_eq!(item.discarded_reason.as_deref(), Some(reason.as_str()));
    }

    #[test]
    fn ndjson_roundtrips(desc in "[a-z ]{1,20}", assignee in "[a-z\\-]{1,15}") {
        let item = ActionItem::new(
            "AI-001".to_owned(),
            ActionType::Research,
            desc,
            assignee,
            ActionTiming::Deferred,
        );
        let line = claude_disciplined::action_item::to_ndjson_line(&item);
        let parsed = claude_disciplined::action_item::from_ndjson_line(&line).unwrap();
        prop_assert_eq!(item, parsed);
    }
}
