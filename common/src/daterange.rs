//! Module handling date ranges
//!

use chrono::{DateTime, Duration, Utc};
use eyre::Result;
use jiff::{civil::Date, ToSpan};

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

    let mut intv = begin
        .series(1.days())
        .take_while(|&ts| ts < end)
        .collect::<Vec<_>>();

    if intv.is_empty() && begin < end {
        intv.push(begin);
    }

    Ok(intv)
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
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
}
