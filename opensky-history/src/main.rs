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

use chrono::prelude::*;
use clap::{crate_authors, crate_version, Parser};
use eyre::{eyre, Result};
use inline_python::{python, Context};
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use fetiche_formats::Format;

use crate::cli::{banner, Opts};
use crate::location::{list_locations, load_locations, Location, BB};
use crate::segment::{extract_segments, read_segment};

mod cli;
mod location;
mod segment;

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

#[tracing::instrument]
fn main() -> Result<()> {
    trace!("enter");

    let opts = Opts::parse();

    // Initialise logging.
    //
    let fmt = fmt::layer().with_target(false).compact();

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry().with(filter).with(fmt).init();

    // Banner
    //
    banner()?;

    trace!("read locations");
    let loc: BTreeMap<String, Location> = load_locations(opts.config)?;

    trace!("parse arguments");

    // List loaded locations if nothing is specified, neither name nor location
    //
    if opts.name.is_none() {
        let dist = opts.range;
        let str = list_locations(&loc, dist)?;
        eprintln!("{}", str);
        return Ok(());
    }

    // Get arguments, parse anything as a date
    //
    let start = match opts.start {
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

    let bb = match opts.name {
        Some(name) => match loc.get(&name) {
            Some(loc) => loc,
            None => return Err(eyre!("Unknown location")),
        },
        None => return Err(eyre!("You must specify a location")),
    };

    // If the --icao option is specified, add the parameter to the query string.
    //
    let icao = if opts.icao.is_some() {
        format!(" AND CALLSIGN = '{}'", opts.icao.unwrap())
    } else {
        String::new()
    };

    // Default range is 25 nm
    //
    let bb = BB::from_location(bb, opts.range);
    let bb = [bb.min_lon, bb.min_lat, bb.max_lon, bb.max_lat];

    // Initialise our embedded Python environment
    //
    trace!("initialise python");

    let v1 = v.clone();
    let ctx: Context = python! {
        from pyopensky.impala import Impala

        impala = Impala()

        print("From: ", 'start, "To: ", 'end, "BB=", 'bb)
        print("Segments: ", len('v1))
    };

    // Now for each segment, use the python code to fetch and return the DataFrames in CSV format
    //
    trace!("fetch segments");

    let mut p = progress::Bar::new();
    p.set_job_title("Fetching segments");

    let step = 100 / v.len() as i32;

    let data: Vec<_> = v
        .iter()
        .inspect(|&tm| trace!("Fetching segment {}", tm))
        .map(|&tm| {
            let icao = icao.clone();
            ctx.run(python! {
                seg = 'tm
                bb = 'bb
                q = "SELECT * FROM state_vectors_data4 \
                WHERE lat >= {} AND lat <= {} AND lon >= {} AND lon <= {} AND hour={}{};\
                ".format(bb[1], bb[3], bb[0], bb[2], seg, 'icao)

                df = impala.history()
                if df is None:
                    data = ""
                else:
                    data = df.to_csv()
            });
            p.add_percent(step);
            ctx.get::<String>("data")
        })
        .collect();
    p.reach_percent(100);
    p.jobs_done();

    // End of the Python part thanks $DEITY! (and @m_ou_se on Twitter)
    //
    let format = Format::PandaStateVector;

    trace!("now merging {} csv segments", data.len());

    #[cfg(feature = "arrow2")]
    let data: Vec<_> = data
        .iter()
        .map(|seg| {
            let data = read_segment(seg).unwrap();
            data
        })
        .collect();

    dbg!(&data);

    #[cfg(not(feature = "arrow2"))]
    let data = {
        // data is a Vec<String> with each component a CSV "file"
        //
        let mut p = progress::Bar::new();
        p.set_job_title("Merging csv");
        let step = 100 / data.len() as i32;

        let data: Vec<Cat21> = data
            .iter()
            .flat_map(|seg| {
                let mut rdr = ReaderBuilder::new()
                    .flexible(true)
                    .has_headers(true)
                    .from_reader(seg.as_bytes());
                p.add_percent(step);
                format.from_csv(&mut rdr).unwrap()
            })
            .collect();
        p.reach_percent(100);
        p.jobs_done();

        let data = prepare_csv(data, true)?;
        data
    };
    // Manage output
    //
    match opts.output {
        Some(output) => fs::write(output, data)?,
        _ => println!("{:?}", data),
    }

    Ok(())
}
