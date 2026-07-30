//! Various conversion functions for jiff/chrono
//!

use chrono::{DateTime, Datelike, TimeZone, Utc};
use jiff::{RoundMode, Timestamp, Unit, Zoned, ZonedRound};

/// Normalises a given `DateTime<Utc>` instance to the beginning of the same day (00:00:00 UTC).
///
/// # Arguments
///
/// * `date` - A `DateTime<Utc>` instance representing the input date and time.
///
/// # Returns
///
/// This function returns a `Result` containing a `DateTime<Utc>` instance set to the start of the day
/// corresponding to the input date. If an error occurs during the normalisation process, an `Err` is returned.
///
/// # Examples
///
/// ```rust
/// use chrono::{Utc, TimeZone};
/// use fetiche_common::normalise_day;
///
/// let date = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
/// let result = normalise_day(date);
///
/// assert!(result.is_ok());
/// let normalised_date = result.unwrap();
/// assert_eq!(normalised_date, Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
/// ```
///
/// # Errors
///
/// This function will return an `Err` if any error occurs while constructing the `DateTime<Utc>` object,
/// such as invalid date or time values.
///
#[inline]
#[tracing::instrument]
pub fn normalise_day(date: DateTime<Utc>) -> eyre::Result<DateTime<Utc>> {
    let date = Utc
        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .unwrap();
    Ok(date)
}

/// Normalises a given `jiff::Zoned` instance to the beginning of the same day (00:00:00 in its timezone).
///
/// This function rounds down the provided zoned datetime to the start of the day (midnight)
/// in the same timezone, effectively setting the time component to 00:00:00.
///
/// # Arguments
///
/// * `date` - A `jiff::Zoned` instance representing the input date and time in a specific timezone.
///
/// # Returns
///
/// This function returns a `Result` containing a `jiff::Zoned` instance set to the start of the day
/// corresponding to the input date in the same timezone. If an error occurs during the rounding process,
/// an `Err` is returned.
///
/// # Examples
///
/// ```rust
/// use jiff::Timestamp;
/// use fetiche_common::normalise_day_jiff;
///
/// let date: Timestamp = "2024-01-01 12:34:56-00".parse().unwrap();
/// let zoned = date.in_tz("UTC").unwrap();
/// let result = normalise_day_jiff(zoned);
///
/// assert!(result.is_ok());
/// let normalised_date = result.unwrap();
/// assert_eq!(normalised_date.to_string(), "2024-01-01T00:00:00+00:00[UTC]");
/// ```
///
/// # Errors
///
/// This function will return an `Err` if the rounding operation fails, which may occur with
/// invalid datetime values or edge cases in the jiff library.
///
#[inline]
#[tracing::instrument]
pub fn normalise_day_jiff(date: Zoned) -> eyre::Result<Zoned> {
    Ok(date.round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Floor))?)
}

/// Converts a `chrono::DateTime<Utc>` to a `jiff::Timestamp`.
///
/// This is a zero-cost conversion that preserves nanosecond precision.
/// Used when receiving chrono timestamps from external APIs (like fetiche-formats)
/// and converting to jiff for internal computation.
///
/// # Arguments
///
/// * `dt` - A `DateTime<Utc>` from chrono
///
/// # Returns
///
/// A `jiff::Timestamp` representing the same instant in time.
///
/// # Examples
///
/// ```rust
/// use chrono::{Utc, TimeZone};
/// use fetiche_common::chrono_to_jiff;
///
/// let chrono_dt = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
/// let jiff_ts = chrono_to_jiff(chrono_dt);
/// assert_eq!(jiff_ts.as_second(), chrono_dt.timestamp());
/// ```
#[inline]
pub fn chrono_to_jiff(dt: DateTime<Utc>) -> Timestamp {
    Timestamp::from_second(dt.timestamp())
        .expect("valid chrono timestamp")
        .checked_add(jiff::Span::new().nanoseconds(dt.timestamp_subsec_nanos() as i64))
        .expect("nanosecond addition")
}

/// Converts a `jiff::Timestamp` to a `chrono::DateTime<Utc>`.
///
/// This is a zero-cost conversion that preserves nanosecond precision.
/// Used when returning to chrono timestamps for external APIs (like database writes,
/// CSV output, or fetiche-formats types).
///
/// # Arguments
///
/// * `ts` - A `jiff::Timestamp`
///
/// # Returns
///
/// A `Result` containing a `DateTime<Utc>`, or an error if the timestamp is out of
/// chrono's supported range.
///
/// # Examples
///
/// ```rust
/// use jiff::Timestamp;
/// use fetiche_common::jiff_to_chrono;
///
/// let jiff_ts: Timestamp = "2024-01-01T12:00:00Z".parse().unwrap();
/// let chrono_dt = jiff_to_chrono(jiff_ts).unwrap();
/// assert_eq!(chrono_dt.timestamp(), jiff_ts.as_second());
/// ```
///
/// # Errors
///
/// Returns an error if the jiff timestamp is outside chrono's representable range
/// (roughly year 262000 BCE to 262000 CE).
#[inline]
pub fn jiff_to_chrono(ts: Timestamp) -> eyre::Result<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(ts.as_second(), ts.subsec_nanosecond() as u32)
        .ok_or_else(|| eyre::eyre!("Timestamp out of chrono range: {}", ts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalise_day;
    use chrono::prelude::*;
    use jiff::Timestamp;
    use rstest::rstest;

    #[rstest]
    #[case("2024-01-01 00:00:00 UTC", "2024-01-01T00:00:00Z")]
    #[case("2024-01-01 12:00:00 UTC", "2024-01-01T00:00:00Z")]
    #[case("2024-01-01 12:34:56 UTC", "2024-01-01T00:00:00Z")]
    #[case("2024-12-31 12:34:56 UTC", "2024-12-31T00:00:00Z")]
    #[case("2024-04-01 08:34:56 UTC", "2024-04-01T00:00:00Z")]
    fn test_normalise_day(#[case] date: &str, #[case] res: &str) {
        let d = dateparser::parse(&date).unwrap();
        let r = normalise_day(d);
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!(res, r.to_rfc3339_opts(SecondsFormat::Secs, true));
    }

    #[rstest]
    #[case("2024-01-01 00:00:00-00", "2024-01-01T00:00:00+00:00[UTC]")]
    #[case("2024-01-01 12:00:00-00", "2024-01-01T00:00:00+00:00[UTC]")]
    #[case("2024-12-31 23:59:59-00", "2024-12-31T00:00:00+00:00[UTC]")]
    #[case("2024-04-01 08:34:56-00", "2024-04-01T00:00:00+00:00[UTC]")]
    fn test_normalise_day_jiff(#[case] date: &str, #[case] res: &str) {
        let d: Timestamp = date.parse().unwrap();
        let r = normalise_day_jiff(d.in_tz("UTC").unwrap());
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!(res, r.to_string());
    }

    // Test conversion helpers
    #[test]
    fn test_chrono_to_jiff_roundtrip() {
        let chrono_dt = Utc.with_ymd_and_hms(2024, 7, 30, 12, 34, 56).unwrap();
        let jiff_ts = chrono_to_jiff(chrono_dt);
        let chrono_dt2 = jiff_to_chrono(jiff_ts).unwrap();

        assert_eq!(chrono_dt.timestamp(), chrono_dt2.timestamp());
        assert_eq!(chrono_dt, chrono_dt2);
    }

    #[test]
    fn test_jiff_to_chrono_roundtrip() {
        let jiff_ts: Timestamp = "2024-07-30T12:34:56Z".parse().unwrap();
        let chrono_dt = jiff_to_chrono(jiff_ts).unwrap();
        let jiff_ts2 = chrono_to_jiff(chrono_dt);

        assert_eq!(jiff_ts.as_second(), jiff_ts2.as_second());
    }

    #[test]
    fn test_chrono_to_jiff_epoch() {
        let chrono_dt = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        let jiff_ts = chrono_to_jiff(chrono_dt);

        assert_eq!(jiff_ts.as_second(), 0);
    }

    #[test]
    fn test_jiff_to_chrono_epoch() {
        let jiff_ts = Timestamp::from_second(0).unwrap();
        let chrono_dt = jiff_to_chrono(jiff_ts).unwrap();

        assert_eq!(chrono_dt.timestamp(), 0);
        assert_eq!(chrono_dt, Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_conversion_preserves_nanoseconds() {
        let chrono_dt = Utc.timestamp_nanos(1609459200_123456789);
        let jiff_ts = chrono_to_jiff(chrono_dt);
        let chrono_dt2 = jiff_to_chrono(jiff_ts).unwrap();

        assert_eq!(chrono_dt.timestamp_nanos_opt(), chrono_dt2.timestamp_nanos_opt());
    }
}
