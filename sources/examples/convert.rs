//! Simple `FusedData` to `DronePoint` streaming converter.
//!
//! Debug:
//! ```text
//! ❯ hyperfine --warmup 3 -i  "..\target\debug\examples\convert.exe ..\data\all-senhive.json"
//! Benchmark 1: ..\target\debug\examples\convert.exe ..\data\all-senhive.json
//!   Time (mean ± σ):      3.522 s ±  0.138 s    [User: 3.160 s, System: 0.112 s]
//!   Range (min … max):    3.438 s …  3.907 s    10 runs
//! ```
//!
//! Release:
//! ```text
//! ❯ hyperfine --warmup 3 -i  "..\target\release\examples\convert.exe ..\data\all-senhive.json"
//! Benchmark 1: ..\target\release\examples\convert.exe ..\data\all-senhive.json
//!   Time (mean ± σ):     363.8 ms ±   6.0 ms    [User: 285.9 ms, System: 63.4 ms]
//!   Range (min … max):   355.8 ms … 372.8 ms    10 runs
//! ```
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

    let ind_read = ProgressBar::no_length().with_style(ProgressStyle::with_template(
        "[{elapsed_precise}: {human_pos} -- {per_sec}",
    )?);

    let data = rdr.lines();

    let data = ind_read
        .wrap_iter(data)
        .map(|r| {
            let r: FusedData = serde_json::from_str(&r.unwrap()).unwrap();
            let r: DronePoint = (&r).into();
            r
        })
        .collect::<Vec<_>>();

    let length = data.len();
    let output = Path::new(input).file_stem().unwrap().to_str().unwrap();
    let output = Path::new(output).with_extension("csv");

    let out = File::create(&output)?;
    let mut wtr = csv::WriterBuilder::new()
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(out);

    data.iter().for_each(|r| wtr.serialize(r).unwrap());
    wtr.flush()?;

    eprintln!("\n{input} converted to {output:?} with {length} lines");
    Ok(())
}
