//! This library is there to share some common code amongst all fetiche modules.
//!

use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::{crate_name, crate_version};
use eyre::Result;

pub use container::*;
pub use dateopts::*;
pub use daterange::*;
pub use location::*;
pub use runtime::*;

mod container;
mod dateopts;
mod daterange;
mod location;
mod macros;
mod runtime;

const NAME: &str = crate_name!();
const VERSION: &str = crate_version!();

pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}

#[inline]
#[tracing::instrument]
pub fn normalise_day(date: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let date = Utc
        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .unwrap();
    Ok(date)
}

#[cfg(test)]
mod tests {
    use chrono::prelude::*;
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
}
