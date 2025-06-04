//! This module provides conversion functions for handling SenHive data formats.
//!
//! It contains utilities to convert between different data formats:
//! - JSON to JSON Lines conversion
//! - JSON to CSV conversion with DronePoint data structure
//!
//! These conversions are used primarily for data processing and format standardization
//! when working with SenHive sensor data.
//!
use std::io::Cursor;
use std::num::NonZeroUsize;

use csv::{QuoteStyle, WriterBuilder};
use polars::prelude::{JsonFormat, JsonReader, JsonWriter, SerReader, SerWriter};

use fetiche_formats::senhive::FusedData;
use fetiche_formats::DronePoint;


/// Converts JSON data from regular JSON format to JSON Lines format.
///
/// Takes a byte slice containing JSON data and converts it to JSON Lines format
/// where each JSON object is on a separate line.
///
/// # Arguments
///
/// * `data` - A byte slice containing the JSON data to convert
///
/// # Returns
///
/// Returns a `Result` containing the converted JSON Lines string if successful,
/// or an error if the conversion fails.
///
#[inline]
pub(crate) fn from_json_to_nl(data: &[u8]) -> eyre::Result<String> {
    let cur = Cursor::new(data);
    let mut df = JsonReader::new(cur)
        .with_json_format(JsonFormat::Json)
        .infer_schema_len(NonZeroUsize::new(3))
        .finish()?;

    let mut buf = vec![];
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::JsonLines)
        .finish(&mut df)?;
    Ok(String::from_utf8(buf)?)
}

/// Converts JSON data into CSV format containing DronePoint data.
///
/// Takes a byte slice containing JSON data, deserializes it into a FusedData struct,
/// converts it to a DronePoint, and then serializes it to CSV format.
///
/// # Arguments
///
/// * `data` - A byte slice containing the JSON data to convert
///
/// # Returns
///
/// Returns a `Result` containing the CSV string if successful,
/// or an error if the conversion fails.
///
#[inline]
pub(crate) fn from_json_to_csv(data: &[u8]) -> eyre::Result<String> {
    let cur = Cursor::new(data);
    let data: FusedData = serde_json::from_reader(cur)?;
    let data: DronePoint = (&data).into();

    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(vec![]);

    // Insert data
    //
    wtr.serialize(data)?;

    // Output final csv line
    //
    let data = String::from_utf8(wtr.into_inner()?)?;

    Ok(data)
}
