//! Simple `FusedData` to `DronePoint` streaming converter.
//!
//! Debug:
//! ```text
//! ❯ hyperfine --warmup 3 -i  "..\target\debug\examples\convert.exe ..\data\all-senhive.json"
//! Benchmark 1: ..\target\debug\examples\convert.exe ..\data\all-senhive.json
//!   Time (mean ± σ):      3.437 s ±  0.053 s    [User: 3.286 s, System: 0.094 s]
//!   Range (min … max):    3.372 s …  3.550 s    10 runs//! ```
//!
//! Release:
//! ```text
//! ❯ hyperfine --warmup 3 -i  "..\target\release\examples\convert.exe ..\data\all-senhive.json"
//! Benchmark 1: ..\target\release\examples\convert.exe ..\data\all-senhive.json
//!   Time (mean ± σ):     333.0 ms ±   2.7 ms    [User: 273.1 ms, System: 45.3 ms]
//!   Range (min … max):   328.0 ms … 335.4 ms    10 runs//! ```
//!

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use csv::QuoteStyle;
use eyre::Result;
use fetiche_formats::senhive::{DronePoint, FusedData};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// CLI options
#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    input: String,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    let input = opts.input.as_str();

    let inp = File::open(input)?;
    let rdr = BufReader::new(inp);

    let output = Path::new(input).file_stem().unwrap().to_str().unwrap();
    let output = Path::new(output).with_extension("csv");

    let out = File::create(&output)?;
    let mut wtr = csv::WriterBuilder::new()
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(out);

    let progress = ProgressBar::no_length().with_style(ProgressStyle::with_template(
        "Converting [{elapsed_precise}]: {human_pos} -- {per_sec}",
    )?);

    let data = rdr.lines();

    progress.wrap_iter(data).for_each(|r| {
        let r: FusedData = serde_json::from_str(&r.unwrap()).unwrap();
        let r: DronePoint = (&r).into();
        wtr.serialize(r).unwrap();
    });
    progress.finish();
    wtr.flush()?;

    eprintln!("\n{input} converted to {output:?}.");
    Ok(())
}
