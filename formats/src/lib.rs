//! Definition of a data formats
//!
//! This module makes the link between the shared output formats `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new formats, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input formats and the transformations needed.
//!

use csv::WriterBuilder;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::EnumString;
use tracing::trace;

// Re-export for convenience
//
pub use aeroscope::*;
pub use asd::*;
pub use asterix::*;
pub use avionix::*;
pub use dronepoint::*;
#[cfg(feature = "flightaware")]
pub use flightaware::*;
pub use format::*;
pub use opensky::*;
#[cfg(feature = "safesky")]
pub use safesky::*;

mod aeroscope;
mod asd;
mod asterix;
mod avionix;
mod dronepoint;
#[cfg(feature = "flightaware")]
mod flightaware;
mod format;
mod opensky;
#[cfg(feature = "safesky")]
mod safesky;
pub mod senhive;

/// WWe distinguish between the site-specific data formats and general ADS-B
///
#[derive(Clone, Debug, Deserialize, strum::Display, EnumString, strum::VariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum DataType {
    /// ADS-B data
    Adsb,
    /// Drone data, site-specific
    Drone,
    /// Write formats
    Write,
}

/// This is the special hex string for ICAO codes
///
pub type ICAOString = [u8; 6];

/// Generate a converter called `$name` which takes `&str` and
/// output a `Vec<$to>`.  `input` is deserialized from JSON as
/// `$from`.
///
/// Uses `$to::from()` for each format.
///
/// You will need to `use` these in every file you use the macro
/// ```no_run
/// use eyre::Result;
/// use log::debug;
/// ```
/// or
/// ```no_run
/// use eyre::Result;
/// use tracing::debug;
/// ```
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
            pub fn $name(input: &str) -> Result<Vec<$to>> {
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

/// This structure hold a general location object with lat/long.
///
/// In CSV files, the two fields are merged into this struct on deserialization
/// and used as-is when coming from JSON.
///
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Position {
    // Latitude in degrees
    pub latitude: f32,
    /// Longitude in degrees
    pub longitude: f32,
}

impl Default for Position {
    fn default() -> Self {
        Position {
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}


#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TodCalculated {
    C,
    L,
    #[default]
    N,
    R,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Bool {
    Y,
    #[default]
    N,
}

/// Convert into feet
///
#[inline]
pub fn to_feet(a: f32) -> u32 {
    (a * 3.28084) as u32
}

/// Convert into knots
///
#[inline]
pub fn to_knots(a: f32) -> f32 {
    a * 0.54
}

/// Convert to meters
///
#[inline]
pub fn to_meters(a: f32) -> f32 {
    a / 3.28084
}

/// Output the final csv file with a different delimiter 'now ":")
///
#[tracing::instrument]
pub fn prepare_csv<T>(data: Vec<T>, header: bool) -> Result<String>
where
    T: Serialize + Debug,
{
    trace!("Generating output…");
    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new()
        .delimiter(b':')
        .has_headers(header)
        .from_writer(vec![]);

    // Insert data
    //
    data.iter().for_each(|rec| {
        wtr.serialize(rec).unwrap();
    });

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_default() {
        let s = Format::default();

        assert_eq!(Format::None, s);
    }

    #[test]
    fn test_to_feet() {
        assert_eq!(1, to_feet(0.305))
    }

    #[test]
    fn test_to_knots() {
        assert_eq!(1.00008, to_knots(1.852))
    }

    #[test]
    fn test_position_default() {
        let p = Position::default();
        assert_eq!(
            Position {
                latitude: 0.0,
                longitude: 0.0,
            },
            p
        );
    }
}
