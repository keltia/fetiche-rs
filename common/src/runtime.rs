//! Common logging and telemetry initializer
//!
//! TODO: Add code for metrics.

use eyre::Result;
use opentelemetry::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[tracing::instrument]
pub fn init_logging(name: &'static str, use_telemetry: bool) -> Result<()> {
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

    let exporter = opentelemetry_otlp::new_exporter().tonic();
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    let tracer = provider.tracer(name);
    let telemetry = if use_telemetry {
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    };

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
    Ok(())
}

#[tracing::instrument]
pub fn close_logging() {
    opentelemetry::global::shutdown_tracer_provider();
}
