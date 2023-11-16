//! Read some data as json and write it into a parquet file
//!

use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::parquet::basic::{Compression::ZSTD, ZstdLevel};
use datafusion::prelude::*;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use fetiche_formats::Asd;

#[tracing::instrument]
async fn read_write_data(base: &str) -> eyre::Result<()> {
    trace!("Read data.");
    let ctx = SessionContext::new();

    let first = format!("{}.json", base);

    let data = tokio::fs::read_to_string(&first).await?;
    let data: Vec<Asd> = serde_json::from_str(&data)?;

    let data = ctx.read_json(&first, NdJsonReadOptions::default()).await?;
    trace!("schema={}", data.schema().to_string());

    trace!("Decode data.");
    let data = data.select_columns(&["timestamp", "latitude", "longitude"])?;

    info!("{} records read", data.clone().count().await?);

    // Prepare output
    //
    let props = datafusion::parquet::file::properties::WriterProperties::builder()
        .set_created_by("fetiche".to_string())
        .set_compression(ZSTD(ZstdLevel::default()))
        .build();

    let dfopts = DataFrameWriteOptions::new().with_single_file_output(true);

    info!("Writing in {}", OUTPUT);
    let r = data.write_parquet(OUTPUT, dfopts, Some(props)).await?;
    trace!("Done.");
    trace!("r={:?}", r);

    Ok(())
}

const INPUT: &str = "small";
const OUTPUT: &str = "small.parquet";

#[tokio::main]
async fn main() -> eyre::Result<()> {
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
        .with_service_name(OUTPUT)
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

    let _ = read_write_data(INPUT).await?;

    // Transform into our own format
    //

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
