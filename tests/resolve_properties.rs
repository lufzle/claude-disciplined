use claude_disciplined::{commands, resolve, store::MemStore};
use proptest::prelude::*;

proptest! {
    #[test]
    fn singleton_path_always_ends_with_md(name in "(product-brief|architecture|ux-foundations|tech-stack|roadmap)") {
        let path = resolve::singleton_path(&name);
        prop_assert!(
            std::path::Path::new(&path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")),
            "path should end with .md: {path}"
        );
    }

    #[test]
    fn singleton_path_contains_name(name in "(product-brief|architecture|ux-foundations|tech-stack|roadmap)") {
        let path = resolve::singleton_path(&name);
        prop_assert!(path.contains(&name));
    }

    #[test]
    fn artifact_path_resolves_requirement(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_requirement(&store, &slug).unwrap();
        let path = resolve::artifact_path(&store, &id.to_string()).unwrap();
        prop_assert!(path.contains(&id.to_string()));
    }

    #[test]
    fn artifact_path_resolves_milestone(slug in "[a-z][a-z0-9\\-]{0,9}") {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        let id = commands::create_milestone(&store, &slug).unwrap();
        let path = resolve::artifact_path(&store, &id.to_string()).unwrap();
        prop_assert!(path.contains(&id.to_string()));
        prop_assert!(path.ends_with("/milestone.md"));
    }

    #[test]
    fn artifact_path_fails_for_nonexistent_id(_seed in 0u32..100) {
        let store = MemStore::new();
        commands::init(&store).unwrap();
        prop_assert!(resolve::artifact_path(&store, "REQ-FFF").is_err());
    }
}
