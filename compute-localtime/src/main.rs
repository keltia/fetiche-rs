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
use std::io::stdin;
use tracing::error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() -> eyre::Result<()> {
    let filter = EnvFilter::from_default_env();

    // Log to file?
    //
    let file_appender = tracing_appender::rolling::hourly("/tmp/compute-localtime", "");
    let fs = tracing_subscriber::fmt::layer().with_writer(file_appender);

    // Combine filters & exporters
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(fs)
        .init();

    
    stdin().lines().for_each(|l| {
        let text = match l {
            Ok(text) => text,
            Err(_) => {
                error!("WARNING: could not read from stdin");
                return
            },
        };
        let params: Vec<&str> = text.split_whitespace().collect();
        let ts = params[0].parse::<i64>().unwrap_or(0);
        let ts = match Timestamp::from_second(ts) {
            Ok(ts) => ts,
            Err(e) => {
                error!("ERROR: {}", e.to_string());
                return;
            },
        };
        let timezone = params[1].parse::<String>().unwrap_or("Europe/Paris".into());

        let ts = ts.in_tz(&timezone).unwrap();
        println!("{}", ts.time())
    });
    Ok(())
}
