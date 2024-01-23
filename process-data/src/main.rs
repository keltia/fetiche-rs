//! Utility implement different processing tasks over our locally stored data.
//!

use clap::{crate_authors, crate_version, Parser};
use duckdb::arrow::record_batch::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;
use duckdb::Config;
use eyre::Result;
use tokio::time::Instant;
use tracing::{info, trace};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use crate::cli::Opts;

mod cli;

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_span_retrace(true)
        .with_span_modes(true)
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

    info!("Connecting to {}", opts.database);
    let dbh = duckdb::Connection::open_with_flags(
        &opts.database,
        Config::default()
            .allow_unsigned_extensions()?
            .enable_autoload_extension(true)?,
    )?;

    dbh.execute_batch("LOAD spatial")?;

    trace!("execute");

    // Fetch sites
    //
    let t1 = Instant::now();
    let mut stmt = dbh.prepare(
        r##"
SELECT
  name,
  code,
  ST_Point3D(2.35, 48.6,10) AS ref,
  ST_Point3D(longitude, latitude,0) AS here,
  ST_Distance(
    ST_Transform(here, 'EPSG:4326', 'ESRI:102718'), 
    ST_Transform(ref, 'EPSG:4326', 'ESRI:102718') 
  ) / 5280 AS distance
FROM sites
ORDER BY
  name
    "##,
    )?;
    // let res_iter = stmt.query_map([], |row| {
    //     let name: String = row.get_unwrap(0);
    //     let code: String = row.get_unwrap(1);
    //     let coord: Geometry = row.get_unwrap(2);
    //     Ok((name, code, coord))
    // })?;
    // for site in res_iter {
    //     let (n, c, l) = site.unwrap();
    //     println!("site={} code={} coord={:?}", n, c, l);
    // }
    let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
    let t1 = t1.elapsed().as_millis();
    println!("q1 took {}ms", t1);
    print_batches(&rbs)?;

    // Fetch antennas as Arrow
    //
    let t1 = Instant::now();
    let mut stmt = dbh.prepare("select * from antennas;")?;
    let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
    let t1 = t1.elapsed().as_millis();
    println!("q2 took {}ms", t1);
    print_batches(&rbs)?;

    // Find all installations with sites' name and antenna's ID
    //
    let t1 = Instant::now();
    let mut stmt = dbh.prepare(
        r##"
SELECT 
  inst.id,
  sites.name,
  start_at,
  end_at,
  antennas.name AS station_name
FROM
  installations AS inst
  JOIN antennas ON antennas.id = inst.antenna_id
  JOIN sites ON inst.site_id = sites.id
ORDER BY start_at
        "##,
    )?;
    let t1 = t1.elapsed().as_millis();
    println!("prepare took {}ms", t1);

    let t1 = Instant::now();
    let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
    let t1 = t1.elapsed().as_millis();
    println!("q3 took {}ms", t1);
    print_batches(&rbs)?;

    let t1 = Instant::now();
    let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
    let t1 = t1.elapsed().as_millis();
    println!("q4 took {}ms", t1);
    print_batches(&rbs)?;

    // Finish
    //
    opentelemetry::global::shutdown_tracer_provider();
    trace!("Closing DB.");
    let _ = dbh.close();
    Ok(())
}
