//! # Geoid Height Calculator
//!
//! This program computes the geoid height for a given latitude and longitude
//! using the EGM2008 model. It leverages the `egm2008` crate to perform the
//! calculations. The program can be used via the command-line interface (CLI).
//!
//! ## Features
//!
//! - Computes geoid height for a specified location.
//! - Provides optional verbose mode, displaying detailed program information.
//!
//! ## Usage
//!
//! Ensure you have the required dependencies installed (`clap`, `egm2008`, etc.).
//!
//! ### Example Usage
//!
//! ```bash
//! # Basic usage
//! compute-height 40.7128 -74.0060
//!
//! # Verbose output
//! compute-height -v 40.7128 -74.0060
//! ```
//!
//! The output will be the geoid height in meters for the specified coordinates.
//!
//! ## Notes
//!
//! - Latitude should be between -90 and 90 degrees.
//! - Longitude should be between -180 and 180 degrees.
//! - Verbose mode (`--verbose` or `-v`) outputs program metadata (such as name, version, authors).
//!

use clap::Parser;
use egm2008::geoid_height;
use std::io::stdin;

/// Program name.
const NAME: &str = env!("CARGO_PKG_NAME");
/// Program version.
const VERSION: &str = env!("CARGO_PKG_VERSION");

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

    let text = stdin().lines().next().unwrap()?;
    let coords: Vec<&str> = text.split_whitespace().collect();
    let lat = coords[0].parse::<f32>()?;
    let lon = coords[1].parse::<f32>()?;

    if opts.verbose {
        eprintln!("{}", banner());
    }

    let height = geoid_height(lat, lon)?;
    if opts.verbose {
        println!("Variation aka geoid height at {},{} = {} m", lat, lon, height);
    } else {
        println!("{}", height);
    }
    Ok(())
}

// -----

fn banner() -> String {
    format!(
        "{} v{} - {}\n{}\n",
        NAME,
        VERSION,
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_DESCRIPTION")
    )
}