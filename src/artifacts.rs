use crate::{id::Id, time};

// -- Paths --

pub fn requirement_path(id: &Id, slug: &str) -> String {
    format!("requirements/{id}-{slug}.md")
}

pub fn nfr_path(id: &Id, slug: &str) -> String {
    format!("requirements/{id}-{slug}.md")
}

pub fn milestone_dir(id: &Id, slug: &str) -> String {
    format!("milestones/{id}-{slug}")
}

pub fn milestone_path(id: &Id, slug: &str) -> String {
    format!("{}/milestone.md", milestone_dir(id, slug))
}

pub fn epic_dir(milestone_dir: &str, id: &Id, slug: &str) -> String {
    format!("{milestone_dir}/epics/{id}-{slug}")
}

pub fn epic_path(milestone_dir: &str, id: &Id, slug: &str) -> String {
    format!("{}/epic.md", epic_dir(milestone_dir, id, slug))
}

pub fn story_dir(epic_dir: &str, id: &Id, slug: &str) -> String {
    format!("{epic_dir}/stories/{id}-{slug}")
}

pub fn story_path(epic_dir: &str, id: &Id, slug: &str) -> String {
    format!("{}/story.md", story_dir(epic_dir, id, slug))
}

pub fn task_path(story_dir: &str, id: &Id, slug: &str) -> String {
    format!("{story_dir}/tasks/{id}-{slug}.md")
}

pub fn flow_path(story_dir: &str, id: &Id, slug: &str) -> String {
    format!("{story_dir}/flows/{id}-{slug}.md")
}

// -- Front matter --

fn front_matter(fields: &[(&str, &str)]) -> String {
    use std::fmt::Write;
    let mut out = String::from("---\n");
    for (key, value) in fields {
        let _ = writeln!(out, "{key}: {value}");
    }
    out.push_str("---\n");
    out
}

pub fn requirement_front_matter(id: &Id) -> String {
    let today = time::today();
    front_matter(&[
        ("id", &id.to_string()),
        ("status", "active"),
        ("superseded_by", "null"),
        ("contributes", "[]"),
        ("created", &today),
        ("updated", &today),
    ])
}

pub fn nfr_front_matter(id: &Id) -> String {
    let today = time::today();
    front_matter(&[
        ("id", &id.to_string()),
        ("status", "draft"),
        ("superseded_by", "null"),
        ("contributes", "[]"),
        ("created", &today),
        ("updated", &today),
    ])
}

pub fn milestone_front_matter(id: &Id) -> String {
    let today = time::today();
    front_matter(&[
        ("id", &id.to_string()),
        ("status", "draft"),
        ("superseded_by", "null"),
        ("satisfies", "[]"),
        ("created", &today),
        ("updated", &today),
    ])
}

pub fn epic_front_matter(id: &Id) -> String {
    let today = time::today();
    front_matter(&[
        ("id", &id.to_string()),
        ("status", "draft"),
        ("superseded_by", "null"),
        ("satisfies", "[]"),
        ("created", &today),
        ("updated", &today),
    ])
}

pub fn story_front_matter(id: &Id) -> String {
    let today = time::today();
    front_matter(&[
        ("id", &id.to_string()),
        ("status", "draft"),
        ("superseded_by", "null"),
        ("satisfies", "[]"),
        ("nfrs", "[]"),
        ("created", &today),
        ("updated", &today),
    ])
}

pub fn task_front_matter(id: &Id) -> String {
    let today = time::today();
    front_matter(&[
        ("id", &id.to_string()),
        ("status", "pending"),
        ("informed_by", "[]"),
        ("blocked_reason", "null"),
        ("created", &today),
        ("updated", &today),
    ])
}

pub fn flow_front_matter(id: &Id, flow_type: &str) -> String {
    let today = time::today();
    front_matter(&[
        ("id", &id.to_string()),
        ("type", flow_type),
        ("status", "active"),
        ("superseded_by", "null"),
        ("created", &today),
        ("updated", &today),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{Id, Prefix};

    fn id(prefix: Prefix) -> Id {
        Id::new(prefix, 1).unwrap()
    }

    // -- Paths --

    #[test]
    fn requirement_path_format() {
        assert_eq!(
            requirement_path(&id(Prefix::Req), "build-apps"),
            "requirements/REQ-001-build-apps.md"
        );
    }

    #[test]
    fn nfr_path_format() {
        assert_eq!(
            nfr_path(&id(Prefix::Nfr), "fast"),
            "requirements/NFR-001-fast.md"
        );
    }

    #[test]
    fn milestone_path_format() {
        assert_eq!(
            milestone_path(&id(Prefix::M), "mvp"),
            "milestones/M-001-mvp/milestone.md"
        );
    }

    #[test]
    fn epic_path_format() {
        assert_eq!(
            epic_path("milestones/M-001-mvp", &id(Prefix::E), "auth"),
            "milestones/M-001-mvp/epics/E-001-auth/epic.md"
        );
    }

    #[test]
    fn story_path_format() {
        let ed = "milestones/M-001-mvp/epics/E-001-auth";
        assert_eq!(
            story_path(ed, &id(Prefix::S), "login"),
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/story.md"
        );
    }

    #[test]
    fn task_path_format() {
        let sd = "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login";
        assert_eq!(
            task_path(sd, &id(Prefix::T), "ui"),
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/tasks/T-001-ui.md"
        );
    }

    #[test]
    fn flow_path_format() {
        let sd = "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login";
        assert_eq!(
            flow_path(sd, &id(Prefix::F), "happy"),
            "milestones/M-001-mvp/epics/E-001-auth/stories/S-001-login/flows/F-001-happy.md"
        );
    }

    // -- Front matter structure --

    #[test]
    fn all_front_matter_valid_structure() {
        let fms = [
            requirement_front_matter(&id(Prefix::Req)),
            nfr_front_matter(&id(Prefix::Nfr)),
            milestone_front_matter(&id(Prefix::M)),
            epic_front_matter(&id(Prefix::E)),
            story_front_matter(&id(Prefix::S)),
            task_front_matter(&id(Prefix::T)),
            flow_front_matter(&id(Prefix::F), "happy-path"),
        ];
        for fm in &fms {
            assert!(fm.starts_with("---\n"), "missing opening: {fm}");
            assert!(fm.ends_with("---\n"), "missing closing: {fm}");
            assert!(fm.contains("id:"), "missing id: {fm}");
            assert!(fm.contains("created:"), "missing created: {fm}");
            assert!(fm.contains("updated:"), "missing updated: {fm}");
        }
    }

    // -- Spec-required fields --

    #[test]
    fn requirement_has_superseded_by() {
        assert!(requirement_front_matter(&id(Prefix::Req)).contains("superseded_by: null"));
    }

    #[test]
    fn nfr_has_superseded_by() {
        assert!(nfr_front_matter(&id(Prefix::Nfr)).contains("superseded_by: null"));
    }

    #[test]
    fn milestone_has_superseded_by() {
        assert!(milestone_front_matter(&id(Prefix::M)).contains("superseded_by: null"));
    }

    #[test]
    fn epic_has_superseded_by() {
        assert!(epic_front_matter(&id(Prefix::E)).contains("superseded_by: null"));
    }

    #[test]
    fn story_has_superseded_by() {
        assert!(story_front_matter(&id(Prefix::S)).contains("superseded_by: null"));
    }

    #[test]
    fn flow_has_superseded_by() {
        assert!(flow_front_matter(&id(Prefix::F), "error").contains("superseded_by: null"));
    }

    #[test]
    fn task_has_blocked_reason() {
        assert!(task_front_matter(&id(Prefix::T)).contains("blocked_reason: null"));
    }

    // -- Status defaults --

    #[test]
    fn requirement_status_active() {
        assert!(requirement_front_matter(&id(Prefix::Req)).contains("status: active"));
    }

    #[test]
    fn nfr_status_draft() {
        assert!(nfr_front_matter(&id(Prefix::Nfr)).contains("status: draft"));
    }

    #[test]
    fn task_status_pending() {
        assert!(task_front_matter(&id(Prefix::T)).contains("status: pending"));
    }

    #[test]
    fn story_has_nfrs_field() {
        assert!(story_front_matter(&id(Prefix::S)).contains("nfrs: []"));
    }

    #[test]
    fn flow_has_type_field() {
        assert!(flow_front_matter(&id(Prefix::F), "error").contains("type: error"));
    }

    #[test]
    fn task_has_informed_by_field() {
        assert!(task_front_matter(&id(Prefix::T)).contains("informed_by: []"));
    }

    // -- Dates are real --

    #[test]
    fn dates_are_not_hardcoded() {
        let fm = requirement_front_matter(&id(Prefix::Req));
        assert!(
            !fm.contains("2026-01-01"),
            "date should not be hardcoded placeholder"
        );
    }
}
