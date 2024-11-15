//! Read some data as json and write it into a parquet file
//!

use datafusion::prelude::*;
use datafusion::{config::TableParquetOptions, dataframe::DataFrameWriteOptions};
use eyre::Result;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[tracing::instrument]
async fn read_write_output(base: &str) -> Result<()> {
    trace!("Read data.");

    let fname = format!("{}.json", base);
    trace!("fname={:?}", fname);

    let ctx = SessionContext::new();

    let df = ctx.read_json(&fname, NdJsonReadOptions::default()).await?;
    info!("{} records read", df.clone().count().await?);

    // Prepare output
    //
    let fname = format!("{}.parquet", base);

    let dfopts = DataFrameWriteOptions::default().with_single_file_output(true);

    let mut options = TableParquetOptions::default();
    options.global.created_by = "acutectl/save".to_string();
    options.global.writer_version = "2.0".to_string();
    options.global.encoding = Some("plain".to_string());
    options.global.statistics_enabled = Some("page".to_string());
    options.global.compression = Some("zstd(8)".to_string());

    info!("Writing in {}", fname);
    let _ = df.write_parquet(&fname, dfopts, Some(options)).await?;

    info!("Done.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_span_retrace(true)
        .with_targets(true)
        .with_verbose_entry(true)
        .with_verbose_exit(true)
        .with_bracketed_fields(true);

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .init();
    trace!("Logging initialised.");

    let fname = std::env::args().nth(1).ok_or("small").unwrap();

    let _ = read_write_output(&fname).await?;

    Ok(())
}
