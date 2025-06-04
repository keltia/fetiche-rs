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
//!                 <name>compute_localtime</name>
//!                 <return_type>String</return_type>
//!                 <argument>
//!                         <type>Datetime</type>
//!                         <name>ts</name>
//!                 </argument>
//!                 <argument>
//!                         <type>String</type>
//!                         <name>timezone</name>
//!                 </argument>
//!                 <format>TabSeparated</format>
//!                 <command>compute-localtime</command>
//!         </function>
//! </functions>
//! ```
//!
//! # Timing
//!
//! ```text
//! ```
//!

use clap::Parser;
use egm2008::geoid_height;
use std::io::stdin;

/// Command-line options for computing geoid height.
///
/// # Fields
/// - `verbose` or `-v`: Enables detailed output.
///
#[derive(Debug, Parser)]
pub struct Opts;

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
            eprintln!(
                "Variation aka geoid height at {},{} = {} m",
                lat, lon, height
            );
        } else {
            println!("{}", height);
        }
    });
    Ok(())
}
