//! Definition of a data formats
//!
//! This module makes the link between the shared output formats `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new formats, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input formats and the transformations needed.
//!

use std::fmt::Debug;
use std::io::Cursor;

use csv::{QuoteStyle, WriterBuilder};
use eyre::Result;
use serde::de::DeserializeOwned;

// Re-export for convenience
//
pub use common::*;
pub use dronepoint::*;
pub use format::*;

#[cfg(feature = "aeroscope")]
pub use aeroscope::*;
#[cfg(feature = "asd")]
pub use asd::*;
#[cfg(feature = "asterix")]
pub use asterix::*;
#[cfg(feature = "flightaware")]
pub use flightaware::*;
#[cfg(feature = "opensky")]
pub use opensky::*;
#[cfg(feature = "safesky")]
pub use safesky::*;

#[cfg(feature = "aeroscope")]
mod aeroscope;
#[cfg(feature = "asd")]
mod asd;
#[cfg(feature = "asterix")]
mod asterix;
#[cfg(feature = "avionix")]
pub mod avionix;
mod common;
mod dronepoint;
#[cfg(feature = "flightaware")]
mod flightaware;
mod format;
#[cfg(feature = "opensky")]
mod opensky;
#[cfg(feature = "safesky")]
mod safesky;
#[cfg(feature = "senhive")]
pub mod senhive;

/// Generate a converter called `$name` which takes `&str` and
/// output a `Vec<$to>`.  `input` is deserialized from JSON as
/// `$from`.
///
/// Uses `$to::from()` for each format.
///
/// You will need to `use` these in every file you use the macro
///
/// log::debug
/// tracing::debug
///
/// Takes 3 arguments:
///
/// - name of the `fn` to create
/// - name of the input `struct`
/// - name of the output type like `Cat21`
///
#[macro_export]
macro_rules! convert_to {
    ($name:ident, $from:ident, $to:ident) => {
        impl $to {
            #[doc = concat!("This is ", stringify!($name), " which convert a json string into a ", stringify!($to), "object")]
            ///
            #[tracing::instrument]
            pub fn $name(input: &str) -> eyre::Result<Vec<$to>> {
                debug!("IN={:?}", input);
                let stream = ::std::io::BufReader::new(input.as_bytes());
                let res = ::serde_json::Deserializer::from_reader(stream).into_iter::<$from>();

                let res: Vec<_> = res
                    .filter(|l| l.is_ok())
                    .enumerate()
                    .inspect(|(n, f)| debug!("cnt={}/{:?}", n, f.as_ref().unwrap()))
                    .map(|(_cnt, rec)| {
                        $to::from(&rec.unwrap())
                    })
                    .collect();
                debug!("res={:?}", res);
                Ok(res)
            }
        }
    };
}

/// Take some struct in JSON and turn it into our own `DronePoint` as a CSV.
///
/// Example:
/// ```no_run
/// # fn main() -> eyre::Result<()> {
/// # use fetiche_formats::Asd;
/// use fetiche_formats::from_json_to_csv;
///
/// # let data: Asd = Asd::default();
/// # let input = String::from("");
///
/// let res = from_json_to_csv(&input.into_bytes(), &data)?;
/// eprintln!("res = {res}");
/// # Ok(())
/// # }
/// ```
///
/// NOTE: `_fake`  is only there to make it a generic.
///
#[inline]
pub fn from_json_to_csv<T>(data: &[u8], _fake: &T) -> Result<String>
where
    T: DeserializeOwned + Debug,
    DronePoint: From<T>,
{
    let cur = Cursor::new(data);
    let data: T = serde_json::from_reader(cur)?;
    let data: DronePoint = data.into();

    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(vec![]);

    // Insert data
    //
    wtr.serialize(data)?;
    wtr.flush()?;

    let res = String::from_utf8(wtr.into_inner()?.to_vec())?;
    Ok(res)
}
