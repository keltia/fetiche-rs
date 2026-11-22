use fetiche_common::{DateOpts, normalise_day};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use tracing::info;

/// Parses the provided `DateOpts` into a start and end date range.
///
/// This function utilizes the `DateOpts::parse` method to interpret and convert
/// the given date options into a corresponding date interval `(start, stop)`
/// represented as Jiff timestamps. If the parsing fails, it defaults to the
/// current day.
///
/// # Arguments
///
/// * `date_opts` - A `DateOpts` variant that specifies the desired date range or format.
///
/// # Returns
///
/// A `Result` containing:
/// - `Ok((start, stop))` if the date options are successfully parsed.
/// - `Err` if any error occurs while parsing or normalizing the dates.
///
/// # Behavior
///
/// - If `date_opts` is valid, the function logs the interval and returns
///   the calculated start and stop dates.
/// - If an error occurs, it defaults to the current day, logs the fallback,
///   and returns both `start` and `stop` as the start of the current day.
///
/// # Examples
///
/// Valid date range:
/// ```rust
/// use fetiche_common::DateOpts;
///
/// let date_opts = DateOpts::From {
///     begin: "2023-10-01".to_string(),
///     end: "2023-10-10".to_string(),
/// };
/// let result = DateOpts::parse(date_opts).unwrap();
/// assert_eq!(result.0, "2023-10-01T00:00:00Z".parse().unwrap());
/// assert_eq!(result.1, "2023-10-10T00:00:00Z".parse().unwrap());
/// ```
///
/// Invalid date fallback:
/// ```rust
/// use fetiche_common::DateOpts;
///
/// let date_opts = DateOpts::Week { num: 66 };
/// assert!(DateOpts::parse(date_opts).is_err());
/// ```
///
#[tracing::instrument]
pub fn parse_date_interval(date_opts: DateOpts) -> eyre::Result<(Timestamp, Timestamp)> {
    match DateOpts::parse(date_opts) {
        Ok((start, stop)) => {
            // DateOpts::parse returns jiff::Timestamp already
            info!("Interval: from {} to {}", start, stop);
            Ok((start, stop))
        }
        Err(_) => {
            // Default to current day using jiff
            let now = Timestamp::now();
            let day = normalise_day_jiff(now)?;
            info!("Defaulting to current day: {}", day);
            Ok((day, day))
        }
    }
}

/// Normalizes a jiff timestamp to start of day (00:00:00)
///
pub fn normalise_day_jiff(ts: Timestamp) -> eyre::Result<Timestamp> {
    Ok(normalise_day(ts.to_zoned_datetime(TimeZone::UTC))?.timestamp())
}

/// Formats a timestamp in UTC for installation tables and KML metadata.
///
/// The output uses `YYYY-MM-DD HH:MM:SS` followed by a literal ` UTC` suffix.
/// Whole seconds have no fractional part. Other values use 3, 6, or 9 digits
/// for milliseconds, microseconds, or nanoseconds, using the shortest of
/// these widths that preserves the value exactly.
///
/// # Examples
///
/// | Input timestamp | Formatted output |
/// | --- | --- |
/// | `2024-01-01T00:00:00Z` | `2024-01-01 00:00:00 UTC` |
/// | `2024-01-01T00:00:00.73Z` | `2024-01-01 00:00:00.730 UTC` |
/// | `2024-01-01T00:00:00.73001Z` | `2024-01-01 00:00:00.730010 UTC` |
/// | `2024-01-01T00:00:00.730010002Z` | `2024-01-01 00:00:00.730010002 UTC` |
///
pub(crate) fn format_utc_datetime(value: &Timestamp) -> String {
    let nanos = value.subsec_nanosecond();
    let format = if nanos == 0 {
        "%Y-%m-%d %H:%M:%S UTC"
    } else if nanos % 1_000_000 == 0 {
        "%Y-%m-%d %H:%M:%S%.3f UTC"
    } else if nanos % 1_000 == 0 {
        "%Y-%m-%d %H:%M:%S%.6f UTC"
    } else {
        "%Y-%m-%d %H:%M:%S%.9f UTC"
    };
    value.strftime(format).to_string()
}

#[cfg(test)]
mod tests {
    use super::format_utc_datetime;
    use crate::{normalise_day_jiff, parse_date_interval};
    use fetiche_common::DateOpts;
    use jiff::Timestamp;

    #[test]
    fn test_format_utc_datetime_preserves_fractional_precision() {
        for fraction in ["", ".730", ".730010", ".730010002"] {
            let timestamp = format!("2024-01-01T00:00:00{fraction}Z").parse().unwrap();
            assert_eq!(
                format_utc_datetime(&timestamp),
                format!("2024-01-01 00:00:00{fraction} UTC")
            );
        }
    }

    #[test]
    fn test_parse_date_interval_valid_range() {
        let start_date = "2023-10-01";
        let end_date = "2023-10-10";
        let date_opts = DateOpts::From {
            begin: start_date.to_string(),
            end: end_date.to_string(),
        };

        let result = parse_date_interval(date_opts).unwrap();
        // Results are jiff Timestamps now
        let expected_start = "2023-10-01T00:00:00Z".parse::<Timestamp>().unwrap();
        let expected_end = "2023-10-10T00:00:00Z".parse::<Timestamp>().unwrap();
        assert_eq!(result.0, expected_start);
        assert_eq!(result.1, expected_end);
    }

    #[test]
    fn test_parse_date_interval_invalid_range_defaults_to_now() {
        let invalid_date_opts = DateOpts::Week { num: 66 };
        let now = Timestamp::now();
        let expected_day = normalise_day_jiff(now).unwrap();

        let result = parse_date_interval(invalid_date_opts).unwrap();
        // Both should be the same day (start of day)
        assert_eq!(result.0, expected_day);
        assert_eq!(result.1, expected_day);
    }
}
