// Shared with build.rs so date conversion can be checked without build dependencies.
pub fn date_from_unix_days(days: u64) -> String {
    // Gregorian civil date, using March as the start of the calculation year.
    let days = days as i64 + 719468;
    let era = days / 146097;
    let day_of_era = days - era * 146097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    format!("{year:04}{month:02}{day:02}")
}

pub fn windows_version(date: &str) -> u64 {
    assert!(date.len() == 8 && date.bytes().all(|byte| byte.is_ascii_digit()),
        "CAVESTORY_BUILD_VERSION must be YYYYMMDD");
    let year: u64 = date[..4].parse().unwrap();
    let month: u64 = date[4..6].parse().unwrap();
    let day: u64 = date[6..].parse().unwrap();
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month { 2 => if leap { 29 } else { 28 }, 4 | 6 | 9 | 11 => 30, 1 | 3 | 5 | 7 | 8 | 10 | 12 => 31, _ => 0 };
    assert!(year >= 1970 && day >= 1 && day <= days, "CAVESTORY_BUILD_VERSION must be a valid build date");
    (year << 48) | (month << 32) | (day << 16)
}

#[cfg(test)]
mod tests {
    #[test]
    fn utc_build_dates_cover_epoch_leap_day_and_year_boundary() {
        for (days, date) in [(0, "19700101"), (19782, "20240229"), (20088, "20241231"), (20089, "20250101"), (20710, "20260914")] {
            assert_eq!(super::date_from_unix_days(days), date);
        }
        assert_eq!(super::windows_version("20260914"), (2026 << 48) | (9 << 32) | (14 << 16));
    }

    #[test]
    #[should_panic(expected = "valid build date")]
    fn invalid_build_dates_are_rejected() { super::windows_version("20260229"); }
}
