//! This is the module using `datafusion` to read csv files and generate parquet files.
//!
//! This is way faster than `arrow2` (see benches/csv-to-parquet.rs) for larger datasets
//!

use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::parquet::basic::{Compression, Encoding, ZstdLevel};
use datafusion::parquet::file::properties::{EnabledStatistics, WriterProperties};
use datafusion::prelude::*;
use eyre::Result;

use crate::Options;

/// Async reading and writing
///
#[tracing::instrument]
pub async fn parquet_through_df(fname: &str, output: &str, opts: Options) -> Result<()> {
    // nh = no header line (default = false which means has header line).
    //
    let header = opts.header;
    let delim = opts.delim;

    // Setup datafusion for csv files
    //
    let ctx = SessionContext::new();
    let copts = CsvReadOptions::new().delimiter(delim).has_header(header);

    // Do the reading
    //
    let df = ctx.read_csv(fname, copts).await?;

    // Setup our options, both for dataframe and file
    //
    let dopts = DataFrameWriteOptions::default().with_single_file_output(true);
    let props = WriterProperties::builder()
        .set_created_by("bench_df".to_string())
        .set_encoding(Encoding::PLAIN)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_compression(Compression::ZSTD(ZstdLevel::try_new(8)?))
        .build();

    // Do the writing
    //
    let _ = df.write_parquet(output, dopts, Some(props)).await?;

    Ok(())
}
