//! Clickhouse UDF executable that converts UTC Unix timestamp to local time string
//! based on provided timezone.
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
//!

use jiff::Timestamp;
use log::{error, info};
use std::io::stdin;
use stderrlog::LogLevelNum::Trace;

fn main() -> eyre::Result<()> {
    info!("Starting compute-localtime");

    stderrlog::new()
        .verbosity(Trace)
        .init()?;

    stdin().lines().for_each(|l| {
        let text = match l {
            Ok(text) => text,
            Err(e) => {
                error!("WARNING: could not read from stdin");
                return;
            }
        };
        let params: Vec<&str> = text.split_whitespace().collect();
        let ts = params[0].parse::<i64>().unwrap_or(0);
        let ts = match Timestamp::from_second(ts) {
            Ok(ts) => ts,
            Err(e) => {
                error!("ERROR: {}", e.to_string());
                return;
            }
        };
        let timezone = params.get(1).unwrap_or(&"Europe/Paris");

        let ts = match ts.in_tz(timezone) {
            Ok(ts) => ts,
            Err(e) => {
                error!("ERROR: {}", e.to_string());
                return;
            }
        };
        println!("{}", ts.time())
    });
    Ok(())
}
