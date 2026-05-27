use claude_disciplined::time;
use proptest::prelude::*;

proptest! {
    #[test]
    fn today_always_ten_chars(_seed in 0u32..100) {
        let t = time::today();
        prop_assert_eq!(t.len(), 10);
    }

    #[test]
    fn today_always_has_dashes_at_correct_positions(_seed in 0u32..100) {
        let t = time::today();
        prop_assert_eq!(&t[4..5], "-");
        prop_assert_eq!(&t[7..8], "-");
    }

    #[test]
    fn now_iso_always_ends_with_z(_seed in 0u32..100) {
        let n = time::now_iso();
        prop_assert!(n.ends_with('Z'));
    }

    #[test]
    fn now_iso_always_contains_t(_seed in 0u32..100) {
        let n = time::now_iso();
        prop_assert!(n.contains('T'));
    }
}
