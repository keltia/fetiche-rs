use clap::Parser;

/// `import adsb` options
///
#[derive(Debug, Parser)]
pub struct Opts {
    /// Datalake path.
    #[clap(short = 'd', long)]
    pub datalake: Option<String>,
    /// Table name to import into.
    #[clap(short = 'T', long)]
    pub table: String,
    /// Batch import by this number of lines.
    #[clap(short = 't', long, default_value = "100000")]
    pub threshold: usize,
    /// DB Profile to use.
    #[clap(short = 'P', long)]
    pub profile: Option<String>,
    // -----
    /// Dry-run, do not write to the database.
    #[clap(short = 'n', long)]
    pub dry_run: bool,
    // -----
    /// Enable telemetry with OTLP.
    #[clap(short = 'T', long)]
    pub use_telemetry: bool,
    /// Enable logging in a hierarchical manner (aka tree)
    #[clap(short = 'L', long)]
    pub use_tree: bool,
    /// Enable logging to a file in that location.
    #[clap(short = 'F', long)]
    pub use_file: Option<String>,
    // -----
    /// Filename
    pub fname: String,
}
