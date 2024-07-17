use eyre::Result;
use tracing::trace;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;
use tracing_tree::HierarchicalLayer;

#[tracing::instrument]
pub(crate) fn init_runtime(name: &str) -> Result<()> {
    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_span_retrace(true)
        .with_span_modes(true)
        .with_targets(true)
        .with_verbose_entry(true)
        .with_verbose_exit(true)
        .with_bracketed_fields(true);

    // Setup Open Telemetry with Jaeger
    //
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_auto_split_batch(true)
        .with_max_packet_size(9_216)
        .with_service_name(name)
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
    Ok(())
}
