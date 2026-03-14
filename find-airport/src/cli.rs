use clap::Parser;

/// Command-line options for the airport lookup application.
///
#[derive(Debug, Parser)]
pub struct Opts {
    /// Directory holding the parquet files for the datalake.
    pub datalake: Option<String>,
    /// Enable logging in a hierarchical manner (aka tree)
    #[clap(short = 'L', long)]
    pub use_tree: bool,
    /// This parameter enables logging to a file in that location.
    #[clap(short = 'F', long)]
    pub use_file: Option<String>,
    /// Dry run
    #[clap(short = 'n', long)]
    pub dry_run: bool,
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

/// List of all sub-commands.
///
#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Remove the current parquet files from the datalake.
    Clean,
    /// Fetch or refresh the latest parquet files from the datalake.
    Fetch,
    /// Find airports by IATA code, ICAO code, name, or country.
    Find(FindOpts),
    /// Show the current parquet files in the datalake.
    Show,
}

/// Only `find` tales options
///
#[derive(Clone, Debug, Parser)]
pub struct FindOpts {
    /// Display by country code
    #[clap(short = 'C', long)]
    pub country: bool,
    /// Find by ICAO code
    #[clap(short = 'I', long)]
    pub icao: bool,
    /// Airport IATA code
    #[clap(short = 'A', long)]
    pub iata: bool,
    /// Find by searching in the name
    #[clap(short = 'N', long)]
    pub name: bool,
    // -----
    /// Output as CSV.
    #[clap(short = 'c', long)]
    pub csv: bool,
    /// Output as JSON.
    #[clap(short = 'J', long)]
    pub json: bool,
    /// Output as NDJSON.
    #[clap(short = 'L', long)]
    pub ndjson: bool,
    // ------
    /// Search text
    pub text: String,
}

/// This enum is for specifying the output format of a query with `find`, not
/// for the actual fetching of the database.
///
#[derive(Clone, Debug, Default)]
pub enum Format {
    /// Output as CSV.
    Csv,
    /// Output as JSON.
    Json,
    /// Output as NDJSON.
    Ndjson,
    /// Table (default)
    #[default]
    Plain,
}
