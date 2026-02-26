/// Returns the current date as YYYY-MM-DD.
pub fn today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple date calculation without external dependencies
    let days = now / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Returns the current UTC timestamp as ISO 8601.
pub fn now_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = now / 86400;
    let secs_in_day = now % 86400;
    let (year, month, day) = days_to_ymd(days);
    let hour = secs_in_day / 3600;
    let minute = (secs_in_day % 3600) / 60;
    let second = secs_in_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn days_to_ymd(total_days: u64) -> (u64, u64, u64) {
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    let z = total_days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_format_is_yyyy_mm_dd() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[7..8], "-");
    }

    #[test]
    fn now_iso_format_has_t_and_z() {
        let n = now_iso();
        assert!(n.contains('T'));
        assert!(n.ends_with('Z'));
    }

    #[test]
    fn today_year_is_reasonable() {
        let t = today();
        let year: u64 = t[..4].parse().unwrap();
        assert!(year >= 2025);
        assert!(year <= 2100);
    }

    #[test]
    fn epoch_day_zero_is_1970_01_01() {
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn known_date_2026_03_08() {
        // 2026-03-08 is day 20520 from epoch
        let (y, m, d) = days_to_ymd(20_520);
        assert_eq!((y, m, d), (2026, 3, 8));
    }
}
