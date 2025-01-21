//! Common logging and telemetry initializer
//!
//! TODO: Add code for metrics.

use eyre::Result;
use opentelemetry::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

/// Initializes logging for the application.
///
/// This function sets up logging based on the provided options. It can be configured to:
/// - Use hierarchical logging output if `use_tree` is enabled.
/// - Send logs to OpenTelemetry if `use_telemetry` is enabled.
/// - Log output to a file specified by `use_file`.
///
/// The log levels and filters can also be customized using the environment variable `RUST_LOG`.
///
/// # Parameters
///
/// - `name`: The name of the application, used for telemetry and file naming.
/// - `use_telemetry`: Enables OpenTelemetry tracing if set to `true`.
/// - `use_tree`: Enables hierarchical logging tree output if set to `true`.
/// - `use_file`: Specifies an optional file path for appending logs, supporting hourly rotation.
///
/// # Returns
///
/// Returns `Ok(())` if logging is successfully initialized or an error wrapped
/// in `eyre::Result` if any issue occurs during initialization.
#[tracing::instrument]
pub fn init_logging(
    name: &'static str,
    use_telemetry: bool,
    use_tree: bool,
    use_file: Option<String>,
) -> Result<()> {
    // Initialise logging early
    //
    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Do we want hierarchical output?
    //
    let tree = if use_tree {
        Some(
            HierarchicalLayer::new(2)
                .with_ansi(true)
                .with_span_retrace(true)
                .with_span_modes(true)
                .with_targets(true)
                .with_verbose_entry(true)
                .with_verbose_exit(true)
                .with_bracketed_fields(true),
        )
    } else {
        None
    };

    // Enable telemetry?
    //
    let otlp = if use_telemetry {
        let exporter = opentelemetry_otlp::new_exporter().tonic();
        let provider = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;
        let tracer = provider.tracer(name);
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    };

    // Log to file?
    //
    let file = if use_file.is_some() {
        // Basic append-only rolling file for all traces.
        //
        let file_appender = tracing_appender::rolling::hourly(use_file.unwrap(), name);
        Some(tracing_subscriber::fmt::layer().with_writer(file_appender))
    } else {
        None
    };

    // Combine filters & exporters
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .with(otlp)
        .with(file)
        .init();

    Ok(())
}

#[tracing::instrument]
pub fn close_logging() {
    opentelemetry::global::shutdown_tracer_provider();
}
