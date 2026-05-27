use claude_disciplined::{
    gates,
    state::SetupStep,
    store::{MemStore, Store},
};
use proptest::prelude::*;

fn arb_setup_step() -> impl Strategy<Value = SetupStep> {
    prop_oneof![
        Just(SetupStep::ProductBrief),
        Just(SetupStep::Requirements),
        Just(SetupStep::UxFoundations),
        Just(SetupStep::Roadmap),
        Just(SetupStep::Architecture),
        Just(SetupStep::TechStack),
        Just(SetupStep::EpicPlanning),
    ]
}

proptest! {
    #[test]
    fn unapproved_step_always_has_violations(step in arb_setup_step()) {
        let store = MemStore::new();
        let violations = gates::check_setup_advance(step, &store);
        prop_assert!(!violations.is_empty());
    }

    #[test]
    fn approved_step_has_no_violations(step in arb_setup_step()) {
        let store = MemStore::new();
        store.append_approval(&format!(
            "{{\"step\":\"{}\",\"approved_by\":\"stakeholder\"}}",
            step.as_str()
        )).unwrap();
        let violations = gates::check_setup_advance(step, &store);
        prop_assert!(violations.is_empty(), "violations for {step:?}: {violations:?}");
    }

    #[test]
    fn wrong_approval_doesnt_satisfy_gate(step in arb_setup_step()) {
        let store = MemStore::new();
        store.append_approval(r#"{"step":"nonexistent","approved_by":"stakeholder"}"#).unwrap();
        let violations = gates::check_setup_advance(step, &store);
        prop_assert!(!violations.is_empty());
    }

    #[test]
    fn all_violations_have_rule_and_message(step in arb_setup_step()) {
        let store = MemStore::new();
        let violations = gates::check_setup_advance(step, &store);
        for v in &violations {
            prop_assert!(!v.rule.is_empty());
            prop_assert!(!v.message.is_empty());
        }
    }
}
