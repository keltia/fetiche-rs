//! This is the Rust equivalent of [import-adsb.py] with batching capabilities
//!

use std::fs;

mod adsb;
mod cli;
mod config;
mod error;
mod query;
mod runtime;

pub use adsb::*;
pub use cli::*;
pub use error::*;
pub use query::*;
pub use runtime::*;

use clap::Parser;
use eyre::Result;
use tracing::{debug, trace};

pub const NAME: &str = "import-data";

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let ctx = init_runtime(&opts).await?;

    let files = get_list(&opts)?;

    // We have a list of at least one file
    //
    for file in files {
        let n = match process_one(&ctx, &file, &opts.table).await {
            Ok(n) => n,
            Err(e) => {
                debug!("ignored={}, error={}", file, e.to_string());
                continue;
            }
        };
        trace!("loaded rows={} fname={}", n, file);
    }

    Ok(())
}

// -----

#[tracing::instrument]
pub fn get_list(opts: &Opts) -> Result<Vec<String>> {
    // Do we have a directory?
    //
    let name = opts.fname.clone();
    let st = fs::metadata(&name)?;
    let files = if st.is_dir() {
        let files = fs::read_dir(&name)?;

        let list = files.fold(vec![], |mut acc, f| {
            let file = match f {
                Ok(f) => f.file_name().to_string_lossy().to_string(),
                Err(e) => {
                    debug!("ignored file due to error: {}", e);
                    return acc;
                }
            };
            acc.push(file);
            acc
        });
        list
    } else {
        vec![name.clone()]
    };
    Ok(files)
}

#[tracing::instrument(skip(ctx))]
pub async fn process_one(ctx: &Context, fname: &str, table: &str) -> Result<usize> {
    // First analyse the filename, existence, etc.
    //
    if !fs::exists(fname)? {
        return Err(CmdError::UnknownFile(fname.into()).into());
    }

    // Read File
    //
    let rows = import_one_adsb(&ctx, fname, table).await?;
    trace!("loaded rows={} fname={}", rows, fname);

    Ok(rows)
}

