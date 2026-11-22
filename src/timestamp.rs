//! UTC timestamps used in the headers of generated files.

use std::time::{SystemTime, UNIX_EPOCH};

/// The current UTC time as an RFC 3339 timestamp, e.g. `2026-11-22T14:05:09Z`.
///
/// If the system clock is before the Unix epoch, the epoch itself is used so
/// that generation never fails just because it cannot read the time.
pub fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    format_rfc3339(seconds)
}

fn format_rfc3339(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Convert days since the Unix epoch to a proleptic Gregorian date.
///
/// This is Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::format_rfc3339;

    #[test]
    fn epoch_is_formatted_in_utc() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn known_timestamps_are_formatted() {
        assert_eq!(format_rfc3339(1_000_000_000), "2001-09-09T01:46:40Z");
        assert_eq!(format_rfc3339(1_764_000_000), "2025-11-24T16:00:00Z");
    }

    #[test]
    fn leap_day_is_formatted() {
        assert_eq!(format_rfc3339(1_709_164_800), "2024-02-29T00:00:00Z");
    }
}
