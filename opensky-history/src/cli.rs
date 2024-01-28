use crate::{AUTHORS, NAME, VERSION};
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Location file path
    #[clap(short = 'C', long)]
    pub config: Option<String>,
    /// ICAO code for searches
    #[clap(short = 'I', long)]
    pub icao: Option<String>,
    /// Location name (if in `locations.hcl`).
    #[clap(short = 'n', long)]
    pub name: Option<String>,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Detection range in nautical miles.
    #[clap(short = 'R', long, default_value = "70")]
    pub range: u32,
    /// Start date (YYYY-MM-DD).
    pub start: Option<String>,
    /// End date (YYYY-MM-DD).
    pub end: Option<String>,
}

/// Return our version number
///
#[inline]
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}

/// Display banner
///
pub fn banner() -> eyre::Result<()> {
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
