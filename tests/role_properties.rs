use claude_disciplined::{
    role::{self, Permission, Role},
    state::{EpicStep, SetupStep},
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

fn arb_role() -> impl Strategy<Value = Role> {
    prop_oneof![
        Just(Role::Stakeholder),
        Just(Role::Analyst),
        Just(Role::UxDesigner),
        Just(Role::StaffEngineer),
        Just(Role::ProductEngineer),
        Just(Role::PlatformEngineer),
    ]
}

const ALL_ROLES: [Role; 6] = [
    Role::Stakeholder,
    Role::Analyst,
    Role::UxDesigner,
    Role::StaffEngineer,
    Role::ProductEngineer,
    Role::PlatformEngineer,
];

proptest! {
    #[test]
    fn every_setup_step_has_at_least_one_driver(step in arb_setup_step()) {
        let drivers: Vec<_> = ALL_ROLES.iter()
            .filter(|&&r| role::setup_permission(step, r) == Some(Permission::Driver))
            .collect();
        prop_assert!(!drivers.is_empty(), "no driver for setup step {step:?}");
    }

    #[test]
    fn stakeholder_always_allowed_in_setup(step in arb_setup_step()) {
        prop_assert!(role::is_allowed_setup(step, Role::Stakeholder));
    }

    #[test]
    fn every_epic_step_has_at_least_one_allowed_role(step in arb_epic_step()) {
        let allowed: Vec<_> = ALL_ROLES.iter()
            .filter(|&&r| role::is_allowed_epic(step, r))
            .collect();
        prop_assert!(!allowed.is_empty(), "no roles allowed for epic step {step:?}");
    }

    #[test]
    fn driver_implies_allowed_setup(step in arb_setup_step(), role in arb_role()) {
        if role::is_driver_setup(step, role) {
            prop_assert!(role::is_allowed_setup(step, role));
        }
    }

    #[test]
    fn driver_implies_allowed_epic(step in arb_epic_step(), role in arb_role()) {
        if role::is_driver_epic(step, role) {
            prop_assert!(role::is_allowed_epic(step, role));
        }
    }

    #[test]
    fn all_roles_allowed_at_retrospective(role in arb_role()) {
        prop_assert!(role::is_allowed_epic(EpicStep::Retrospective, role));
    }
}
