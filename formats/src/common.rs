//! Common code and struct.
//!

use csv::WriterBuilder;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::EnumString;
use tracing::trace;

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
pub fn prepare_csv<T>(data: Vec<T>, header: bool) -> eyre::Result<String>
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
    use crate::Format;

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
