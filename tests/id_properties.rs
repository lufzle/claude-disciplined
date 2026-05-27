use claude_disciplined::id::{Id, Prefix};
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
    fn parse_roundtrips_display(prefix in arb_prefix(), value in 1u16..=0xFFF) {
        let id = Id::new(prefix, value).unwrap();
        let s = id.to_string();
        let parsed = Id::parse(&s).unwrap();
        prop_assert_eq!(id, parsed);
    }

    #[test]
    fn new_rejects_out_of_range(prefix in arb_prefix(), value in 0x1000u16..=u16::MAX) {
        prop_assert!(Id::new(prefix, value).is_err());
    }

    #[test]
    fn new_rejects_zero(prefix in arb_prefix()) {
        prop_assert!(Id::new(prefix, 0).is_err());
    }

    #[test]
    fn display_always_three_hex_digits(prefix in arb_prefix(), value in 1u16..=0xFFF) {
        let id = Id::new(prefix, value).unwrap();
        let s = id.to_string();
        let hex_part = s.rsplit_once('-').unwrap().1;
        prop_assert_eq!(hex_part.len(), 3);
        prop_assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn display_hex_is_uppercase(prefix in arb_prefix(), value in 1u16..=0xFFF) {
        let id = Id::new(prefix, value).unwrap();
        let s = id.to_string();
        let hex_part = s.rsplit_once('-').unwrap().1;
        prop_assert_eq!(hex_part, hex_part.to_uppercase());
    }
}
