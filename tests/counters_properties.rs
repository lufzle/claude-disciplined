use claude_disciplined::{counters::Counters, id::Prefix};
use proptest::prelude::*;

fn arb_prefix() -> impl Strategy<Value = Prefix> {
    prop_oneof![
        Just(Prefix::Req),
        Just(Prefix::Nfr),
        Just(Prefix::M),
        Just(Prefix::E),
        Just(Prefix::S),
        Just(Prefix::T),
        Just(Prefix::D),
        Just(Prefix::Ai),
        Just(Prefix::F),
        Just(Prefix::Ur),
    ]
}

proptest! {
    #[test]
    fn next_ids_are_sequential(prefix in arb_prefix(), count in 1u16..=50) {
        let mut c = Counters::new();
        let mut ids = Vec::new();
        for _ in 0..count {
            ids.push(c.next(prefix).unwrap());
        }
        for (i, id) in ids.iter().enumerate() {
            let expected = u16::try_from(i).unwrap() + 1;
            prop_assert_eq!(id.value(), expected);
        }
    }

    #[test]
    fn count_matches_allocations(prefix in arb_prefix(), count in 0u16..=50) {
        let mut c = Counters::new();
        for _ in 0..count {
            c.next(prefix).unwrap();
        }
        prop_assert_eq!(c.count(prefix), count);
    }

    #[test]
    fn prefixes_are_independent(a in arb_prefix(), b in arb_prefix(), n in 1u16..=10) {
        prop_assume!(a != b);
        let mut c = Counters::new();
        for _ in 0..n {
            c.next(a).unwrap();
        }
        prop_assert_eq!(c.count(a), n);
        prop_assert_eq!(c.count(b), 0);
    }

    #[test]
    fn yaml_roundtrip_preserves_counts(prefix in arb_prefix(), count in 1u16..=20) {
        let mut c = Counters::new();
        for _ in 0..count {
            c.next(prefix).unwrap();
        }
        let yaml = serde_yaml::to_string(&c).unwrap();
        let parsed: Counters = serde_yaml::from_str(&yaml).unwrap();
        prop_assert_eq!(c, parsed);
    }
}
