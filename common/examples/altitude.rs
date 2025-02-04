//! An example program to calculate the geoid height using the EGM2008 model.
//!
//! This program demonstrates how to:
//! 1. Load a list of locations from a source file or default.
//! 2. Calculate the geoid height for each location using the `egm2008` crate.
//! 3. Compare stored altitudes with the calculated geoid-based altitude.
//!
//! # Usage
//!
//! Simply run the program to view the results for each location.
//!
//! ```sh
//! cargo run --example altitude
//! ```
//!
//! # Dependencies
//!
//! - `egm2008`: For geoid-height calculation based on the EGM2008 model.
//! - `fetiche_common`: For loading the locations.
//!
//! # Output
//!
//! For each location, the output will display:
//! - Name of the location.
//! - Stored reference altitude.
//! - EGM2008-provided geoid height.
//! - Computed geometric altitude (sum of reference altitude and geoid height).
//!
use egm2008::geoid_height;
use fetiche_common::load_locations;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let list = load_locations(None)?;
    list.iter().for_each(|(n, l)| {
        let diff_h = geoid_height(l.latitude as f32, l.longitude as f32).unwrap();
        println!("{}: stored altitude={:3} EGM2008={:4.0} goemetric altitude={:4.0}", n, l.ref_altitude, diff_h, l.ref_altitude as f32 + diff_h);
    });

    Ok(())
}