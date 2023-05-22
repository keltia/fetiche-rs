//! Definition of a data formats
//!
//! This module makes the link between the shared output formats `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new formats, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input formats and the transformations needed.
//!

use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::io::Read;

use anyhow::Result;
use csv::{Reader, WriterBuilder};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use tabled::{builder::Builder, settings::Style};

// Re-export for convenience
//
pub use aeroscope::*;
pub use asd::*;
pub use asterix::*;
pub use drone::*;
pub use opensky::*;
pub use safesky::*;

mod aeroscope;
mod asd;
mod asterix;
mod drone;
mod opensky;
mod safesky;

/// Current formats.hcl version
///
const FVERSION: usize = 2;

// -----

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
#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Format {
    #[default]
    None,
    Aeroscope,
    Asd,
    Opensky,
    Safesky,
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
    /// Process each record coming from the input source, apply `Cat::from()` onto it
    /// and return the list.  This is used when reading from the csv files.
    ///
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
        let str = format!("List all formats:\n\n{allf}");
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

impl From<&str> for Format {
    /// Create a formats from its name
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
#[derive(Copy, Clone, Default, Debug, Deserialize, PartialEq, Serialize)]
pub struct Position {
    // Latitude in degrees
    pub latitude: f32,
    /// Longitude in degrees
    pub longitude: f32,
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
pub fn prepare_csv<T>(data: Vec<T>) -> Result<String>
where
    T: Serialize,
{
    trace!("Generating output…");
    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new()
        .delimiter(b':')
        .has_headers(true)
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
