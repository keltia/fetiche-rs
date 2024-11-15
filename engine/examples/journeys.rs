use datafusion::arrow::array::{Array, Int32Array, RecordBatch};
use datafusion::prelude::*;
use eyre::Result;
use tracing::trace;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_tree::HierarchicalLayer;

use fetiche_formats::Asd;

const NAME: &str = "journeys";

pub struct Journey {
    pub id: u32,
    pub points: Vec<Asd>,
}

async fn read_and_generate_journeys(fname: &str) -> Result<Vec<RecordBatch>> {
    trace!("Read data.");

    let fname = format!("{}.parquet", fname);
    trace!("fname={:?}", fname);

    let ctx = SessionContext::new();

    ctx.register_parquet("asd", &fname, ParquetReadOptions::default())
        .await?;

    // Get all sorted unique journey ids
    //
    let c = ctx
        .sql("SELECT DISTINCT journey FROM asd ORDER BY journey")
        .await?;
    eprintln!("{} records read", c.clone().count().await?);

    let _ = c.clone().show().await?;

    let journeys = c.collect().await?;
    dbg!(&journeys);

    journeys.iter().for_each(|rb| {
        let list = rb.column(0).as_any().downcast_ref::<Int32Array>();
        dbg!(&list);
    });

    Ok(vec![])
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

    let journeys = read_and_generate_journeys(&fname).await?;

    eprintln!("{} batches.", journeys.len());
    eprintln!("{:?}", &journeys);

    Ok(())
}
