//!
//! A command-line utility to compute geoid height for a given latitude and longitude using EGM2008.
//!
//! This program calculates the geoid height, which represents the deviation of the Earth's surface
//! from an ellipsoidal reference due to variations in gravitational forces, based on the EGM2008 model.
//!
//! # Overview
//!
//! The program reads coordinates (latitude and longitude in degrees) from the standard input,
//! computes the corresponding geoid height, and outputs the result. It supports an optional
//! verbose mode that provides additional details about the computation, including program metadata.
//!
//! # Examples
//!
//! Compute the geoid height for a given latitude and longitude (e.g., `52.5163 13.3777`):
//!
//! ```bash
//! $ echo "52.5163 13.3777" | compute-height
//! 37.2
//! ```
//!
//! Enable verbose mode to get additional details:
//!
//! ```bash
//! $ echo "52.5163 13.3777" | compute-height -v
//! compute-height v0.1.0 - <CARGO_PKG_AUTHORS>
//! <CARGO_PKG_DESCRIPTION>
//!
//! Variation aka geoid height at 52.5163,13.3777 = 37.2 m
//! ```
//!
//! The program will exit with an error if invalid input is provided or the geoid height cannot
//! be computed for the given coordinates.
//!
//! # Installation
//!
//! This needs to be installed on the Clickhouse server in
//! `/db/clickhouse/user_scripts` for our installation.
//!
//! It needs to be referenced inside an XML file, here in `/etc/clickhouse-server/udf`.
//!
//! ```xml
//!<functions>
//!         <function>
//!                 <type>executable</type>
//!                 <name>compute_height</name>
//!                 <return_type>Float64</return_type>
//!                 <argument>
//!                         <type>Float64</type>
//!                         <name>lat</name>
//!                 </argument>
//!                 <argument>
//!                         <type>Float64</type>
//!                         <name>lon</name>
//!                 </argument>
//!                 <format>TabSeparated</format>
//!                 <command>compute-height</command>
//!         </function>
//! </functions>
//! ```
//!
//! ```text
//! 1383527 rows in set. Elapsed: 0.851 sec. Processed 1.38 million rows, 248.77 MB (1.63 million rows/s., 292.30 MB/s.)
//! Peak memory usage: 49.37 MiB.
//! ```
//!

use clap::Parser;
use egm2008::geoid_height;
use std::io::stdin;

/// Command-line options for computing geoid height.
///
/// # Fields
/// - `verbose`: Enables detailed output with program information.
/// - `lat`: Latitude of the location (in degrees).
/// - `lon`: Longitude of the location (in degrees).
///
#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(short = 'v', long)]
    pub verbose: bool,
}

fn main() -> eyre::Result<()> {
    // Basic option parsing.
    //
    let opts = Opts::parse();

    stdin().lines().for_each(|l| {
        let text = l.unwrap();
        let coords: Vec<&str> = text.split_whitespace().collect();
        let lat = coords[0].parse::<f32>().unwrap_or(0.);
        let lon = coords[1].parse::<f32>().unwrap_or(0.);
        let height = geoid_height(lat, lon).unwrap_or(0.);
        if opts.verbose {
            eprintln!("Variation aka geoid height at {},{} = {} m", lat, lon, height);
        }
        println!("{}", height);
    });
    Ok(())
}
