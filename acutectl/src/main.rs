use clap::{crate_authors, crate_description, crate_version, Parser};
use eyre::Result;
use tracing::trace;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;
use tracing_tree::HierarchicalLayer;

use acutectl::{handle_subcmd, Config, Engine, Opts};

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

fn main() -> Result<()> {
    let opts = Opts::parse();
    let cfn = opts.config.clone();

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

    // Config only has the credentials for every source now.
    //
    let cfg = Config::load(cfn)?;

    // Banner
    //
    banner()?;

    trace!("Engine starting.");
    // Instantiate Engine
    //
    let mut engine = Engine::new();

    // Load auth data
    //
    engine.auth(cfg.site);

    trace!("Engine initialised and running.");

    let subcmd = &opts.subcmd;
    handle_subcmd(&mut engine, subcmd)
}

/// Return our version number
///
#[inline]
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}

/// Display banner
///
fn banner() -> Result<()> {
    Ok(eprintln!(
        r##"
{}/{} by {}
{}
"##,
        NAME,
        VERSION,
        AUTHORS,
        crate_description!()
    ))
}
