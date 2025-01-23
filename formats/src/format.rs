use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use strum::EnumString;
use tabled::builder::Builder;
use tabled::settings::Style;

/// Current formats.hcl version
///
const FVERSION: usize = 2;

// -----

pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// This struct represents the format descriptor for each of the supported data types.
///
/// It is primarily used to define and document the metadata for all the supported
/// data formats in the application. Each format is tied to a specific type of data
/// and provides details such as a description, source, and a referential URL.
///
/// Fields:
/// - `dtype`: Indicates the type of data the format corresponds to. This is a required field.
/// - `description`: A free-text overview about what the format represents or its purpose.
/// - `source`: Information about the origin or provider of the format.
/// - `url`: A referential URL where the format details or definition can be found.
///
/// The struct is deserializable to allow for reading metadata from an external configuration file.
///
#[derive(Debug, Deserialize)]
pub struct FormatDescr {
    /// Type of data each format refers to
    #[serde(rename = "type")]
    pub dtype: String,
    /// Free text description
    pub description: String,
    /// Source
    pub source: String,
    /// URL to the site where this is defined
    pub url: String,
}


/// This struct represents the format file structure to be loaded from an HCL file.
///
/// It provides the following fields:
/// - `version`: Indicates the version of the file format.
/// - `format`: An ordered map of format metadata, where the key is the format name and 
///   the value is the corresponding `FormatDescr` containing detailed metadata about the format.
///
#[derive(Debug, Deserialize)]
pub struct FormatFile {
    /// Version
    pub version: usize,
    /// Ordered list of format metadata
    pub format: BTreeMap<String, FormatDescr>,
}

/// The `Format` enum represents the various data formats that the application supports.
///
/// It is designed with the following characteristics:
/// - Implements `Copy`, `Clone`, `Debug`, `Default`, `Deserialize`, `PartialEq`, `Eq`, `Serialize`, 
///   and derives formatting traits using `strum`.
/// - Each variant corresponds to a specific data format used by the system.
///
/// # Variants:
///
/// - `None`: Default, represents the absence of a format.
/// - `Adsb21`: Special cut-down version of ADS-B, limited to specific fields.
/// - `Aeroscope`: DJI Aeroscope-specific data.
/// - `Asd`: Consolidated drone data, from airspacedrone.com.
/// - `CubeData`: Aero Network JSON format by Avionix for drones.
/// - `AvionixCat21`: ADS-B data from the Avionix appliance.
/// - `Cat21`: ECTL Asterix Cat21 flattened CSV.
/// - `Cat129`: ECTL Drone-specific Asterix Cat129.
/// - `Flightaware`: Flightaware API v4 Position data.
/// - `Opensky`: ADS-B data from the Opensky API.
/// - `PandaStateVector`: Opensky data from the Impala historical DB.
/// - `Safesky`: ADS-B data from the Safesky API.
/// - `Senhive`: Drone data from Thales Senhive API.
/// - `DronePoint`: Generic format for drone data.
///
/// ## Example Usage
///
/// ```rust
/// use fetiche_formats::Format;
///
/// let format = Format::CubeData;
/// match format {
///     Format::None => println!("No format selected."),
///     Format::CubeData => println!("Selected CubeData format."),
///     _ => println!("Other formats."),
/// }
/// ```
///
#[derive(
    Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq, strum::Display, EnumString, Serialize,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Format {
    #[default]
    None,
    /// Special cut-down version of ADS-B, limited to specific fields
    Adsb21,
    /// DJI Aeroscope-specific data, coming from the antenna
    Aeroscope,
    /// Consolidated drone data, from airspacedrone.com (ASD)
    Asd,
    /// Aero Network JSON format by Avionix for drones
    CubeData,
    /// ADS-B data from the Avionix appliance
    AvionixCat21,
    /// ECTL Asterix Cat21 flattened CSV
    Cat21,
    /// ECTL Drone specific Asterix Cat129
    Cat129,
    /// Flightaware API v4 Position data
    Flightaware,
    /// ADS-B data from the Opensky API
    Opensky,
    /// Opensky data from the Impala historical DB
    PandaStateVector,
    /// ADS-B data  from the Safesky API
    Safesky,
    /// Drone data from Thales Senhive API
    Senhive,
    /// Genetic format for drone data.
    DronePoint,
}

impl Format {
    /// List all supported formats into a string using `tabled`.
    ///
    pub fn list() -> eyre::Result<String> {
        let descr = include_str!("formats.hcl");
        let fstr: FormatFile = hcl::from_str(descr)?;

        // Safety checks
        //
        assert_eq!(fstr.version, FVERSION);

        let header = vec!["Name", "Type", "Description"];

        let mut builder = Builder::default();
        builder.push_record(header);

        fstr.format.iter().for_each(|(name, entry)| {
            let mut row = vec![];

            let name = name.clone();
            let dtype = entry.dtype.clone();
            let description = entry.description.clone();
            let source = entry.source.clone();
            let url = entry.url.clone();

            let row_text = format!("{}\nSource: {} -- URL: {}", description, source, url);
            let dtype = dtype.to_string();
            row.push(&name);
            row.push(&dtype);
            row.push(&row_text);
            builder.push_record(row);
        });
        let allf = builder.build().with(Style::modern()).to_string();
        let str = format!("List all formats:\n{allf}");
        Ok(str)
    }

    /// List all supported formats into a string
    ///
    pub fn list_plain() -> eyre::Result<String> {
        let descr = include_str!("formats.hcl");
        let fstr: FormatFile = hcl::from_str(descr)?;
        assert_eq!(fstr.version, FVERSION);
        let allf = fstr
            .format
            .iter()
            .map(|(name, entry)| {
                format!(
                    "{:10}{:6}{}\n{:16}Source: {} -- URL: {}",
                    name, entry.dtype, entry.description, "", entry.source, entry.url
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let str = format!("List all formats:\n\n{allf}");
        Ok(str)
    }
}
