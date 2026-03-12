mod clean;
mod fetch;
mod show;

pub use clean::*;
pub use fetch::*;
pub use show::*;

use std::fmt::Debug;
use std::fs::File;
use std::path::Path;

use polars::prelude::{ParquetReader, SerReader};

#[tracing::instrument]
pub async fn read_parquet_size<P>(fname: P) -> eyre::Result<usize>
where
    P: AsRef<Path> + Debug,
{
    let fh = File::open(fname)?;
    let mut rdr = ParquetReader::new(fh);
    Ok(rdr.num_rows()?)
}

