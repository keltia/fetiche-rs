//! Common code and struct.
//!

use csv::WriterBuilder;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::EnumString;
use tracing::trace;

/// Represents different types of data formats that can be encountered or utilized 
/// in the context of this application.
///
/// # Variants
///
/// - `Adsb`: Represents ADS-B (Automatic Dependent Surveillance-Broadcast) data, 
///   typically used for tracking aircraft.
/// - `Drone`: Represents site-specific data related to drones, which is not part 
///   of the general ADS-B data format.
/// - `Write`: Represents formats intended for writing output or processing into a
///   specific structure for export or storage.
///
/// # Usage
/// This enum can be useful for distinguishing between different data formats when
/// processing data, and can also be serialized or deserialized for compatibility 
/// with various data storage forms like JSON or CSV.
///
/// # Features
/// - Implements traits like `Clone`, `Debug`, `Deserialize`, `Display`, and `EnumString`.
/// - Supports case-insensitive matching for variant names when deserializing.
/// - Can be converted into a displayable lowercase string with the `strum` crate's
///   `Display` and `VariantNames` traits.
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
/// Represents a geographical position with latitude and longitude values.
///
/// This struct is used to store and manipulate geographic coordinates,
/// typically in degrees. It is compatible with both CSV and JSON formats by
/// deriving the necessary serialization and deserialization traits.
///
/// # Fields
/// * `latitude` - Latitude of the position in degrees. Positive values indicate north, while negative
///   values indicate south.
/// * `longitude` - Longitude of the position in degrees. Positive values indicate east, while negative
///   values indicate west.
///
/// # Usage
/// The `Position` struct is useful for applications that require location-based
/// data, such as mapping, navigation, or aviation systems.
///
/// # Default Values
/// By default, a `Position` struct is initialized to:
/// * `latitude = 0.0`
/// * `longitude = 0.0`
///
/// This can be changed at runtime by creating a new struct instance.
///
/// # Example
/// ```
/// use fetiche_formats::Position;
///
/// let position = Position {
///     latitude: 37.7749,   // Latitude of San Francisco
///     longitude: -122.4194, // Longitude of San Francisco
/// };
///
/// assert_eq!(position.latitude, 37.7749);
/// assert_eq!(position.longitude, -122.4194);
///
/// // Using the default position:
/// let default_position = Position::default();
/// assert_eq!(default_position.latitude, 0.0);
/// assert_eq!(default_position.longitude, 0.0);
/// ```
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

/// Prepares a CSV output from a vector of data, using a custom delimiter.
///
/// # Arguments
///
/// * `data` - A vector of data to serialize into a CSV string. Each item in the vector
///            must implement the `Serialize` and `Debug` traits.
/// * `header` - A boolean that determines if the CSV output should include headers.
///
/// # Returns
///
/// This function returns a `Result<String, eyre::Error>`:
/// * `Ok(String)` - The generated CSV data as a string.
/// * `Err(eyre::Error)` - If an error occurs during the writing or conversion process.
///
/// # CSV Details
///
/// - The delimiter used in the CSV output is `':'` (colon).
/// - Headers are included in the output CSV if `header` is `true`.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use fetiche_formats::prepare_csv;
///
/// #[derive(Serialize, Debug)]
/// struct Record {
///     field1: String,
///     field2: u32,
/// }
///
/// let data = vec![
///     Record {
///         field1: "example1".to_string(),
///         field2: 42,
///     },
///     Record {
///         field1: "example2".to_string(),
///         field2: 87,
///     },
/// ];
///
/// let result = prepare_csv(data, true).unwrap();
/// println!("{}", result);
/// ```
///
/// # Tracing
///
/// This function emits a debug-level tracing message, indicating that CSV output 
/// is being generated.
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
    fn test_position_creation() {
        let position = Position {
            latitude: 51.5074,  // Latitude of London
            longitude: -0.1278, // Longitude of London
        };

        assert_eq!(position.latitude, 51.5074);
        assert_eq!(position.longitude, -0.1278);
    }

    #[test]
    fn test_position_default() {
        let default_position = Position::default();

        assert_eq!(default_position.latitude, 0.0);
        assert_eq!(default_position.longitude, 0.0);
    }

    #[test]
    fn test_position_equality() {
        let pos1 = Position {
            latitude: 40.7128,
            longitude: -74.0060, // New York
        };

        let pos2 = Position {
            latitude: 40.7128,
            longitude: -74.0060, // New York
        };

        assert_eq!(pos1, pos2);
    }

    #[derive(Serialize, Debug, PartialEq)]
    struct SampleRecord {
        field1: String,
        field2: u32,
    }

    #[test]
    fn test_prepare_csv_with_headers() {
        let data = vec![
            SampleRecord {
                field1: "value1".to_string(),
                field2: 10,
            },
            SampleRecord {
                field1: "value2".to_string(),
                field2: 20,
            },
        ];

        let csv_result = prepare_csv(data, true).unwrap();

        let expected_csv = "FIELD1:FIELD2\nvalue1:10\nvalue2:20\n";
        assert_eq!(csv_result, expected_csv);
    }

    #[test]
    fn test_prepare_csv_without_headers() {
        let data = vec![
            SampleRecord {
                field1: "value1".to_string(),
                field2: 10,
            },
            SampleRecord {
                field1: "value2".to_string(),
                field2: 20,
            },
        ];

        let csv_result = prepare_csv(data, false).unwrap();

        let expected_csv = "value1:10\nvalue2:20\n";
        assert_eq!(csv_result, expected_csv);
    }

    #[test]
    fn test_prepare_csv_empty_data() {
        let data: Vec<SampleRecord> = vec![];

        let csv_result_with_headers = prepare_csv(data.clone(), true).unwrap();
        let csv_result_without_headers = prepare_csv(data.clone(), false).unwrap();

        let expected_with_headers = "FIELD1:FIELD2\n";
        let expected_without_headers = "";

        assert_eq!(csv_result_with_headers, expected_with_headers);
        assert_eq!(csv_result_without_headers, expected_without_headers);
    }

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
