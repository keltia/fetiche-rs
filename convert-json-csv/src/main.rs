//! A simple JSON-to-CSV file converter utility using Polars.
//!
//! This tool converts JSON Lines (JSONL) formatted files to CSV while preserving the column
//! ordering. It uses the Polars data processing library to handle the conversion, which
//! prevents the column reordering issues that can occur with other tools.
//!
//! # Features
//!
//! - Converts JSONL files to CSV format
//! - Preserves column ordering in output
//! - Automatically infers schema from input data
//! - Adds headers to the output CSV file
//!
//! # Usage
//!
//! ```shell
//! convert-json-csv input.jsonl
//! ```
//!
//! The tool will create an output file with the same base name but with a .csv extension.
//! For example, `input.jsonl` will be converted to `input.csv`.
//!
use std::{fs::File, io::BufReader, num::NonZeroUsize, path::Path};

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use eyre::Result;
use polars_io::prelude::*;

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Filename, can be just the basename and .csv/.parquet are implied
    pub input: String,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let input = opts.input.as_str();

    let inp = File::open(input)?;
    let rdr = BufReader::new(inp);

    let output = Path::new(input).file_stem().unwrap().to_str().unwrap();
    let output = Path::new(output).with_extension("csv");

    let mut df = JsonReader::new(rdr)
        .with_json_format(JsonFormat::JsonLines)
        .infer_schema_len(NonZeroUsize::new(10))
        .finish()?;

    let mut out = File::create(&output)?;
    let _ = CsvWriter::new(&mut out)
        .include_header(true)
        .with_quote_style(QuoteStyle::Necessary)
        .finish(&mut df)?;

    eprintln!("\n{input} converted to {output:?}.");
    Ok(())
}
