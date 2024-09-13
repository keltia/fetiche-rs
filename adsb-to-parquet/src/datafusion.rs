//! This is the module using `datafusion` to read csv files and generate parquet files.
//!
//! This is way faster than `arrow2` (see benches/csv-to-parquet.rs) for larger datasets
//!

use datafusion::config::TableParquetOptions;
use datafusion::dataframe::DataFrameWriteOptions;
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

    // Do the reading
    //
    let df = ctx
        .read_csv(
            fname,
            CsvReadOptions::default()
                .delimiter(delim)
                .has_header(header),
        )
        .await?;

    let fname = "../data/test-df.parquet";

    let dfopts = DataFrameWriteOptions::default().with_single_file_output(true);

    let mut options = TableParquetOptions::default();
    options.global.created_by = "bench_polars".to_string();
    options.global.writer_version = "2.0".to_string();
    options.global.encoding = Some("plain".to_string());
    options.global.statistics_enabled = Some("page".to_string());
    options.global.compression = Some("zstd(8)".to_string());

    let _ = df.write_parquet(fname, dfopts, Some(options)).await?;

    Ok(())
}
