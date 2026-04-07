use clap::Parser;

/// `import adsb` options
///
#[derive(Debug, Parser)]
pub struct Opts {
    /// Datalake path.
    #[clap(short = 'd', long)]
    pub datalake: Option<String>,
    // -----
    /// Specify the site to import into, if not deductable from the filename.
    #[clap(short = 's', long)]
    pub site: Option<u32>,
    /// Table name to import into.
    #[clap(short = 't', long)]
    pub table: String,
    /// Batch import by this number of lines.
    #[clap(short = 'b', long, default_value = "100000")]
    pub batch_size: usize,
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
