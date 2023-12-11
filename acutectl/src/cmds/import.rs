use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

use eyre::Result;
use strum::EnumVariantNames;
use tracing::trace;

use fetiche_formats::{Asd, DronePoint, Format};

use crate::Engine;

/// Input file format, can be CSV, JSON or Parquet
///
#[derive(Debug, strum::Display, EnumVariantNames)]
pub enum FileInput {
    /// CSV with limited schema support
    Csv,
    /// Invalid
    Invalid,
    /// JSON (NDJSON to be precise)
    Json,
    /// Parquet with embedded schema
    Parquet,
}

impl FromStr for FileInput {
    type Err = std::io::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // value is a pathname
        //
        let ext = Path::new(s).extension().unwrap().to_ascii_lowercase();

        let ext = String::from_utf8(ext.as_encoded_bytes().to_vec()).unwrap();
        Ok(match ext.as_str() {
            ".json" => FileInput::Json,
            ".csv" => FileInput::Csv,
            ".parquet" | ".pq" => FileInput::Parquet,
            _ => FileInput::Invalid,
        })
    }
}

#[tracing::instrument(skip(_engine))]
pub fn import_data(_engine: &Engine, data: &str, _fmt: Format) -> Result<()> {
    trace!("import_data");

    // Transform into our `Drone` struct and sort it by "journey"
    //
    let data: Vec<Asd> = serde_json::from_str(data)?;

    let _journeys = BTreeMap::<u32, Vec<DronePoint>>::new();

    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("foo.csv", FileInput::Csv)]
    #[case("foo.CSv", FileInput::Csv)]
    #[case("foo.json", FileInput::Json)]
    #[case("foo.parquet", FileInput::Parquet)]
    fn test_fileinput_from(#[case] inp: &str, #[case] out: FileInput) -> Result<()> {
        assert_eq!(out, FileInput::from_str(inp));
        Ok(())
    }
}
