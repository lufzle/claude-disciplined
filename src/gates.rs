use crate::{state::SetupStep, store::Store};

/// A single gate violation — a condition that must be met before advancing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub rule: &'static str,
    pub message: String,
}

/// Check all gate conditions for advancing from the given setup step.
///
/// Returns an empty vec if all gates pass.
pub fn check_setup_advance<S: Store>(step: SetupStep, store: &S) -> Vec<Violation> {
    let mut violations = Vec::new();
    require_approval(&mut violations, store, step.as_str());
    violations
}

fn require_approval<S: Store>(violations: &mut Vec<Violation>, store: &S, step_name: &str) {
    if !has_approval(store, step_name) {
        violations.push(Violation {
            rule: "approval-required",
            message: format!("{step_name} must be approved by stakeholder"),
        });
    }
}

fn has_approval<S: Store>(store: &S, step_name: &str) -> bool {
    store.read_approvals().is_ok_and(|approvals| {
        approvals
            .iter()
            .any(|line| line.contains(&format!("\"step\":\"{step_name}\"")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemStore;

    fn store_with_approval(step: &str) -> MemStore {
        let store = MemStore::new();
        store
            .append_approval(&format!(
                "{{\"step\":\"{step}\",\"approved_by\":\"stakeholder\"}}"
            ))
            .unwrap();
        store
    }

    #[test]
    fn product_brief_fails_without_approval() {
        let store = MemStore::new();
        let violations = check_setup_advance(SetupStep::ProductBrief, &store);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "approval-required");
    }

    #[test]
    fn product_brief_passes_with_approval() {
        let store = store_with_approval("product-brief");
        let violations = check_setup_advance(SetupStep::ProductBrief, &store);
        assert!(violations.is_empty());
    }

    #[test]
    fn all_setup_steps_fail_without_approval() {
        let store = MemStore::new();
        for step in SetupStep::ALL {
            let violations = check_setup_advance(step, &store);
            assert!(
                !violations.is_empty(),
                "{step:?} should fail without approval"
            );
        }
    }

    #[test]
    fn all_setup_steps_pass_with_approval() {
        for step in SetupStep::ALL {
            let store = store_with_approval(step.as_str());
            let violations = check_setup_advance(step, &store);
            assert!(
                violations.is_empty(),
                "{step:?} should pass with approval, got: {violations:?}"
            );
        }
    }

    #[test]
    fn violation_message_contains_step_name() {
        let store = MemStore::new();
        let violations = check_setup_advance(SetupStep::Architecture, &store);
        assert!(violations[0].message.contains("architecture"));
    }
}
