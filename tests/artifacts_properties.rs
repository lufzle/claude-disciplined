use claude_disciplined::{
    artifacts,
    id::{Id, Prefix},
};
use proptest::prelude::*;

fn arb_slug() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9\\-]{0,19}".prop_filter("non-empty", |s| !s.is_empty())
}

proptest! {
    #[test]
    fn requirement_path_contains_id_and_slug(value in 1u16..=0xFFF, slug in arb_slug()) {
        let id = Id::new(Prefix::Req, value).unwrap();
        let path = artifacts::requirement_path(&id, &slug);
        prop_assert!(path.starts_with("requirements/"));
        prop_assert!(std::path::Path::new(&path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")));
        prop_assert!(path.contains(&id.to_string()));
        prop_assert!(path.contains(&slug));
    }

    #[test]
    fn nfr_path_contains_id_and_slug(value in 1u16..=0xFFF, slug in arb_slug()) {
        let id = Id::new(Prefix::Nfr, value).unwrap();
        let path = artifacts::nfr_path(&id, &slug);
        prop_assert!(path.starts_with("requirements/"));
        prop_assert!(std::path::Path::new(&path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")));
        prop_assert!(path.contains(&id.to_string()));
    }

    #[test]
    fn milestone_path_contains_id(value in 1u16..=0xFFF, slug in arb_slug()) {
        let id = Id::new(Prefix::M, value).unwrap();
        let path = artifacts::milestone_path(&id, &slug);
        prop_assert!(path.starts_with("milestones/"));
        prop_assert!(path.ends_with("/milestone.md"));
        prop_assert!(path.contains(&id.to_string()));
    }

    #[test]
    fn front_matter_always_valid_yaml(value in 1u16..=0xFFF) {
        let req_id = Id::new(Prefix::Req, value).unwrap();
        let nfr_id = Id::new(Prefix::Nfr, value).unwrap();
        let m_id = Id::new(Prefix::M, value).unwrap();

        for fm in [
            artifacts::requirement_front_matter(&req_id),
            artifacts::nfr_front_matter(&nfr_id),
            artifacts::milestone_front_matter(&m_id),
        ] {
            // Strip --- delimiters and parse as YAML
            let yaml_content = fm.trim_start_matches("---\n").trim_end_matches("---\n");
            let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(yaml_content);
            prop_assert!(parsed.is_ok(), "invalid YAML: {fm}");
        }
    }
}
