//! Clickhouse UDF executable that converts UTC Unix timestamp to local date/time string
//! based on the provided timezone.
//!
//! # Installation
//!
//! This needs to be installed on the Clickhouse server in
//! `/db/clickhouse/user_scripts` for our installation.
//!
//! It needs to be referenced inside an XML file, here in `/etc/clickhouse-server/udf`.
//!
//! For now, it can be run as either `compute-localdate` or `compute-localtime`.
//!
//! ```xml
//! <functions>
//!         <function>
//!                 <type>executable</type>
//!                 <name>compute_localtime</name>
//!                 <return_type>String</return_type>
//!                 <argument>
//!                         <type>UInt32</type>
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
//! or
//! ```xml
//! <functions>
//!         <function>
//!                 <type>executable</type>
//!                 <name>compute_localdate</name>
//!                 <return_type>String</return_type>
//!                 <argument>
//!                         <type>UInt32</type>
//!                         <name>ts</name>
//!                 </argument>
//!                 <argument>
//!                         <type>String</type>
//!                         <name>timezone</name>
//!                 </argument>
//!                 <format>TabSeparated</format>
//!                 <command>compute-localtime -d</command>
//!         </function>
//! </functions>
//! ```

use clap::Parser;
use jiff::{Timestamp, Zoned};
use log::{error, info};
use std::io::stdin;
use stderrlog::LogLevelNum::Trace;

#[derive(Debug, Parser)]
struct Opts {
    #[clap(short = 'd', long)]
    date: bool,
}

fn main() -> eyre::Result<()> {
    info!("Starting compute-localtime/date");

    let opts = Opts::parse();

    let name = std::env::current_exe()?;
    let name = name.file_stem().unwrap().to_str().unwrap();

    let is_date_mode = name == "compute-localdate" || opts.date;
    info!(
        "Running in {} mode",
        if is_date_mode { "date" } else { "time" }
    );

    let output = if is_date_mode {
        |p: Zoned| p.date().to_string()
    } else {
        |p: Zoned| p.time().to_string()
    };

    stderrlog::new().verbosity(Trace).init()?;

    stdin().lines().for_each(|l| {
        let text = match l {
            Ok(text) => text,
            Err(e) => {
                error!("WARNING: could not read from stdin: {e}");
                return;
            }
        };
        let params: Vec<&str> = text.split_whitespace().collect();
        let ts = params[0].parse::<i64>().unwrap_or(0);
        let ts = match Timestamp::from_second(ts) {
            Ok(ts) => ts,
            Err(e) => {
                error!("ERROR: {e}");
                return;
            }
        };
        let timezone = params.get(1).unwrap_or(&"Europe/Paris");

        let ts = match ts.in_tz(timezone) {
            Ok(ts) => ts,
            Err(e) => {
                error!("ERROR: {e}");
                return;
            }
        };
        println!("{}", output(ts));
    });
    Ok(())
}
