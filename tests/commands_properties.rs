use claude_disciplined::{
    commands,
    state::{Phase, SetupStep},
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
    ]
}

fn store_at_step(step: SetupStep) -> MemStore {
    let store = MemStore::new();
    commands::init(&store).unwrap();
    // Advance to the desired step by approving and advancing
    let mut current = SetupStep::ProductBrief;
    while current != step {
        commands::request_approval(&store, current.as_str()).unwrap();
        commands::advance(&store).unwrap();
        current = current.next().unwrap();
    }
    store
}

proptest! {
    #[test]
    fn init_always_produces_setup_product_brief(_seed in 0u32..1000) {
        let store = MemStore::new();
        let state = commands::init(&store).unwrap();
        prop_assert_eq!(state.phase, Phase::Setup);
        prop_assert_eq!(state.step.as_deref(), Some("product-brief"));
        prop_assert!(state.current_milestone.is_none());
        prop_assert!(state.epic.is_none());
        prop_assert!(state.active_meeting.is_none());
    }

    #[test]
    fn status_after_init_matches_init_result(_seed in 0u32..1000) {
        let store = MemStore::new();
        let init_state = commands::init(&store).unwrap();
        let status_state = commands::status(&store).unwrap();
        prop_assert_eq!(init_state, status_state);
    }

    #[test]
    fn init_is_idempotent_guard(_seed in 0u32..100) {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let err = commands::init(&store);
        prop_assert!(err.is_err());
    }

    #[test]
    fn advance_without_approval_always_fails(step in arb_setup_step()) {
        let store = store_at_step(step);
        let result = commands::advance(&store);
        prop_assert!(result.is_err());
    }

    #[test]
    fn advance_with_approval_moves_to_next(step in arb_setup_step()) {
        let store = store_at_step(step);
        commands::request_approval(&store, step.as_str()).unwrap();
        let new_state = commands::advance(&store).unwrap();
        let expected_next = step.next().unwrap();
        prop_assert_eq!(new_state.step.as_deref(), Some(expected_next.as_str()));
    }

    #[test]
    fn advance_preserves_phase(step in arb_setup_step()) {
        let store = store_at_step(step);
        commands::request_approval(&store, step.as_str()).unwrap();
        let new_state = commands::advance(&store).unwrap();
        prop_assert_eq!(new_state.phase, Phase::Setup);
    }

    #[test]
    fn request_approval_is_recorded(step in arb_setup_step()) {
        let store = store_at_step(step);
        commands::request_approval(&store, step.as_str()).unwrap();
        let approvals = store.read_approvals().unwrap();
        let has_step = approvals.iter().any(|a| a.contains(step.as_str()));
        prop_assert!(has_step);
    }

    #[test]
    fn create_requirement_ids_are_sequential(count in 1u16..=10) {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        for i in 1..=count {
            let id = commands::create_requirement(&store, &format!("req-{i}")).unwrap();
            prop_assert_eq!(id.value(), i);
        }
    }

    #[test]
    fn create_requirement_writes_readable_file(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_requirement(&store, &slug).unwrap();
        let path = format!("requirements/{id}-{slug}.md");
        let content = store.read_file(&path).unwrap();
        prop_assert!(content.contains(&id.to_string()));
        prop_assert!(content.contains("status: active"));
    }

    #[test]
    fn propose_nfr_always_starts_draft(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::propose_nfr(&store, &slug).unwrap();
        let path = format!("requirements/{id}-{slug}.md");
        let content = store.read_file(&path).unwrap();
        prop_assert!(content.contains("status: draft"));
    }

    #[test]
    fn create_milestone_writes_readable_file(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_milestone(&store, &slug).unwrap();
        let path = format!("milestones/{id}-{slug}/milestone.md");
        let content = store.read_file(&path).unwrap();
        prop_assert!(content.contains(&id.to_string()));
    }

    #[test]
    fn create_epic_writes_inside_milestone(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_epic(&store, "milestones/M-001-mvp", &slug).unwrap();
        let path = format!("milestones/M-001-mvp/epics/{id}-{slug}/epic.md");
        let content = store.read_file(&path).unwrap();
        prop_assert!(content.contains(&id.to_string()));
    }

    #[test]
    fn create_story_has_nfrs_field(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_story(&store, "epics/E-001-auth", &slug).unwrap();
        let path = format!("epics/E-001-auth/stories/{id}-{slug}/story.md");
        let content = store.read_file(&path).unwrap();
        prop_assert!(content.contains("nfrs: []"));
    }

    #[test]
    fn create_task_has_pending_status(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_task(&store, "stories/S-001-login", &slug).unwrap();
        let path = format!("stories/S-001-login/tasks/{id}-{slug}.md");
        let content = store.read_file(&path).unwrap();
        prop_assert!(content.contains("status: pending"));
    }

    #[test]
    fn write_then_read_preserves_content(slug in "[a-z][a-z0-9\\-]{0,9}", body in "[a-z ]{1,50}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::create_requirement(&store, &slug).unwrap();
        commands::write_artifact(&store, "requirement", Some("REQ-001"), &body).unwrap();
        let content = commands::read_artifact(&store, "requirement", Some("REQ-001")).unwrap();
        prop_assert!(content.contains(&body));
        prop_assert!(content.contains("id: REQ-001"));
    }

    #[test]
    fn create_flow_has_type(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_flow(&store, "stories/S-001-login", &slug, "happy-path").unwrap();
        let path = format!("stories/S-001-login/flows/{id}-{slug}.md");
        let content = store.read_file(&path).unwrap();
        prop_assert!(content.contains("type: happy-path"));
    }

    // -- Front matter utilities --

    #[test]
    fn replace_field_roundtrip(
        field in "[a-z_]{1,15}",
        old_val in "[a-z0-9\\-]{1,20}",
        new_val in "[a-z0-9\\-]{1,20}",
    ) {
        let content = format!("---\n{field}: {old_val}\nother: keep\n---\n");
        let replaced = commands::replace_front_matter_field(&content, &field, &new_val);
        let extracted = commands::extract_field(&replaced, &field);
        prop_assert_eq!(extracted.as_deref(), Some(new_val.as_str()));
        // Other fields preserved
        prop_assert!(replaced.contains("other: keep"));
    }

    #[test]
    fn replace_field_preserves_line_count(
        field in "[a-z_]{1,15}",
        old_val in "[a-z0-9\\-]{1,20}",
        new_val in "[a-z0-9\\-]{1,20}",
    ) {
        let content = format!("---\n{field}: {old_val}\nother: keep\n---\n");
        let replaced = commands::replace_front_matter_field(&content, &field, &new_val);
        prop_assert_eq!(content.lines().count(), replaced.lines().count());
    }

    #[test]
    fn extract_field_returns_none_for_absent(
        field in "[a-z_]{1,15}",
    ) {
        let content = "---\nunrelated: value\n---\n";
        let result = commands::extract_field(content, &field);
        prop_assert!(result.is_none());
    }

    #[test]
    fn extract_field_returns_none_for_null(
        field in "[a-z_]{1,15}",
    ) {
        let content = format!("---\n{field}: null\n---\n");
        let result = commands::extract_field(&content, &field);
        prop_assert!(result.is_none());
    }

    // -- Task lifecycle properties --

    #[test]
    fn task_start_then_complete_is_completed(_seed in 0u32..50) {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::create_milestone(&store, "mvp").unwrap();
        commands::create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        commands::create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();
        let state = claude_disciplined::state::State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        let id = commands::create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "ui",
        ).unwrap();
        let id_str = id.to_string();
        commands::task_start(&store, &id_str).unwrap();
        commands::task_complete(&store, &id_str).unwrap();
        let tasks = commands::task_list(&store, None, None, Some("completed")).unwrap();
        prop_assert_eq!(tasks.len(), 1);
        prop_assert_eq!(&tasks[0].id, &id_str);
    }

    #[test]
    fn task_block_unblock_returns_to_pending(_seed in 0u32..50) {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        commands::create_milestone(&store, "mvp").unwrap();
        commands::create_epic(&store, "milestones/M-001-mvp", "auth").unwrap();
        commands::create_story(&store, "milestones/M-001-mvp/epics/E-001-auth", "login").unwrap();
        let state = claude_disciplined::state::State {
            phase: Phase::Epic,
            step: Some("implementation".to_owned()),
            current_milestone: Some("M-001".to_owned()),
            epic: Some("E-001".to_owned()),
            active_meeting: None,
        };
        store.write_state(&state).unwrap();
        let id = commands::create_task(
            &store,
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login",
            "ui",
        ).unwrap();
        let id_str = id.to_string();
        commands::task_block(&store, &id_str, "waiting").unwrap();
        commands::task_unblock(&store, &id_str).unwrap();
        let tasks = commands::task_list(&store, None, None, Some("pending")).unwrap();
        prop_assert_eq!(tasks.len(), 1);
    }
}
