use claude_disciplined::{
    counters::Counters,
    id::{Id, Prefix},
    role::{self, Permission, Role},
    state::{EpicStep, Phase, SetupStep, State},
    store::{MemStore, Store},
};

// -- id::Id::new boundary tests --

#[test]
fn id_new_boundary_one_is_valid() {
    assert!(Id::new(Prefix::T, 1).is_ok());
}

#[test]
fn id_new_boundary_zero_is_invalid() {
    assert!(Id::new(Prefix::T, 0).is_err());
}

#[test]
fn id_new_boundary_fff_is_valid() {
    assert!(Id::new(Prefix::T, 0xFFF).is_ok());
}

#[test]
fn id_new_boundary_1000_is_invalid() {
    assert!(Id::new(Prefix::T, 0x1000).is_err());
}

// -- id::Id::parse hex length --

#[test]
fn id_parse_two_digit_hex_rejected() {
    assert!(Id::parse("T-0F").is_err());
}

#[test]
fn id_parse_four_digit_hex_rejected() {
    assert!(Id::parse("T-000F").is_err());
}

#[test]
fn id_parse_three_digit_hex_accepted() {
    assert!(Id::parse("T-00F").is_ok());
}

// -- id::Id::value accessor returns the right value --

#[test]
fn id_value_accessor_nonzero() {
    let id = Id::new(Prefix::S, 0x0AB).unwrap();
    assert_eq!(id.value(), 0x0AB);
}

#[test]
fn id_value_accessor_one() {
    let id = Id::new(Prefix::S, 1).unwrap();
    assert_eq!(id.value(), 1);
}

#[test]
fn id_value_accessor_not_zero() {
    // Catches mutation: replace value() -> 0
    let id = Id::new(Prefix::T, 2).unwrap();
    assert_ne!(id.value(), 0);
}

#[test]
fn id_value_accessor_not_one_when_different() {
    // Catches mutation: replace value() -> 1
    let id = Id::new(Prefix::T, 2).unwrap();
    assert_ne!(id.value(), 1);
}

// -- IdError Display --

#[test]
fn id_error_out_of_range_display() {
    let err = Id::new(Prefix::T, 0).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("out of range"),
        "expected 'out of range' in: {msg}"
    );
    assert!(msg.contains('T'), "expected prefix T in: {msg}");
}

#[test]
fn id_error_invalid_format_display() {
    let err = Id::parse("NOPE").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("invalid id format"),
        "expected 'invalid id format' in: {msg}"
    );
}

#[test]
fn id_error_unknown_prefix_display() {
    let err = Id::parse("FOO-001").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown id prefix"),
        "expected 'unknown id prefix' in: {msg}"
    );
}

// -- state::SetupStep::next boundary --

#[test]
fn setup_step_product_brief_next_is_requirements() {
    assert_eq!(
        SetupStep::ProductBrief.next(),
        Some(SetupStep::Requirements)
    );
}

#[test]
fn setup_step_epic_planning_next_is_none() {
    assert_eq!(SetupStep::EpicPlanning.next(), None);
}

#[test]
fn setup_step_all_has_seven_entries() {
    assert_eq!(SetupStep::ALL.len(), 7);
}

// -- state::EpicStep::next boundary --

#[test]
fn epic_step_analysis_next_is_ux_design() {
    assert_eq!(EpicStep::Analysis.next(), Some(EpicStep::UxDesign));
}

#[test]
fn epic_step_retrospective_next_is_none() {
    assert_eq!(EpicStep::Retrospective.next(), None);
}

#[test]
fn epic_step_all_has_nine_entries() {
    assert_eq!(EpicStep::ALL.len(), 9);
}

// -- state::EpicStep::escalation_targets --

#[test]
fn epic_step_ux_design_can_escalate() {
    assert!(!EpicStep::UxDesign.escalation_targets().is_empty());
}

#[test]
fn epic_step_analysis_cannot_escalate() {
    assert!(EpicStep::Analysis.escalation_targets().is_empty());
}

// -- state::State::init --

#[test]
fn state_init_phase_is_setup() {
    assert_eq!(State::init().phase, Phase::Setup);
}

#[test]
fn state_init_step_is_product_brief() {
    assert_eq!(State::init().step.as_deref(), Some("product-brief"));
}

// -- state::State step parsers --

#[test]
fn state_setup_step_returns_some_for_valid() {
    let s = State::init();
    assert!(s.setup_step().is_some());
}

#[test]
fn state_epic_step_returns_some_for_valid() {
    let s = State {
        phase: Phase::Epic,
        step: Some("analysis".to_owned()),
        current_milestone: None,
        epic: None,
        active_meeting: None,
    };
    assert!(s.epic_step().is_some());
}

#[test]
fn state_step_none_returns_none_for_both() {
    let s = State {
        phase: Phase::Setup,
        step: None,
        current_milestone: None,
        epic: None,
        active_meeting: None,
    };
    assert!(s.setup_step().is_none());
    assert!(s.epic_step().is_none());
}

// -- role::Role::as_str --

#[test]
fn role_as_str_not_empty() {
    let roles = [
        Role::Stakeholder,
        Role::Analyst,
        Role::UxDesigner,
        Role::StaffEngineer,
        Role::ProductEngineer,
        Role::PlatformEngineer,
    ];
    for role in roles {
        assert!(!role.as_str().is_empty(), "{role:?} as_str is empty");
        assert_ne!(role.as_str(), "xyzzy", "{role:?} as_str is placeholder");
    }
}

// -- role::setup_permission specific "Involved" arms that mutants missed --

#[test]
fn requirements_stakeholder_is_involved() {
    assert_eq!(
        role::setup_permission(SetupStep::Requirements, Role::Stakeholder),
        Some(Permission::Involved)
    );
}

#[test]
fn ux_foundations_analyst_is_involved() {
    assert_eq!(
        role::setup_permission(SetupStep::UxFoundations, Role::Analyst),
        Some(Permission::Involved)
    );
}

#[test]
fn ux_foundations_stakeholder_is_involved() {
    assert_eq!(
        role::setup_permission(SetupStep::UxFoundations, Role::Stakeholder),
        Some(Permission::Involved)
    );
}

#[test]
fn architecture_stakeholder_is_involved() {
    assert_eq!(
        role::setup_permission(SetupStep::Architecture, Role::Stakeholder),
        Some(Permission::Involved)
    );
}

// -- role::epic_permission specific "Involved" arms that mutants missed --

#[test]
fn ux_design_analyst_is_involved() {
    assert_eq!(
        role::epic_permission(EpicStep::UxDesign, Role::Analyst),
        Some(Permission::Involved)
    );
}

#[test]
fn ux_design_product_is_involved() {
    assert_eq!(
        role::epic_permission(EpicStep::UxDesign, Role::ProductEngineer),
        Some(Permission::Involved)
    );
}

#[test]
fn technical_design_staff_is_involved() {
    assert_eq!(
        role::epic_permission(EpicStep::TechnicalDesign, Role::StaffEngineer),
        Some(Permission::Involved)
    );
}

#[test]
fn technical_design_platform_is_involved() {
    assert_eq!(
        role::epic_permission(EpicStep::TechnicalDesign, Role::PlatformEngineer),
        Some(Permission::Involved)
    );
}

#[test]
fn planning_staff_is_involved() {
    assert_eq!(
        role::epic_permission(EpicStep::Planning, Role::StaffEngineer),
        Some(Permission::Involved)
    );
}

#[test]
fn planning_platform_is_involved() {
    assert_eq!(
        role::epic_permission(EpicStep::Planning, Role::PlatformEngineer),
        Some(Permission::Involved)
    );
}

// -- role::is_driver_epic catches "replace with true" --

#[test]
fn is_driver_epic_returns_false_for_involved() {
    // Analyst is involved (not driver) at implementation
    assert!(!role::is_driver_epic(
        EpicStep::Implementation,
        Role::Analyst
    ));
}

#[test]
fn is_driver_epic_returns_false_for_not_allowed() {
    // Stakeholder is not allowed at implementation
    assert!(!role::is_driver_epic(
        EpicStep::Implementation,
        Role::Stakeholder
    ));
}

// -- counters::Counters --

#[test]
fn counters_new_count_is_zero() {
    let c = Counters::new();
    assert_eq!(c.count(Prefix::Req), 0);
}

#[test]
fn counters_first_next_is_001() {
    let mut c = Counters::new();
    let id = c.next(Prefix::T).unwrap();
    assert_eq!(id.to_string(), "T-001");
}

#[test]
fn counters_next_increments_by_one() {
    let mut c = Counters::new();
    c.next(Prefix::T).unwrap();
    let second = c.next(Prefix::T).unwrap();
    assert_eq!(second.value(), 2);
}

#[test]
fn counters_count_not_zero_after_next() {
    let mut c = Counters::new();
    c.next(Prefix::Req).unwrap();
    assert_ne!(c.count(Prefix::Req), 0);
}

#[test]
fn counters_count_not_one_when_zero() {
    let c = Counters::new();
    assert_ne!(c.count(Prefix::T), 1);
}

// -- store::MemStore --

#[test]
fn memstore_initialize_makes_initialized() {
    let s = MemStore::new();
    assert!(!s.is_initialized());
    s.initialize(&State::init(), &Counters::new()).unwrap();
    assert!(s.is_initialized());
}

#[test]
fn memstore_write_state_is_readable() {
    let s = MemStore::new();
    let state = State::init();
    s.write_state(&state).unwrap();
    assert_eq!(s.read_state().unwrap().phase, Phase::Setup);
}

#[test]
fn memstore_append_approval_not_empty() {
    let s = MemStore::new();
    s.append_approval("test").unwrap();
    assert!(!s.read_approvals().unwrap().is_empty());
}

#[test]
fn memstore_approval_content_matches() {
    let s = MemStore::new();
    s.append_approval("hello").unwrap();
    assert_eq!(s.read_approvals().unwrap()[0], "hello");
}

// -- commands::CmdError Display --

#[test]
fn cmd_error_already_initialized_display() {
    let err = claude_disciplined::commands::CmdError::AlreadyInitialized;
    assert!(!err.to_string().is_empty());
}

#[test]
fn cmd_error_not_initialized_display() {
    let err = claude_disciplined::commands::CmdError::NotInitialized;
    assert!(!err.to_string().is_empty());
}

#[test]
fn cmd_error_store_display() {
    let err = claude_disciplined::commands::CmdError::Store("boom".to_owned());
    let msg = err.to_string();
    assert!(msg.contains("boom"));
}

#[test]
fn memstore_error_display_not_empty() {
    let s = MemStore::new();
    let err = s.read_state().unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty());
}

// -- store::FsStore mutation targets --

#[test]
fn fs_store_error_display_not_empty() {
    let err = claude_disciplined::store::FsStoreError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "not found",
    ));
    let msg = err.to_string();
    assert!(!msg.is_empty());
    assert!(msg.contains("not found"));
}

#[test]
fn fs_store_is_initialized_requires_both_dir_and_state() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());

    // Neither exists
    assert!(!store.is_initialized());

    // Create dir but not state file
    std::fs::create_dir_all(tmp.path().join(".workflow")).unwrap();
    assert!(!store.is_initialized());
}

#[test]
fn fs_store_counters_roundtrip_preserves_values() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());
    store.initialize(&State::init(), &Counters::new()).unwrap();

    let mut counters = Counters::new();
    counters.next(Prefix::Req).unwrap();
    counters.next(Prefix::Req).unwrap();
    store.write_counters(&counters).unwrap();

    let loaded = store.read_counters().unwrap();
    assert_eq!(loaded.count(Prefix::Req), 2);
}

#[test]
fn fs_store_read_counters_returns_default_when_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let store = claude_disciplined::store::FsStore::new(tmp.path());

    // Create .workflow dir but no counters file
    std::fs::create_dir_all(tmp.path().join(".workflow")).unwrap();
    std::fs::write(
        tmp.path().join(".workflow/state.yml"),
        "phase: setup\nstep: product-brief\n",
    )
    .unwrap();

    let counters = store.read_counters().unwrap();
    assert_eq!(counters.count(Prefix::Req), 0);
}
