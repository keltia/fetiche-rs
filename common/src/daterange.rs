//! Module handling date ranges
//!

use eyre::Result;
use jiff::{civil::Date, SignedDuration, Span, SpanRelativeTo, Timestamp, Unit};

/// This function takes a start and end `jiff::civil::Date` and generates a vector of all days
/// between (inclusive of start, exclusive of end). It increments the date by one day
/// at each step, returning all days as `jiff::civil::Date`.
///
/// # Arguments
///
/// * `begin` - The starting `jiff::civil::Date` of the interval.
/// * `end` - The ending `jiff::civil::Date` of the interval.
///
/// # Returns
///
/// A `Result` containing a vector of `jiff::civil::Date` representing all the dates within the interval.
/// If an error occurs, it will be inside the `Err` variant.
///
/// # Example
///
/// ```
/// use jiff::civil::date;
/// use fetiche_common::expand_interval_civil;
///
/// let start = date(2024, 2, 1);
/// let end = date(2024, 2, 4);
/// let interval = expand_interval_civil(start, end).unwrap();
///
/// assert_eq!(interval.len(), 3);
/// assert_eq!(interval[0], date(2024, 2, 1));
/// assert_eq!(interval[1], date(2024, 2, 2));
/// assert_eq!(interval[2], date(2024, 2, 3));
/// ```
///
/// # Errors
///
/// Returns an `Err` if there are issues with date arithmetic or span calculation.
///
pub fn expand_interval_civil(begin: Date, end: Date) -> Result<Vec<Date>> {
    if begin >= end {
        return Ok(vec![]);
    }

    let days_count = end.since(begin)?.total((Unit::Day, begin))?.max(0.0) as usize;

    let mut intv = Vec::with_capacity(days_count);
    let day = Span::new().days(1);
    let mut d = begin;

    while d < end {
        intv.push(d);
        d = d.checked_add(day)?;
    }

    Ok(intv)
}

/// This function takes a start and end `jiff::Timestamp` and generates a vector of all days
/// between (inclusive of start, exclusive of end). It increments by one day at each step,
/// returning all timestamps as `jiff::Timestamp`.
///
/// Each step advances by exactly 24 hours (not a timezone-dependent calendar day).
///
/// # Arguments
///
/// * `begin` - The starting `jiff::Timestamp` of the interval.
/// * `end` - The ending `jiff::Timestamp` of the interval.
///
/// # Returns
///
/// A `Result` containing a vector of `jiff::Timestamp` representing all the dates within the interval.
/// If an error occurs, it will be inside the `Err` variant.
///
/// # Example
///
/// ```
/// use jiff::Timestamp;
/// use fetiche_common::expand_interval;
///
/// let start: Timestamp = "2024-02-01T00:00:00Z".parse().unwrap();
/// let end: Timestamp = "2024-02-04T00:00:00Z".parse().unwrap();
/// let interval = expand_interval(start, end).unwrap();
///
/// assert_eq!(interval.len(), 3);
/// ```
///
/// # Errors
///
/// Returns an `Err` if there are issues with date arithmetic (overflow).
///
pub fn expand_interval(begin: Timestamp, end: Timestamp) -> Result<Vec<Timestamp>> {
    if begin >= end {
        return Ok(vec![]);
    }

    let days_count = end
        .since(begin)?
        .total((Unit::Day, SpanRelativeTo::days_are_24_hours()))?
        .ceil() as usize;
    let mut intv = Vec::with_capacity(days_count);

    let day = SignedDuration::from_hours(24);
    let mut d = begin;
    while d < end {
        intv.push(d);
        d = d.checked_add(day)?;
    }
    Ok(intv)
}


#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;
    use jiff::civil::date;
    use rstest::rstest;

    #[rstest]
    #[case("2024-02-01T00:00:00Z", "2024-02-01T00:00:00Z", &[])]
    #[case("2024-02-01T00:00:00Z", "2024-02-02T00:00:00Z", &["2024-02-01T00:00:00Z"])]
    #[case("2024-02-01T00:00:00Z", "2024-02-04T00:00:00Z", &[
        "2024-02-01T00:00:00Z", "2024-02-02T00:00:00Z", "2024-02-03T00:00:00Z"
    ])]
    fn test_expand_interval(
        #[case] begin: &str,
        #[case] end: &str,
        #[case] expected: &[&str],
    ) -> Result<()> {
        let begin: Timestamp = begin.parse()?;
        let end: Timestamp = end.parse()?;
        let expected: Vec<Timestamp> = expected
            .iter()
            .map(|value| value.parse())
            .collect::<std::result::Result<Vec<_>, _>>()?;

        assert_eq!(expand_interval(begin, end)?, expected);
        Ok(())
    }

    #[test]
    fn test_expand_interval_civil_empty_result() {
        let start = date(2024, 2, 1);
        let end = date(2024, 2, 1);

        let result = expand_interval_civil(start, end).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_expand_interval_civil_single_day() {
        let start = date(2024, 2, 1); // 2024-02-01 00:00:00 UTC
        let end = date(2024, 2, 2); // 2024-02-02 00:00:00 UTC

        let result = expand_interval_civil(start, end).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], start);
    }

    #[test]
    fn test_expand_interval_civil_multiple_days() {
        let start = date(2024, 2, 1); // 2024-02-01 00:00:00 UTC
        let end = date(2024, 2, 4); // 2024-02-04 00:00:00 UTC

        let result = expand_interval_civil(start, end).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], date(2024, 2, 1));
        assert_eq!(result[1], date(2024, 2, 2));
        assert_eq!(result[2], date(2024, 2, 3));
    }

    #[test]
    fn test_expand_interval_civil_invalid_date_range() {
        let start = date(2024, 2, 4); // 2024-02-04 00:00:00 UTC
        let end = date(2024, 2, 1); // 2024-02-01 00:00:00 UTC

        let result = expand_interval_civil(start, end).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_expand_interval_empty_result() {
        let start: Timestamp = "2024-02-01T00:00:00Z".parse().unwrap();
        let end: Timestamp = "2024-02-01T00:00:00Z".parse().unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_expand_interval_single_day() {
        let start: Timestamp = "2024-02-01T00:00:00Z".parse().unwrap();
        let end: Timestamp = "2024-02-02T00:00:00Z".parse().unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], start);
    }

    #[test]
    fn test_expand_interval_multiple_days() {
        let start: Timestamp = "2024-02-01T00:00:00Z".parse().unwrap();
        let end: Timestamp = "2024-02-04T00:00:00Z".parse().unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "2024-02-01T00:00:00Z".parse::<Timestamp>().unwrap());
        assert_eq!(result[1], "2024-02-02T00:00:00Z".parse::<Timestamp>().unwrap());
        assert_eq!(result[2], "2024-02-03T00:00:00Z".parse::<Timestamp>().unwrap());
    }

    #[test]
    fn test_expand_interval_invalid_range() {
        let start: Timestamp = "2024-02-04T00:00:00Z".parse().unwrap();
        let end: Timestamp = "2024-02-01T00:00:00Z".parse().unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_expand_interval_civil_leap_day() -> Result<()> {
        let result = expand_interval_civil(date(2024, 2, 28), date(2024, 3, 2))?;
        assert_eq!(result, vec![date(2024, 2, 28), date(2024, 2, 29), date(2024, 3, 1)]);
        Ok(())
    }

    #[test]
    fn test_expand_interval_partial_day() -> Result<()> {
        let begin: Timestamp = "2024-02-01T12:00:00Z".parse()?;
        let end: Timestamp = "2024-02-03T00:00:00Z".parse()?;
        let expected: Timestamp = "2024-02-02T12:00:00Z".parse()?;

        let result = expand_interval(begin, end)?;
        assert_eq!(result, vec![begin, expected]);
        assert_eq!(result.capacity(), 2);
        Ok(())
    }

    #[test]
    fn test_expand_interval_capacity_preallocation() -> Result<()> {
        let start_jiff = date(2024, 1, 1);
        let end_jiff = date(2024, 1, 11);
        let res_jiff = expand_interval_civil(start_jiff, end_jiff)?;
        assert_eq!(res_jiff.len(), 10);
        assert_eq!(res_jiff.capacity(), 10);

        let start_ts: Timestamp = "2024-01-01T00:00:00Z".parse()?;
        let end_ts: Timestamp = "2024-01-11T00:00:00Z".parse()?;
        let res_ts = expand_interval(start_ts, end_ts)?;
        assert_eq!(res_ts.len(), 10);
        assert_eq!(res_ts.capacity(), 10);

        Ok(())
    }

    #[test]
    fn test_timestamp_to_paris_time() -> Result<()> {
        let timestamps = vec![
            1735689600, // 2025-01-01 00:00:00 UTC
            1735776000, // 2025-01-02 00:00:00 UTC
            1751328000, // 2025-07-01 00:00:00 UTC
        ];

        let paris_tz = "Europe/Paris";

        for ts in timestamps {
            let utc = Timestamp::from_second(ts)?;
            let utc = utc.in_tz("UTC")?;
            let paris_time = utc.in_tz(paris_tz)?;

            // Verify same date
            assert_eq!(utc.date(), paris_time.date());

            // Verify +1h in winter and +2h in summer
            let month = paris_time.date().month();
            if (4..=10).contains(&month) {
                assert_eq!(paris_time.hour(), (utc.hour() + 2) % 24); // Summer time
            } else {
                assert_eq!(paris_time.hour(), (utc.hour() + 1) % 24); // Winter time
            }
        }
        Ok(())
    }
}
