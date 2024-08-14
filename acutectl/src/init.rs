use eyre::Result;
use opentelemetry::trace::TracerProvider;
use tracing::trace;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use crate::NAME;

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

    // Setup Open Telemetry with OTLP
    //
    let exporter = opentelemetry_otlp::new_exporter().tonic();
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    let tracer = provider.tracer(NAME);
    let _telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        //      .with(telemetry)
        .init();
    trace!("Logging initialised.");
    Ok(())
}
