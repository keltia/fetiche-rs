//! This program does retrieval of historical data from Opensky.
//!
//! Author: Ollivier Robert <ollivier.robert@eurocontrol.int> for the ACUTE project.
//!
//! It uses an embedded python script through the [inline-python] crate.
//! The script use [pyopensky] to connect to the [Impala Shell] at Opensky.
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
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use inline_python::{python, Context};

/// Calculate the list of 1h segments necessary for a given time interval
///
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

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Start date (YYYY-MM-DD).
    pub start: String,
    /// End date (YYYY-MM-DD).
    pub end: String,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    // Get arguments, add hours ourselves as we do not care about them.
    //
    let start = opts.start + "00:00:00";
    let end = opts.end + "00:00:00";

    // Convert into UNIX timestamps
    //
    let start = NaiveDateTime::parse_from_str(&start, "%Y-%m-%d %H:%M:%S")?;
    let start = DateTime::<Utc>::from_utc(start, Utc).timestamp();

    let end = NaiveDateTime::parse_from_str(&end, "%Y-%m-%d %H:%M:%S")?;
    let end = DateTime::<Utc>::from_utc(end, Utc).timestamp();

    println!("From: {} To: {}", start, end);

    let v = extract_segments(start as i32, end as i32)?;
    println!("{} segments", v.len());
    println!("{:?}", v);

    // We fetch data through this embedded python script
    //
    let ctx = Context::new();
    ctx.run(python! {
        from pyopensky import OpenskyImpalaWrapper

        opensky = OpenskyImpalaWrapper()
        print(opensky)

        print("From: ", 'start, "To: ", 'end)
    });

    Ok(())
}
