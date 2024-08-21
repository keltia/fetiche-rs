//! This library is there to share some common code amongst all fetiche modules.
//!

use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::{crate_name, crate_version};
pub use config::*;
pub use container::*;
pub use dateopts::*;
pub use daterange::*;
use eyre::Result;
pub use location::*;
pub use runtime::*;

mod config;
mod container;
mod dateopts;
mod daterange;
mod location;
mod macros;
mod runtime;

const NAME: &str = crate_name!();
const VERSION: &str = crate_version!();

// -----

/// How to retrieve specific version of this crate.
///
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
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
/// fn main() {
///     let foo = Foo::new();
///
///     assert_eq!(2, foo.version());
///     println!("struct Foo version is {}", foo.version());
/// }
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

/// This takes any given date and return the beginning of this day as a `DateTime<Utc>`
///
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
