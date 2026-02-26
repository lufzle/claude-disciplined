use serde::{Deserialize, Serialize};

/// Top-level workflow phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Setup,
    Execution,
    Epic,
}

/// Setup-phase steps (1–6) and the per-milestone epic-planning step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SetupStep {
    ProductBrief,
    Requirements,
    UxFoundations,
    Roadmap,
    Architecture,
    TechStack,
    EpicPlanning,
}

impl SetupStep {
    /// All setup steps in order. Epic planning is last (per-milestone).
    pub const ALL: [Self; 7] = [
        Self::ProductBrief,
        Self::Requirements,
        Self::UxFoundations,
        Self::Roadmap,
        Self::Architecture,
        Self::TechStack,
        Self::EpicPlanning,
    ];

    /// Returns the next step in sequence, or `None` if at the end.
    pub fn next(self) -> Option<Self> {
        let idx = Self::ALL.iter().position(|&s| s == self)?;
        Self::ALL.get(idx + 1).copied()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProductBrief => "product-brief",
            Self::Requirements => "requirements",
            Self::UxFoundations => "ux-foundations",
            Self::Roadmap => "roadmap",
            Self::Architecture => "architecture",
            Self::TechStack => "tech-stack",
            Self::EpicPlanning => "epic-planning",
        }
    }
}

/// Epic-execution sub-steps (7.1–7.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpicStep {
    Analysis,
    UxDesign,
    TechnicalDesign,
    Planning,
    Implementation,
    Verification,
    Review,
    Release,
    Retrospective,
}

impl EpicStep {
    /// All epic steps in forward order.
    pub const ALL: [Self; 9] = [
        Self::Analysis,
        Self::UxDesign,
        Self::TechnicalDesign,
        Self::Planning,
        Self::Implementation,
        Self::Verification,
        Self::Review,
        Self::Release,
        Self::Retrospective,
    ];

    /// Returns the next step in forward sequence, or `None` if at
    /// retrospective.
    pub fn next(self) -> Option<Self> {
        let idx = Self::ALL.iter().position(|&s| s == self)?;
        Self::ALL.get(idx + 1).copied()
    }

    /// Returns the valid backward-transition targets from this step.
    pub fn escalation_targets(self) -> &'static [Self] {
        match self {
            Self::UxDesign => &[Self::Analysis],
            Self::TechnicalDesign => &[Self::UxDesign],
            Self::Planning => &[Self::TechnicalDesign],
            Self::Review | Self::Verification => &[Self::Implementation],
            _ => &[],
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analysis => "analysis",
            Self::UxDesign => "ux-design",
            Self::TechnicalDesign => "technical-design",
            Self::Planning => "planning",
            Self::Implementation => "implementation",
            Self::Verification => "verification",
            Self::Review => "review",
            Self::Release => "release",
            Self::Retrospective => "retrospective",
        }
    }
}

/// The full workflow state, persisted as `.workflow/state.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub phase: Phase,
    /// Current step for setup/epic-planning phases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// Current milestone (e.g., "M-001").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_milestone: Option<String>,
    /// Current epic (epic branch only, e.g., "E-001").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,
    /// Currently active meeting slug, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_meeting: Option<String>,
}

impl State {
    /// Create the initial state for a new project.
    pub fn init() -> Self {
        Self {
            phase: Phase::Setup,
            step: Some(SetupStep::ProductBrief.as_str().to_owned()),
            current_milestone: None,
            epic: None,
            active_meeting: None,
        }
    }

    /// Parse the `step` field into a `SetupStep`, if applicable.
    pub fn setup_step(&self) -> Option<SetupStep> {
        let step_str = self.step.as_deref()?;
        serde_yaml::from_str(&format!("\"{step_str}\"")).ok()
    }

    /// Parse the `step` field into an `EpicStep`, if applicable.
    pub fn epic_step(&self) -> Option<EpicStep> {
        let step_str = self.step.as_deref()?;
        serde_yaml::from_str(&format!("\"{step_str}\"")).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Phase serde --

    #[test]
    fn phase_serializes_lowercase() {
        let yaml = serde_yaml::to_string(&Phase::Setup).unwrap();
        assert_eq!(yaml.trim(), "setup");
    }

    #[test]
    fn phase_deserializes_lowercase() {
        let phase: Phase = serde_yaml::from_str("execution").unwrap();
        assert_eq!(phase, Phase::Execution);
    }

    // -- SetupStep --

    #[test]
    fn setup_step_all_in_order() {
        assert_eq!(SetupStep::ALL[0], SetupStep::ProductBrief);
        assert_eq!(SetupStep::ALL[6], SetupStep::EpicPlanning);
    }

    #[test]
    fn setup_step_next_advances() {
        assert_eq!(
            SetupStep::ProductBrief.next(),
            Some(SetupStep::Requirements)
        );
        assert_eq!(SetupStep::TechStack.next(), Some(SetupStep::EpicPlanning));
    }

    #[test]
    fn setup_step_next_at_end_returns_none() {
        assert_eq!(SetupStep::EpicPlanning.next(), None);
    }

    #[test]
    fn setup_step_serde_roundtrip() {
        for step in SetupStep::ALL {
            let yaml = serde_yaml::to_string(&step).unwrap();
            let parsed: SetupStep = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(step, parsed);
        }
    }

    #[test]
    fn setup_step_as_str_matches_serde() {
        for step in SetupStep::ALL {
            let yaml = serde_yaml::to_string(&step).unwrap();
            assert_eq!(step.as_str(), yaml.trim());
        }
    }

    // -- EpicStep --

    #[test]
    fn epic_step_all_in_order() {
        assert_eq!(EpicStep::ALL[0], EpicStep::Analysis);
        assert_eq!(EpicStep::ALL[8], EpicStep::Retrospective);
    }

    #[test]
    fn epic_step_next_advances() {
        assert_eq!(EpicStep::Analysis.next(), Some(EpicStep::UxDesign));
        assert_eq!(EpicStep::Release.next(), Some(EpicStep::Retrospective));
    }

    #[test]
    fn epic_step_next_at_end_returns_none() {
        assert_eq!(EpicStep::Retrospective.next(), None);
    }

    #[test]
    fn epic_step_serde_roundtrip() {
        for step in EpicStep::ALL {
            let yaml = serde_yaml::to_string(&step).unwrap();
            let parsed: EpicStep = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(step, parsed);
        }
    }

    #[test]
    fn epic_step_as_str_matches_serde() {
        for step in EpicStep::ALL {
            let yaml = serde_yaml::to_string(&step).unwrap();
            assert_eq!(step.as_str(), yaml.trim());
        }
    }

    // -- Escalation targets --

    #[test]
    fn escalation_targets_ux_design_to_analysis() {
        assert_eq!(
            EpicStep::UxDesign.escalation_targets(),
            &[EpicStep::Analysis]
        );
    }

    #[test]
    fn escalation_targets_technical_design_to_ux_design() {
        assert_eq!(
            EpicStep::TechnicalDesign.escalation_targets(),
            &[EpicStep::UxDesign]
        );
    }

    #[test]
    fn escalation_targets_planning_to_technical_design() {
        assert_eq!(
            EpicStep::Planning.escalation_targets(),
            &[EpicStep::TechnicalDesign]
        );
    }

    #[test]
    fn escalation_targets_review_to_implementation() {
        assert_eq!(
            EpicStep::Review.escalation_targets(),
            &[EpicStep::Implementation]
        );
    }

    #[test]
    fn escalation_targets_verification_to_implementation() {
        assert_eq!(
            EpicStep::Verification.escalation_targets(),
            &[EpicStep::Implementation]
        );
    }

    #[test]
    fn escalation_targets_analysis_has_none() {
        assert!(EpicStep::Analysis.escalation_targets().is_empty());
    }

    #[test]
    fn escalation_targets_implementation_has_none() {
        assert!(EpicStep::Implementation.escalation_targets().is_empty());
    }

    #[test]
    fn escalation_targets_release_has_none() {
        assert!(EpicStep::Release.escalation_targets().is_empty());
    }

    #[test]
    fn escalation_targets_retrospective_has_none() {
        assert!(EpicStep::Retrospective.escalation_targets().is_empty());
    }

    // -- State --

    #[test]
    fn state_init_starts_at_product_brief() {
        let state = State::init();
        assert_eq!(state.phase, Phase::Setup);
        assert_eq!(state.step.as_deref(), Some("product-brief"));
        assert!(state.current_milestone.is_none());
        assert!(state.epic.is_none());
        assert!(state.active_meeting.is_none());
    }

    #[test]
    fn state_setup_step_parses() {
        let state = State::init();
        assert_eq!(state.setup_step(), Some(SetupStep::ProductBrief));
    }

    #[test]
    fn state_epic_step_parses() {
        let state = State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        assert_eq!(state.epic_step(), Some(EpicStep::Implementation));
    }

    #[test]
    fn state_setup_step_returns_none_for_epic_step() {
        let state = State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: None,
            epic: None,
            active_meeting: None,
        };
        assert!(state.setup_step().is_none());
    }

    #[test]
    fn state_yaml_roundtrip() {
        let state = State::init();
        let yaml = serde_yaml::to_string(&state).unwrap();
        let parsed: State = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(state, parsed);
    }

    #[test]
    fn state_yaml_skips_none_fields() {
        let state = State::init();
        let yaml = serde_yaml::to_string(&state).unwrap();
        assert!(!yaml.contains("current_milestone"));
        assert!(!yaml.contains("epic"));
        assert!(!yaml.contains("active_meeting"));
    }
}
