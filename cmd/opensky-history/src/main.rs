//! This program does retrieval of historical data from Opensky.
//!
//! Author: Ollivier Robert <ollivier.robert@eurocontrol.int> for the ACUTE project.
//!
//! It uses an embedded python script through the [inline-python] crate.
//! The script use [pyopensky] to connect to the [Impala Shell] at Opensky.
//!
//! XXX: "nightly" toolchain is mandatory for this.
//!
//! [inline-python]: https://crates.io/crates/inline-python
//! [pyopensky]: https://pypi.org/project/pyopensky/
//! [Impala Shell]: https://opensky-network.org/data/impala
//!

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;

use chrono::prelude::*;
use clap::{crate_authors, crate_version, Parser};
use eyre::{eyre, Result};
use inline_python::{python, Context};
use polars::io::SerReader;
use polars::prelude::{CsvParseOptions, CsvReadOptions, ParquetWriter};
use tracing::{info, trace};

use fetiche_common::{init_logging, list_locations, load_locations, Location, BB};

use crate::cli::{banner, version, Opts, Otype};
use crate::segment::extract_segments;

mod cli;
mod segment;

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<()> {
    trace!("enter");

    let opts = Opts::parse();

    // Initialise logging.
    //
    init_logging(NAME, false, false, None)?;

    // Banner
    //
    banner()?;

    if opts.version {
        eprintln!("{}", version());
        std::process::exit(0);
    }

    trace!("read locations");
    let loc: BTreeMap<String, Location> = load_locations(opts.config)?;

    trace!("parse arguments");

    // List loaded locations if nothing is specified, neither name nor location
    //
    let site = match opts.name {
        Some(name) => name,
        None => {
            let dist = opts.range;
            let str = list_locations(&loc, dist)?;
            eprintln!("{}", str);
            return Ok(());
        }
    };

    // Get arguments, parse anything as a date
    //
    let start = match opts.begin {
        Some(start) => dateparser::parse(&start),
        None => Ok(Utc::now()),
    }
    .unwrap();
    trace!("start={}", start);

    let end = match opts.end {
        Some(end) => dateparser::parse(&end),
        None => Ok(Utc::now()),
    }
    .unwrap();
    trace!("end={}", end);

    // Convert into UNIX timestamps
    //
    let start = start.timestamp() as i32;
    let end = end.timestamp() as i32;

    println!("From: {} To: {}", start, end);

    // We need to calculate the exact shard the data we want is into, otherwise the query will
    // take hours scanning all shards.
    //
    trace!("calculate segments");

    let v = extract_segments(start, end)?;
    info!("{} segments", v.len());
    trace!("{:?}", v);

    let bb = match loc.get(&site) {
        Some(loc) => loc,
        None => return Err(eyre!("You must specify a location")),
    };

    // If the --icao option is specified, add the parameter to the query string.
    //
    let _icao = if opts.icao.is_some() {
        format!(" AND CALLSIGN = '{}'", opts.icao.unwrap())
    } else {
        String::new()
    };

    // Default range is 25 nm
    //
    let bb = BB::from_location(bb, opts.range);
    let bb = [bb.min_lon, bb.min_lat, bb.max_lon, bb.max_lat];
    trace!("BB={:?}", bb);
    // Initialise our embedded Python environment
    //
    trace!("initialise python");

    let v1 = v.clone();
    let ctx: Context = python! {
        from pyopensky.impala import Impala

        impala = Impala()

        print("From: ", 'start, "To: ", 'end, "BB=", 'bb)
        print("Segments: ", len('v1))
        start = 'start
        end = 'end
        bb = 'bb

        df = impala.history(start, end, bounds=bb)
        if df is None:
            data = ""
        else:
            data = df.to_csv()
    };
    let data = ctx.get::<String>("data");

    // End of the Python part thanks $DEITY! (and @m_ou_se on Twitter)
    //
    trace!("data={}", &data);

    let output = opts.output;

    if opts.otype == Otype::Parquet {
        let cur = Cursor::new(data);

        let opts = CsvParseOptions::default().with_try_parse_dates(false);
        let mut df = CsvReadOptions::default()
            .with_has_header(true)
            .with_parse_options(opts)
            .into_reader_with_file_handle(cur)
            .finish()?;

        info!("writing {}", output);
        let mut file = fs::File::create(output)?;

        ParquetWriter::new(&mut file).finish(&mut df)?;
    } else {
        fs::write(output, data)?;
    }
    Ok(())
}
