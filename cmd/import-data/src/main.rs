//! This is the Rust equivalent of [import-adsb.py] with batching capabilities
//!

use std::fs::File;

mod cli;
mod config;
mod data;
mod error;
mod query;
mod runtime;

pub use cli::*;
pub use error::*;
pub use query::*;
pub use runtime::*;

use crate::data::import_adsb;
use clap::Parser;
use eyre::Result;
use klickhouse::Row;
use polars::prelude::{NamedFrom, SerReader};
use serde::{Deserialize, Serialize};

pub const NAME: &str = "import-data";

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let ctx = init_runtime(&opts).await?;

    let name = opts.fname.clone();

    let rows = import_adsb(&ctx, &opts).await?;

    //db.insert_native_block(format!("INSERT INTO {table} FORMAT native"), rows)
    //    .await?;

    let fhout = File::create("output.csv")?;
    let mut wtr = csv::Writer::from_writer(fhout);
    for row in rows.into_iter() {
        wtr.serialize(row)?;
    }
    wtr.flush()?;

    Ok(())
}
