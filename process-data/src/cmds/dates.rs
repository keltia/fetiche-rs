use fetiche_common::DateOpts;
use jiff::tz::TimeZone;
use jiff::{RoundMode, Timestamp, Unit, ZonedRound};
use tracing::info;

/// Parses the provided `DateOpts` into a start and end date range.
///
/// This function utilizes the `DateOpts::parse` method to interpret and convert
/// the given date options into a corresponding date interval `(start, stop)`
/// represented as `DateTime<Utc>`. If the parsing fails, it defaults to the
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
    let mode = ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Floor);
    Ok(ts.to_zoned(TimeZone::UTC).round(mode)?.timestamp())
}


#[cfg(test)]
mod tests {
    use crate::parse_date_interval;
    use fetiche_common::{DateOpts, normalise_day_jiff};
    use jiff::Timestamp;

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
