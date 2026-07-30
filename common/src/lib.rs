//! This library is there to share some common code amongst all fetiche modules.
//!
pub use config::*;
pub use container::*;
pub use convert::*;
pub use csv::*;
pub use dateopts::*;
pub use daterange::*;
pub use location::*;
pub use logging::*;
pub use tz::*;

mod config;
mod container;
mod convert;
mod csv;
mod dateopts;
mod daterange;
mod location;
mod logging;
mod macros;
mod tz;

use clap::{crate_name, crate_version};

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

