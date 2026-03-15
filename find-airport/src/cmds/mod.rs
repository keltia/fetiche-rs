mod clean;
mod fetch;
mod find;
mod show;

pub use clean::*;
pub use fetch::*;
pub use find::*;
pub use show::*;

use std::fmt::{Debug, Display};
use std::fs::File;
use std::path::Path;
use std::time::UNIX_EPOCH;

use jiff::Timestamp;
use polars::prelude::{ParquetReader, SerReader};
use strum::VariantNames;
use tabled::Tabled;

#[tracing::instrument]
pub async fn read_parquet_size<P>(fname: P) -> eyre::Result<usize>
where
    P: AsRef<Path> + Debug,
{
    let fh = File::open(fname)?;
    let mut rdr = ParquetReader::new(fh);
    Ok(rdr.num_rows()?)
}

#[derive(Clone, Debug, Default, strum::Display, VariantNames)]
pub enum WorkStatus {
    Present,
    Refreshed,
    Removed,
    #[default]
    Unknown,
}

/// `Work` describe a file that was present, fetched, or refreshed
#[derive(Clone, Debug, Tabled)]
pub struct Work {
    #[tabled(rename = "Status")]
    status: WorkStatus,
    #[tabled(rename = "Filename")]
    name: String,
    mtime: Timestamp,
    #[tabled(rename = "Size")]
    size: u64,
    #[tabled(rename = "# Rows")]
    rows: usize,
}

impl Display for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mtime = self.mtime.strftime("%Y-%m-%d %H:%M:%S").to_string();
        write!(
            f,
            "File {{ status: {:?}, name: {:?}, mtime: {}, size: {:?}, rows: {:?} }}",
            self.status, self.name, mtime, self.size, self.rows
        )
    }
}

impl Default for Work {
    fn default() -> Self {
        Self {
            status: WorkStatus::Unknown,
            name: "".to_string(),
            mtime: Timestamp::try_from(UNIX_EPOCH).unwrap(),
            size: 0,
            rows: 0,
        }
    }
}
