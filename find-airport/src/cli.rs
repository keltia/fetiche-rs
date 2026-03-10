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
    List(ListOpts),
    Show,
}

#[derive(Debug, Parser)]
pub struct FindOpts {
    /// Airport IATA code or name to search for (default: "CDG")
    #[clap(default_value = "CDG")]
    pub name: String,
}

#[derive(Debug, Parser)]
pub struct ListOpts {
    #[clap(short = 'C', long)]
    pub country: String,
}
