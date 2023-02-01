//! Definition of a data format-specs
//!
//! This module makes the link between the shared output format-specs `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new format-specs, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input format-specs and the transformations needed.
//!

pub mod input;
pub mod output;

use crate::input::aeroscope::Aeroscope;
use crate::input::asd::Asd;
use crate::input::safesky::Safesky;
use crate::output::Cat21;

use anyhow::Result;
use csv::Reader;
use log::debug;
use serde::{Deserialize, Serialize};

use std::fmt::{Debug, Display, Formatter};
use std::io::Read;

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Format {
    None,
    Aeroscope,
    Asd,
    Opensky,
    Safesky,
}

impl Default for Format {
    fn default() -> Self {
        Format::None
    }
}

/// Macro to create the code which deserialize known types.
///
/// It takes three arguments:
/// - from
/// - object
/// - list of types
///
macro_rules! into_cat21 {
    ($from: ident, $rec:ident, $($name:ident),+) => {
        match $from {
        $(
            Format::$name => {
                let l: $name = $rec.deserialize(None).unwrap();
                Cat21::from(&l)
            },
        )+
            _ => panic!("unknown format"),
        }
    };
}

impl Format {
    // Process each record coming from the input source, apply `Cat::from()` onto it
    // and return the list.  This is used when reading from the csv files.
    //
    pub fn from_csv<T>(self, rdr: &mut Reader<T>) -> Result<Vec<Cat21>>
    where
        T: Read,
    {
        debug!("Reading & transforming…");
        let res: Vec<_> = rdr
            .records()
            .enumerate()
            .map(|(cnt, rec)| {
                let rec = rec.unwrap();
                debug!("rec={:?}", rec);
                let mut line = into_cat21!(self, rec, Aeroscope, Asd, Safesky);
                line.rec_num = cnt;
                line
            })
            .collect();
        Ok(res)
    }
}

impl From<&str> for Format {
    /// Create a format-specs from its name
    ///
    fn from(s: &str) -> Self {
        match s {
            "aeroscope" => Format::Aeroscope,
            "asd" => Format::Asd,
            "opensky" => Format::Opensky,
            "safesky" => Format::Safesky,
            _ => Format::None,
        }
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String = match self {
            Format::Aeroscope => "aeroscope".into(),
            Format::Asd => "asd".into(),
            Format::Safesky => "safesky".into(),
            Format::Opensky => "opensky".into(),
            Format::None => "none".into(),
        };
        write!(f, "{}", s)
    }
}

/// This structure hold a general location object with lat/long.
///
/// In CSV files, the two fields are merged into this struct on deserialization
/// and used as-is when coming from JSON.
///
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Position {
    // Latitude in degrees
    pub latitude: f32,
    /// Longitude in degrees
    pub longitude: f32,
}

impl Default for Position {
    /// makes testing easier
    #[inline]
    fn default() -> Self {
        Position {
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TodCalculated {
    C,
    L,
    N,
    R,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Bool {
    Y,
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
}
