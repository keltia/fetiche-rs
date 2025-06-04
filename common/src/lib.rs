//! This library is there to share some common code amongst all fetiche modules.
//!

use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::{crate_name, crate_version};
pub use config::*;
pub use container::*;
pub use dateopts::*;
pub use daterange::*;
use eyre::Result;
use jiff::{RoundMode, Unit, ZonedRound};
pub use location::*;
pub use logging::*;

mod config;
mod container;
mod dateopts;
mod daterange;
mod location;
mod logging;
mod macros;

const NAME: &str = crate_name!();
const VERSION: &str = crate_version!();

// -----

/// How to retrieve the version of this crate.
///
pub fn version() -> String {
    format!("{NAME}/{VERSION}")
}

// -----

/// This trait implements versioning on a given structure
///
/// ```rust
/// use fetiche_macros::add_version;
/// use fetiche_common::Versioned;
///
/// #[add_version(2)]
/// #[derive(Debug, Default)]
/// pub struct Foo {
///     pub name: String,
/// }
///
/// let foo = Foo::new();
///
/// assert_eq!(2, foo.version());
/// println!("struct Foo version is {}", foo.version());
/// ```
///
pub trait Versioned {
    fn version(&self) -> usize;
}

// -----

/// This trait is a superset of `Versioned` and add a `filename()` method that returns
/// the default filename for the struct when read from a file.
///
/// ```no_run
/// // Specify version and filename.
/// # use serde::Deserialize;
/// use fetiche_common::{IntoConfig, Versioned};
/// use fetiche_macros::into_configfile;
///
/// #[into_configfile(version = 3, filename = "bar.hcl")]
/// #[derive(Debug, Default, Deserialize)]
/// struct Bar {
///     pub value: u32,
/// }
/// ```
///
pub trait IntoConfig: Versioned {
    fn filename(&self) -> String;
}

// -----

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
pub fn normalise_day(date: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let date = Utc
        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .unwrap();
    Ok(date)
}

#[tracing::instrument]
pub fn normalise_day_jiff(date: jiff::Zoned) -> Result<jiff::Zoned> {
    let date = date.round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Trunc))?;
    Ok(date)
}


#[cfg(test)]
mod tests {
    use chrono::prelude::*;
    use jiff::Timestamp;
    use rstest::rstest;

    use super::*;

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
        dbg!(&d);
        let r = normalise_day_jiff(d.in_tz("UTC").unwrap());
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!(res, r.to_string());
    }
}
