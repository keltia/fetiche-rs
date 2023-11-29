//! Read some data as json and write it into a parquet file
//!

use datafusion::prelude::*;
use datafusion::{
    dataframe::DataFrameWriteOptions,
    parquet::{
        basic::{Compression, Encoding, ZstdLevel},
        file::properties::{EnabledStatistics, WriterProperties},
    },
};

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

    let opts = DataFrameWriteOptions::default();

    let props = WriterProperties::builder()
        .set_created_by(NAME.to_string())
        .set_encoding(Encoding::PLAIN)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    info!("Writing in {}", fname);
    let res = df.write_parquet(&fname, opts, Some(props)).await?;

    let count = res.iter().fold(0, |cnt, e| cnt + e.num_columns());
    info!("Done, {} records written.", count);
    Ok(())
}

const NAME: &str = "parquet";

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
        .with_higher_precision(true)
        .with_bracketed_fields(true);

    // Setup Open Telemetry with Jaeger
    //
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_auto_split_batch(true)
        .with_max_packet_size(9_216)
        .with_service_name(NAME)
        .install_simple()?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .with(telemetry)
        .init();
    trace!("Logging initialised.");

    let fname = std::env::args().nth(1).ok_or("small").unwrap();

    let _ = read_write_output(&fname).await?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
