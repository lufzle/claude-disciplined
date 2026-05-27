use claude_disciplined::state::{EpicStep, SetupStep};
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

fn arb_epic_step() -> impl Strategy<Value = EpicStep> {
    prop_oneof![
        Just(EpicStep::Analysis),
        Just(EpicStep::UxDesign),
        Just(EpicStep::TechnicalDesign),
        Just(EpicStep::Planning),
        Just(EpicStep::Implementation),
        Just(EpicStep::Verification),
        Just(EpicStep::Review),
        Just(EpicStep::Release),
        Just(EpicStep::Retrospective),
    ]
}

proptest! {
    #[test]
    fn setup_step_as_str_serde_consistency(step in arb_setup_step()) {
        let yaml = serde_yaml::to_string(&step).unwrap();
        prop_assert_eq!(step.as_str(), yaml.trim());
    }

    #[test]
    fn epic_step_as_str_serde_consistency(step in arb_epic_step()) {
        let yaml = serde_yaml::to_string(&step).unwrap();
        prop_assert_eq!(step.as_str(), yaml.trim());
    }

    #[test]
    fn setup_step_next_is_forward_only(step in arb_setup_step()) {
        if let Some(next) = step.next() {
            let idx = SetupStep::ALL.iter().position(|&s| s == step).unwrap();
            let next_idx = SetupStep::ALL.iter().position(|&s| s == next).unwrap();
            prop_assert_eq!(next_idx, idx + 1);
        }
    }

    #[test]
    fn epic_step_next_is_forward_only(step in arb_epic_step()) {
        if let Some(next) = step.next() {
            let idx = EpicStep::ALL.iter().position(|&s| s == step).unwrap();
            let next_idx = EpicStep::ALL.iter().position(|&s| s == next).unwrap();
            prop_assert_eq!(next_idx, idx + 1);
        }
    }

    #[test]
    fn escalation_targets_are_always_before_current(step in arb_epic_step()) {
        let current_idx = EpicStep::ALL.iter().position(|&s| s == step).unwrap();
        for &target in step.escalation_targets() {
            let target_idx = EpicStep::ALL.iter().position(|&s| s == target).unwrap();
            prop_assert!(target_idx < current_idx,
                "escalation target {:?} (idx {}) is not before {:?} (idx {})",
                target, target_idx, step, current_idx);
        }
    }
}
