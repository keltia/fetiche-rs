//! Definition of a data formats
//!
//! This module makes the link between the shared output formats `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new formats, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input formats and the transformations needed.
//!

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::{BufReader, Read};

use csv::{Reader, WriterBuilder};
use eyre::Result;
use parquet_derive::ParquetRecordWriter;
use serde::{Deserialize, Serialize};
use strum::EnumString;
use tabled::{builder::Builder, settings::Style};
use tracing::{debug, trace};

// Re-export for convenience
//
pub use aeroscope::*;
pub use asd::*;
pub use asterix::*;
pub use avionix::*;
pub use drone::*;
#[cfg(feature = "flightaware")]
pub use flightaware::*;
pub use opensky::*;
pub use parquet::*;
pub use safesky::*;

mod aeroscope;
mod asd;
mod asterix;
mod avionix;
mod drone;
#[cfg(feature = "flightaware")]
mod flightaware;
mod opensky;
mod parquet;
mod safesky;

/// Current formats.hcl version
///
const FVERSION: usize = 2;

// -----

pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// For each format, we define a set of key attributes that will get displayed.
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

/// Struct to be read from an HCL file at compile-time
///
#[derive(Debug, Deserialize)]
pub struct FormatFile {
    /// Version
    pub version: usize,
    /// Ordered list of format metadata
    pub format: BTreeMap<String, FormatDescr>,
}

/// This struct holds the different data formats that we support.
///
#[derive(
    Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq, strum::Display, EnumString, Serialize,
)]
#[strum(serialize_all = "lowercase")]
pub enum Format {
    #[default]
    None,
    Adsb21,
    Aeroscope,
    Asd,
    Avionix,
    Cat21,
    Cat129,
    Flightaware,
    Opensky,
    PandaStateVector,
    Parquet,
    Safesky,
}

/// This is the special hex string for ICAO codes
///
pub type ICAOString = [u8; 6];

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
                let l: $name = match $rec.deserialize(None) {
                    Ok(rec) => rec,
                    Err(e) => {
                        panic!("{}", e.to_string());
                    }
                };
                Cat21::from(&l)
            },
        )+
            _ => panic!("unknown format"),
        }
    };
}

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
                    .map(|(cnt, rec)| {
                        $to::from(&rec.unwrap())
                    })
                    .collect();
                debug!("res={:?}", res);
                Ok(res)
            }
        }
    };
}

impl Format {
    /// Process each record coming from the input source, apply `Cat::from()` onto it
    /// and return the list.  This is used when reading from the csv files.
    ///
    #[tracing::instrument(skip(self))]
    pub fn from_csv<R>(self, rdr: &mut Reader<R>) -> Result<Vec<Cat21>>
    where
        R: Read + Debug,
    {
        debug!("Reading & transforming…");
        let res: Vec<_> = rdr
            .records()
            .enumerate()
            .inspect(|(n, _)| trace!("record #{}", n))
            .map(|(cnt, rec)| {
                let rec = rec.unwrap();
                debug!("rec={:?}", rec);
                let mut line = into_cat21!(self, rec, Aeroscope, Asd, Safesky, PandaStateVector);
                line.rec_num = cnt;
                line
            })
            .collect();
        Ok(res)
    }
}

impl Format {
    /// List all supported formats into a string using `tabled`.
    ///
    pub fn list() -> Result<String> {
        let descr = include_str!("formats.hcl");
        let fstr: FormatFile = hcl::from_str(descr)?;

        // Safety checks
        //
        assert_eq!(fstr.version, FVERSION);

        let header = vec!["Name", "Type", "Description"];

        let mut builder = Builder::default();
        builder.set_header(header);

        fstr.format.iter().for_each(|(name, entry)| {
            let mut row = vec![];

            let name = name.clone();
            let dtype = entry.dtype.clone();
            let description = entry.description.clone();
            let source = entry.source.clone();
            let url = entry.url.clone();

            let row_text = format!("{}\nSource: {} -- URL: {}", description, source, url);
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
    pub fn list_plain() -> Result<String> {
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

/// This structure hold a general location object with lat/long.
///
/// In CSV files, the two fields are merged into this struct on deserialization
/// and used as-is when coming from JSON.
///
#[derive(Copy, Clone, Debug, Deserialize, ParquetRecordWriter, PartialEq, Serialize)]
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
