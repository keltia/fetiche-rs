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
//! convert-from-json input.jsonl
//! ```
//!
//! ```shell
//! convert-from-json -P input.jsonl
//! ```
//!
//! The tool will create an output file with the same base name but with a .csv extension.
//! For example, `input.jsonl` will be converted to `input.csv`.
//!
use std::{fs, fs::File, io::BufReader, num::NonZeroUsize, path::Path};

use clap::{Parser, crate_authors, crate_description, crate_name, crate_version};
use eyre::Result;
use polars_io::prelude::*;
use polars_utils::compression::ZstdLevel;

#[derive(Parser)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Output parquet file instead of CSV
    #[clap(short = 'P', long)]
    pub parquet: bool,
    /// Filename, can be just the basename and .csv/.parquet are implied
    pub input: String,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let input = opts.input.as_str();

    let inp = File::open(input)?;
    let rdr = BufReader::new(inp);

    // Early exit if input file is empty.
    //
    let attr = fs::metadata(input)?;
    if attr.len() == 0 {
        eprintln!("{input} is empty.");
        return Err(eyre::eyre!("Input file is empty"));
    }

    let mut df = JsonReader::new(rdr)
        .with_json_format(JsonFormat::JsonLines)
        .infer_schema_len(NonZeroUsize::new(10))
        .finish()?;

    let output = Path::new(input).file_stem().unwrap().to_str().unwrap();
    let output = if opts.parquet {
        Path::new(output).with_extension("parquet")
    } else {
        Path::new(output).with_extension("csv")
    };

    let mut out = File::create(&output)?;
    if opts.parquet {
        let zstdlevel = ZstdLevel::try_new(8)?;
        let _ = ParquetWriter::new(&mut out)
            .with_compression(ParquetCompression::Zstd(Some(zstdlevel)))
            .with_statistics(StatisticsOptions::default())
            .finish(&mut df)?;
    } else {
        CsvWriter::new(&mut out)
            .include_header(true)
            .with_quote_style(QuoteStyle::Necessary)
            .finish(&mut df)?
    }
    eprintln!("\n{input} converted to {output:?}.");
    Ok(())
}
