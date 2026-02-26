use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::id::{Id, IdError, Prefix};

/// Tracks the next ID to assign for each prefix.
///
/// Persisted as `.workflow/counters.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counters(HashMap<Prefix, u16>);

impl Counters {
    /// Create a fresh set of counters, all starting at zero (no IDs issued).
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Allocate the next ID for the given prefix.
    ///
    /// # Errors
    /// Returns `Err` if the counter would exceed 0xFFF (4095).
    pub fn next(&mut self, prefix: Prefix) -> Result<Id, IdError> {
        let counter = self.0.entry(prefix).or_insert(0);
        let next_value = *counter + 1;
        let id = Id::new(prefix, next_value)?;
        *counter = next_value;
        Ok(id)
    }

    /// Returns how many IDs have been issued for the given prefix.
    pub fn count(&self, prefix: Prefix) -> u16 {
        self.0.get(&prefix).copied().unwrap_or(0)
    }
}

impl Default for Counters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_counters_start_at_zero() {
        let c = Counters::new();
        assert_eq!(c.count(Prefix::Req), 0);
        assert_eq!(c.count(Prefix::T), 0);
    }

    #[test]
    fn next_returns_sequential_ids() {
        let mut c = Counters::new();
        let first = c.next(Prefix::Req).unwrap();
        let second = c.next(Prefix::Req).unwrap();
        assert_eq!(first.to_string(), "REQ-001");
        assert_eq!(second.to_string(), "REQ-002");
    }

    #[test]
    fn next_increments_count() {
        let mut c = Counters::new();
        assert_eq!(c.count(Prefix::T), 0);
        c.next(Prefix::T).unwrap();
        assert_eq!(c.count(Prefix::T), 1);
        c.next(Prefix::T).unwrap();
        assert_eq!(c.count(Prefix::T), 2);
    }

    #[test]
    fn prefixes_are_independent() {
        let mut c = Counters::new();
        c.next(Prefix::Req).unwrap();
        c.next(Prefix::Req).unwrap();
        c.next(Prefix::T).unwrap();
        assert_eq!(c.count(Prefix::Req), 2);
        assert_eq!(c.count(Prefix::T), 1);
        assert_eq!(c.count(Prefix::E), 0);
    }

    #[test]
    fn yaml_roundtrip() {
        let mut c = Counters::new();
        c.next(Prefix::Req).unwrap();
        c.next(Prefix::M).unwrap();
        let yaml = serde_yaml::to_string(&c).unwrap();
        let parsed: Counters = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(c, parsed);
    }

    #[test]
    fn next_at_max_returns_error() {
        let mut c = Counters::new();
        // Manually set counter to 0xFFF
        c.0.insert(Prefix::T, 0xFFF);
        assert!(c.next(Prefix::T).is_err());
    }

    #[test]
    fn next_at_max_does_not_mutate_counter() {
        let mut c = Counters::new();
        c.0.insert(Prefix::T, 0xFFF);
        let _ = c.next(Prefix::T);
        assert_eq!(c.count(Prefix::T), 0xFFF);
    }
}
