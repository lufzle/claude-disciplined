use serde::{Deserialize, Serialize};

use crate::state::{EpicStep, SetupStep};

/// Agent roles in the workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Stakeholder,
    Analyst,
    UxDesigner,
    StaffEngineer,
    ProductEngineer,
    PlatformEngineer,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stakeholder => "stakeholder",
            Self::Analyst => "analyst",
            Self::UxDesigner => "ux-designer",
            Self::StaffEngineer => "staff-engineer",
            Self::ProductEngineer => "product-engineer",
            Self::PlatformEngineer => "platform-engineer",
        }
    }
}

/// The permission level a role has at a given step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Primary driver of the step — can start/end meetings, advance state.
    Driver,
    /// Involved in the step — can participate but not drive.
    Involved,
    /// Approver — can approve/reject (stakeholder only).
    Approver,
}

/// Returns the permission for a role at a given setup step, or `None` if the
/// role is not allowed.
#[allow(clippy::match_same_arms)] // Each step's permissions are intentionally listed separately for readability
pub fn setup_permission(step: SetupStep, role: Role) -> Option<Permission> {
    use Permission::{Approver, Driver, Involved};
    use Role::{
        Analyst, PlatformEngineer, ProductEngineer, StaffEngineer, Stakeholder, UxDesigner,
    };

    match (step, role) {
        // Product Brief: analyst drives, stakeholder + UX involved, stakeholder approves
        (SetupStep::ProductBrief, Analyst) => Some(Driver),
        (SetupStep::ProductBrief, Stakeholder | UxDesigner) => Some(Involved),

        // Requirements: analyst drives, stakeholder involved + approves
        (SetupStep::Requirements, Analyst) => Some(Driver),
        (SetupStep::Requirements, Stakeholder) => Some(Involved),

        // UX Foundations: UX drives, stakeholder + analyst involved
        (SetupStep::UxFoundations, UxDesigner) => Some(Driver),
        (SetupStep::UxFoundations, Stakeholder | Analyst) => Some(Involved),

        // Roadmap: analyst drives, stakeholder + UX + staff involved
        (SetupStep::Roadmap, Analyst) => Some(Driver),
        (SetupStep::Roadmap, Stakeholder | UxDesigner | StaffEngineer) => Some(Involved),

        // Architecture: staff drives, stakeholder involved
        (SetupStep::Architecture, StaffEngineer) => Some(Driver),
        (SetupStep::Architecture, Stakeholder) => Some(Involved),

        // Tech Stack: staff drives, platform + product involved
        (SetupStep::TechStack, StaffEngineer) => Some(Driver),
        (SetupStep::TechStack, PlatformEngineer | ProductEngineer) => Some(Involved),

        // Epic Planning: staff drives, analyst + UX + platform + product involved
        (SetupStep::EpicPlanning, StaffEngineer) => Some(Driver),
        (SetupStep::EpicPlanning, Analyst | UxDesigner | PlatformEngineer | ProductEngineer) => {
            Some(Involved)
        }

        // Stakeholder is approver for all setup steps
        (_, Stakeholder) => Some(Approver),

        _ => None,
    }
}

/// Returns the permission for a role at a given epic step, or `None` if the
/// role is not allowed.
#[allow(clippy::match_same_arms)] // Each step's permissions are intentionally listed separately for readability
pub fn epic_permission(step: EpicStep, role: Role) -> Option<Permission> {
    use Permission::{Approver, Driver, Involved};
    use Role::{
        Analyst, PlatformEngineer, ProductEngineer, StaffEngineer, Stakeholder, UxDesigner,
    };

    match (step, role) {
        // 7.1 Analysis: analyst drives, UX + product involved
        (EpicStep::Analysis, Analyst) => Some(Driver),
        (EpicStep::Analysis, UxDesigner | ProductEngineer) => Some(Involved),

        // 7.2 UX Design: UX drives, analyst + product involved
        (EpicStep::UxDesign, UxDesigner) => Some(Driver),
        (EpicStep::UxDesign, Analyst | ProductEngineer) => Some(Involved),

        // 7.3 Technical Design: product drives, staff + platform involved
        (EpicStep::TechnicalDesign, ProductEngineer) => Some(Driver),
        (EpicStep::TechnicalDesign, StaffEngineer | PlatformEngineer) => Some(Involved),

        // 7.4 Planning: product drives, staff + platform involved
        (EpicStep::Planning, ProductEngineer) => Some(Driver),
        (EpicStep::Planning, StaffEngineer | PlatformEngineer) => Some(Involved),

        // 7.5 Implementation: product + platform drive, staff involved
        (EpicStep::Implementation, ProductEngineer | PlatformEngineer) => Some(Driver),
        (EpicStep::Implementation, StaffEngineer) => Some(Involved),

        // 7.6 Verification: product + platform drive, UX + analyst + staff involved
        (EpicStep::Verification, ProductEngineer | PlatformEngineer) => Some(Driver),
        (EpicStep::Verification, UxDesigner | Analyst | StaffEngineer) => Some(Involved),

        // 7.7 Review: stakeholder approves
        (EpicStep::Review, Stakeholder) => Some(Approver),

        // 7.8 Release: product drives, platform + staff + analyst + UX involved
        (EpicStep::Release, ProductEngineer) => Some(Driver),
        (EpicStep::Release, PlatformEngineer | StaffEngineer | Analyst | UxDesigner) => {
            Some(Involved)
        }

        // 7.9 Retrospective: all roles involved (no single driver)
        (EpicStep::Retrospective, _) => Some(Involved),

        _ => None,
    }
}

/// Check if a role is allowed to participate in a setup step (any permission).
pub fn is_allowed_setup(step: SetupStep, role: Role) -> bool {
    setup_permission(step, role).is_some()
}

/// Check if a role is allowed to participate in an epic step (any permission).
pub fn is_allowed_epic(step: EpicStep, role: Role) -> bool {
    epic_permission(step, role).is_some()
}

/// Check if a role is the primary driver for a setup step.
pub fn is_driver_setup(step: SetupStep, role: Role) -> bool {
    setup_permission(step, role) == Some(Permission::Driver)
}

/// Check if a role is the primary driver for an epic step.
pub fn is_driver_epic(step: EpicStep, role: Role) -> bool {
    epic_permission(step, role) == Some(Permission::Driver)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Setup step permissions --

    #[test]
    fn product_brief_analyst_is_driver() {
        assert_eq!(
            setup_permission(SetupStep::ProductBrief, Role::Analyst),
            Some(Permission::Driver)
        );
    }

    #[test]
    fn product_brief_stakeholder_is_involved() {
        assert_eq!(
            setup_permission(SetupStep::ProductBrief, Role::Stakeholder),
            Some(Permission::Involved)
        );
    }

    #[test]
    fn product_brief_ux_is_involved() {
        assert_eq!(
            setup_permission(SetupStep::ProductBrief, Role::UxDesigner),
            Some(Permission::Involved)
        );
    }

    #[test]
    fn product_brief_staff_not_allowed() {
        assert!(setup_permission(SetupStep::ProductBrief, Role::StaffEngineer).is_none());
    }

    #[test]
    fn requirements_analyst_is_driver() {
        assert!(is_driver_setup(SetupStep::Requirements, Role::Analyst));
    }

    #[test]
    fn ux_foundations_ux_is_driver() {
        assert!(is_driver_setup(SetupStep::UxFoundations, Role::UxDesigner));
    }

    #[test]
    fn roadmap_analyst_is_driver() {
        assert!(is_driver_setup(SetupStep::Roadmap, Role::Analyst));
    }

    #[test]
    fn roadmap_staff_is_involved() {
        assert_eq!(
            setup_permission(SetupStep::Roadmap, Role::StaffEngineer),
            Some(Permission::Involved)
        );
    }

    #[test]
    fn architecture_staff_is_driver() {
        assert!(is_driver_setup(
            SetupStep::Architecture,
            Role::StaffEngineer
        ));
    }

    #[test]
    fn tech_stack_staff_is_driver() {
        assert!(is_driver_setup(SetupStep::TechStack, Role::StaffEngineer));
    }

    #[test]
    fn tech_stack_platform_is_involved() {
        assert!(is_allowed_setup(
            SetupStep::TechStack,
            Role::PlatformEngineer
        ));
    }

    #[test]
    fn epic_planning_staff_is_driver() {
        assert!(is_driver_setup(
            SetupStep::EpicPlanning,
            Role::StaffEngineer
        ));
    }

    #[test]
    fn epic_planning_all_non_stakeholder_involved() {
        assert!(is_allowed_setup(SetupStep::EpicPlanning, Role::Analyst));
        assert!(is_allowed_setup(SetupStep::EpicPlanning, Role::UxDesigner));
        assert!(is_allowed_setup(
            SetupStep::EpicPlanning,
            Role::PlatformEngineer
        ));
        assert!(is_allowed_setup(
            SetupStep::EpicPlanning,
            Role::ProductEngineer
        ));
    }

    // Stakeholder is approver on all setup steps where not already involved
    #[test]
    fn stakeholder_always_allowed_in_setup() {
        for step in SetupStep::ALL {
            assert!(
                is_allowed_setup(step, Role::Stakeholder),
                "stakeholder should be allowed at {step:?}"
            );
        }
    }

    // -- Epic step permissions --

    #[test]
    fn analysis_analyst_is_driver() {
        assert!(is_driver_epic(EpicStep::Analysis, Role::Analyst));
    }

    #[test]
    fn analysis_ux_is_involved() {
        assert!(is_allowed_epic(EpicStep::Analysis, Role::UxDesigner));
    }

    #[test]
    fn ux_design_ux_is_driver() {
        assert!(is_driver_epic(EpicStep::UxDesign, Role::UxDesigner));
    }

    #[test]
    fn technical_design_product_is_driver() {
        assert!(is_driver_epic(
            EpicStep::TechnicalDesign,
            Role::ProductEngineer
        ));
    }

    #[test]
    fn planning_product_is_driver() {
        assert!(is_driver_epic(EpicStep::Planning, Role::ProductEngineer));
    }

    #[test]
    fn implementation_product_is_driver() {
        assert!(is_driver_epic(
            EpicStep::Implementation,
            Role::ProductEngineer
        ));
    }

    #[test]
    fn implementation_platform_is_driver() {
        assert!(is_driver_epic(
            EpicStep::Implementation,
            Role::PlatformEngineer
        ));
    }

    #[test]
    fn implementation_staff_is_involved() {
        assert_eq!(
            epic_permission(EpicStep::Implementation, Role::StaffEngineer),
            Some(Permission::Involved)
        );
    }

    #[test]
    fn verification_product_is_driver() {
        assert!(is_driver_epic(
            EpicStep::Verification,
            Role::ProductEngineer
        ));
    }

    #[test]
    fn verification_ux_is_involved() {
        assert!(is_allowed_epic(EpicStep::Verification, Role::UxDesigner));
    }

    #[test]
    fn verification_analyst_is_involved() {
        assert!(is_allowed_epic(EpicStep::Verification, Role::Analyst));
    }

    #[test]
    fn review_stakeholder_is_approver() {
        assert_eq!(
            epic_permission(EpicStep::Review, Role::Stakeholder),
            Some(Permission::Approver)
        );
    }

    #[test]
    fn review_analyst_not_allowed() {
        assert!(!is_allowed_epic(EpicStep::Review, Role::Analyst));
    }

    #[test]
    fn release_product_is_driver() {
        assert!(is_driver_epic(EpicStep::Release, Role::ProductEngineer));
    }

    #[test]
    fn release_all_others_involved() {
        assert!(is_allowed_epic(EpicStep::Release, Role::PlatformEngineer));
        assert!(is_allowed_epic(EpicStep::Release, Role::StaffEngineer));
        assert!(is_allowed_epic(EpicStep::Release, Role::Analyst));
        assert!(is_allowed_epic(EpicStep::Release, Role::UxDesigner));
    }

    #[test]
    fn retrospective_all_roles_involved() {
        let roles = [
            Role::Stakeholder,
            Role::Analyst,
            Role::UxDesigner,
            Role::StaffEngineer,
            Role::ProductEngineer,
            Role::PlatformEngineer,
        ];
        for role in roles {
            assert!(
                is_allowed_epic(EpicStep::Retrospective, role),
                "{role:?} should be allowed at retrospective"
            );
        }
    }

    // -- Convenience functions --

    #[test]
    fn is_driver_returns_false_for_involved() {
        assert!(!is_driver_setup(SetupStep::ProductBrief, Role::Stakeholder));
    }

    #[test]
    fn is_allowed_returns_false_for_unauthorized() {
        assert!(!is_allowed_setup(
            SetupStep::ProductBrief,
            Role::PlatformEngineer
        ));
    }
}
