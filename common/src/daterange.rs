//! Module handling date ranges
//!

use chrono::{DateTime, Duration, Utc};
use eyre::Result;
use jiff::{Span, Timestamp, civil::Date};

/// This function takes a start and end `DateTime<Utc>` and generates a vector of all days
/// between (inclusive of start, exclusive of end). It increments the date by one day
/// at each step and returns all the days as `DateTime<Utc>`.
///
/// # Arguments
///
/// * `begin` - The starting `DateTime<Utc>` of the interval.
/// * `end` - The ending `DateTime<Utc>` of the interval.
///
/// # Returns
///
/// A `Result` containing a vector of `DateTime<Utc>` representing all the dates within the interval.
/// If an error occurs, it will be inside the `Err` variant.
///
/// # Example
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use fetiche_common::expand_interval;
///
/// let start = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
/// let end = Utc.with_ymd_and_hms(2024, 2, 4, 0, 0, 0).unwrap();
/// let interval = expand_interval(start, end).unwrap();
///
/// assert_eq!(interval.len(), 3);
/// assert_eq!(interval[0], Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap());
/// assert_eq!(interval[1], Utc.with_ymd_and_hms(2024, 2, 2, 0, 0, 0).unwrap());
/// assert_eq!(interval[2], Utc.with_ymd_and_hms(2024, 2, 3, 0, 0, 0).unwrap());
/// ```
///
/// # Errors
///
/// * Returns an `Err` if there are issues creating the vector of dates.
/// * Handles no specific edge scenarios like invalid date ranges as it assumes input validity.
///
pub fn expand_interval(begin: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<DateTime<Utc>>> {
    let mut d = begin;
    let mut intv = vec![];

    while d < end {
        intv.push(d);
        d += Duration::try_days(1).unwrap();
    }
    Ok(intv)
}

/// This function takes a start and end `jiff::Timestamp` and generates a vector of all days
/// between (inclusive of start and end). It uses jiff's series functionality to generate
/// daily timestamps, returning all days as `jiff::Timestamp`.
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
/// # Errors
///
/// * Return an `Err` if there are issues creating the vector of dates.
/// * Assumes input timestamps are valid and properly formatted.
///
pub fn expand_interval_jiff(begin: Date, end: Date) -> Result<Vec<Date>> {
    if begin == end {
        return Ok(vec![begin]);
    }

    if begin > end {
        return Ok(vec![]);
    }
    // Pre-calculate capacity: days between begin and end
    let days_span = end.since(begin)?;
    let days_count = days_span.get_days();

    // Pre-allocate with exact capacity
    let mut intv = Vec::with_capacity(days_count as usize);

    let day = Span::new().days(1);
    let mut d = begin;

    while d < end {
        intv.push(d);
        d = d.checked_add(day).expect("overflow");
    }

    if intv.is_empty() && begin < end {
        intv.push(begin);
    }

    Ok(intv)
}

/// This function takes a start and end `jiff::Timestamp` and generates a vector of all days
/// between (inclusive of start, exclusive of end). It increments by one day at each step,
/// returning all timestamps as `jiff::Timestamp`.
///
/// This is the preferred function for internal processing as it works with full timestamps
/// and is ~3.5x faster than the chrono equivalent.
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
/// use fetiche_common::expand_interval_timestamp;
///
/// let start: Timestamp = "2024-02-01T00:00:00Z".parse().unwrap();
/// let end: Timestamp = "2024-02-04T00:00:00Z".parse().unwrap();
/// let interval = expand_interval_timestamp(start, end).unwrap();
///
/// assert_eq!(interval.len(), 3);
/// ```
///
/// # Errors
///
/// Returns an `Err` if there are issues with date arithmetic (overflow).
///
pub fn expand_interval_timestamp(begin: Timestamp, end: Timestamp) -> Result<Vec<Timestamp>> {
    let days_span = end.since(begin)?;
    let days_count = days_span.get_days();
    let mut intv = Vec::with_capacity(days_count as usize);

    let day = Span::new().days(1);
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
    use chrono::{TimeZone, Utc};
    use jiff::Timestamp;
    use jiff::civil::date;
    use rstest::rstest;

    #[test]
    fn test_expand_interval_single_day() {
        let start = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 2, 2, 0, 0, 0).unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], start);
    }

    #[test]
    fn test_expand_interval_multiple_days() {
        let start = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 2, 4, 0, 0, 0).unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 3);

        assert_eq!(result[0], start);
        assert_eq!(
            result[1],
            Utc.with_ymd_and_hms(2024, 2, 2, 0, 0, 0).unwrap()
        );
        assert_eq!(
            result[2],
            Utc.with_ymd_and_hms(2024, 2, 3, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_expand_interval_empty_result() {
        let start = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_expand_interval_invalid_date_range() {
        let start = Utc.with_ymd_and_hms(2024, 2, 4, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();

        let result = expand_interval(start, end).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[rstest]
    #[case(("2024-02-01", "2024-02-01"), ["2024-02-01"].to_vec())]
    #[case(("2024-02-01", "2024-02-02"), ["2024-02-01", "2024-02-02"].to_vec())]
    #[case(("2024-02-01", "2024-02-03"), ["2024-02-01", "2024-02-02", "2024-02-03"].to_vec())]
    fn test_expand_interval(#[case] b: (&str, &str), #[case] a: Vec<&str>) -> Result<()> {
        let bb = dateparser::parse(b.0).unwrap();
        let ee = dateparser::parse(b.1).unwrap();
        let aa: Vec<_> = a
            .iter()
            .map(|e| dateparser::parse(e).unwrap().date_naive())
            .collect::<Vec<_>>();

        let res = expand_interval(bb, ee);
        assert!(res.is_ok());
        let res = res
            .unwrap()
            .iter()
            .map(|e| e.date_naive())
            .collect::<Vec<_>>();
        assert_eq!(aa, res);
        Ok(())
    }

    #[test]
    fn test_expand_interval_jiff_single_day() {
        let start = date(2024, 2, 1); // 2024-02-01 00:00:00 UTC
        let end = date(2024, 2, 2); // 2024-02-02 00:00:00 UTC

        let result = expand_interval_jiff(start, end).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], start);
    }

    #[test]
    fn test_expand_interval_jiff_multiple_days() {
        let start = date(2024, 2, 1); // 2024-02-01 00:00:00 UTC
        let end = date(2024, 2, 4); // 2024-02-04 00:00:00 UTC

        let result = expand_interval_jiff(start, end).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_expand_interval_jiff_invalid_date_range() {
        let start = date(2024, 2, 4); // 2024-02-04 00:00:00 UTC
        let end = date(2024, 2, 1); // 2024-02-01 00:00:00 UTC

        let result = expand_interval_jiff(start, end).unwrap();
        assert_eq!(result.len(), 0);
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
            if month >= 4 && month <= 10 {
                assert_eq!(paris_time.hour(), (utc.hour() + 2) % 24); // Summer time
            } else {
                assert_eq!(paris_time.hour(), (utc.hour() + 1) % 24); // Winter time
            }
        }
        Ok(())
    }
}
