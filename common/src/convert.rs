//! Date and time normalization helpers using Jiff
//!

use jiff::{RoundMode, Unit, ZonedDateTime, ZonedDateTimeRound};

/// Normalises a given `jiff::ZonedDateTime` instance to the beginning of the same day (00:00:00 in its timezone).
///
/// This function rounds down the provided zoned datetime to the start of the day (midnight)
/// in the same timezone, effectively setting the time component to 00:00:00.
///
/// # Arguments
///
/// * `date` - A `jiff::ZonedDateTime` instance representing the input date and time in a specific timezone.
///
/// # Returns
///
/// This function returns a `Result` containing a `jiff::ZonedDateTime` instance set to the start of the day
/// corresponding to the input date in the same timezone. If an error occurs during the rounding process,
/// an `Err` is returned.
///
/// # Examples
///
/// ```rust
/// use jiff::Timestamp;
/// use fetiche_common::normalise_day;
///
/// let date: Timestamp = "2024-01-01 12:34:56-00".parse().unwrap();
/// let zoned = date.in_tz("UTC").unwrap();
/// let result = normalise_day(zoned);
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
pub fn normalise_day(date: ZonedDateTime) -> eyre::Result<ZonedDateTime> {
    Ok(date.round(ZonedDateTimeRound::new().smallest(Unit::Day).mode(RoundMode::Floor))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;
    use rstest::rstest;

    #[rstest]
    #[case("2024-01-01 00:00:00-00", "2024-01-01T00:00:00+00:00[UTC]")]
    #[case("2024-01-01 12:00:00-00", "2024-01-01T00:00:00+00:00[UTC]")]
    #[case("2024-12-31 23:59:59-00", "2024-12-31T00:00:00+00:00[UTC]")]
    #[case("2024-04-01 08:34:56-00", "2024-04-01T00:00:00+00:00[UTC]")]
    fn test_normalise_day(#[case] date: &str, #[case] res: &str) {
        let d: Timestamp = date.parse().unwrap();
        let r = normalise_day(d.in_tz("UTC").unwrap());
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!(res, r.to_string());
    }

    #[test]
    fn test_normalise_day_in_paris() -> eyre::Result<()> {
        let input: jiff::Timestamp = "2024-07-30T12:34:56Z".parse()?;
        let paris = input.in_tz("Europe/Paris")?;
        let midnight = normalise_day(paris)?;

        assert_eq!(midnight.date().to_string(), "2024-07-30");
        assert_eq!(midnight.hour(), 0);
        assert_eq!(midnight.minute(), 0);
        assert_eq!(midnight.second(), 0);

        Ok(())
    }

}
