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

fn main() -> eyre::Result<()> {
    stdin().lines().for_each(|l| {
        let text = match l {
            Ok(text) => text,
            Err(_) => return,
        };
        let params: Vec<&str> = text.split_whitespace().collect();
        let ts = params[0].parse::<u32>().unwrap();
        let timezone = params[1].parse::<String>().unwrap_or("Europe/Paris".into());

        let ts = Timestamp::from_second(ts as i64).unwrap().in_tz(&timezone).unwrap();
        println!("{}", ts.time())
    });
    Ok(())
}
