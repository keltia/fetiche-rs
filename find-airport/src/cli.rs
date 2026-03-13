use clap::Parser;

/// Command-line options for the airport lookup application.
///
/// Contains the search parameter for finding airports by IATA code or name.
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

/// This enum is for specifying the output format of a query with `find`, not
/// for the actual fetching of the database.
///
#[derive(Debug, Parser)]
pub enum Format {
    /// Output as CSV.
    Csv,
    /// Output as JSON.
    Json,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    Clean,
    Fetch,
    Find(FindOpts),
    Show,
}

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
    /// Search text
    pub text: String,
}
