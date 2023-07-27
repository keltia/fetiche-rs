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

// Algorithm for finding which segments are interesting otherwise Impala takes forever to
// retrieve data
//
// All timestamps are UNIX-epoch kind of timestamp.
//
// start = NNNNNN
// stop = MMMMMM
//
// i(0) => beg_hour = NNNNNN
// i(N) => end_hour = MMMMMM - (MMMMMM mod 3600)
//
// N =  (MMMMMM - NNNNNN) / 3600
//
// thus
//
// [beg_hour <= start] ... [end_hour <= stop]
// i(0)                ... i(N)
//
// N requests
//

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Parser;
use inline_python::{python, Context};
use tracing::trace;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use crate::cli::Opts;
use crate::location::{list_locations, load_locations};

mod cli;
mod location;

/// Calculate the list of 1h segments necessary for a given time interval
///
#[tracing::instrument]
pub fn extract_segments(start: i32, stop: i32) -> Result<Vec<i32>> {
    let beg_hour = start - (start % 3600);
    let end_hour = stop - (stop % 3600);

    let mut v = vec![];
    let mut i = beg_hour;
    while i <= end_hour {
        v.push(i);
        i += 3600;
    }
    Ok(v)
}

// Belfast airport is 54.7 N, 6.2 E (aka -6.2)
// We want +- 25nm around it
//
/// Belfast bounding box
//const BELFAST: [f32; 4] = [54.3, -5.8, 55.1, -6.6];

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

fn main() -> Result<()> {
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

    let loc: BTreeMap<String, Location> = load_locations(opts.config)?;

    // List loaded locations if nothing is specified, neither name nor location
    //
    if opts.lat.is_none() && opts.lon.is_none() && opts.name.is_none() {
        let str = list_locations(&loc)?;
        eprintln!("Locations:\n{}", str);
        std::process::exit(1);
    }

    // Get arguments, add hours ourselves as we do not care about them.
    //
    let start = match opts.start {
        Some(start) => start,
        None => {
            let now: DateTime<Utc> = Utc::now();
            now.format("%Y-%m-%d").to_string()
        }
    } + "00:00:00";
    trace!("start={}", start);

    let end = match opts.end {
        Some(end) => end,
        None => {
            let now: DateTime<Utc> = Utc::now();
            now.format("%Y-%m-%d").to_string()
        }
    } + "00:00:00";
    trace!("end={}", end);

    // Convert into UNIX timestamps
    //
    let start = NaiveDateTime::parse_from_str(&start, "%Y-%m-%d %H:%M:%S")?;
    let start = DateTime::<Utc>::from_utc(start, Utc).timestamp();

    let end = NaiveDateTime::parse_from_str(&end, "%Y-%m-%d %H:%M:%S")?;
    let end = DateTime::<Utc>::from_utc(end, Utc).timestamp();

    println!("From: {} To: {}", start, end);

    // We need to calculate the exact shard the data we want is into, otherwise the query will
    // take hours scanning all shards.
    //
    let v = extract_segments(start as i32, end as i32)?;
    println!("{} segments", v.len());
    println!("{:?}", v);

    let bb = BELFAST;

    // Initialise our embedded Python environment
    //
    let v1 = v.clone();
    let ctx: Context = python! {
        from pyopensky import OpenskyImpalaWrapper

        opensky = OpenskyImpalaWrapper()

        print("From: ", 'start, "To: ", 'end, "BB=", 'bb)
        print("Segments: ", len('v1))
    };

    // Now for each segment, use the python code to fetch and return the DataFrames in CSV format
    //
    let data: Vec<_> = v
        .iter()
        .inspect(|tm| trace!("Fetching segment {}", tm))
        .map(|tm| {
            ctx.run(python! {
                seg = 'tm
                bb = 'bb
                q = "SELECT * FROM state_vectors_data4 \
                WHERE lat >= {} AND lat <= {} AND lon >= {} AND lon <= {} AND hour={};\
                ".format(bb[0], bb[2], bb[3], bb[1], seg)

                df = opensky.rawquery(q)
                data = df.to_csv()
            });
            ctx.get::<String>("data")
        })
        .collect();

    // End of the Python part thanks $DEITY! (and @m_ou_se on Twitter)
    //
    // data is a Vec<String> with each component a CSV string
    //

    println!("final array of csv:\n{:?}", data);

    Ok(())
}

/// Return our version number
///
#[inline]
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}

/// Display banner
///
fn banner() -> Result<()> {
    Ok(eprintln!(
        r##"
{}/{} by {}
{}
"##,
        NAME,
        VERSION,
        AUTHORS,
        crate_description!()
    ))
}
