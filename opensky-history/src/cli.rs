use crate::{AUTHORS, NAME, VERSION};
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use strum::EnumString;

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Do we want CSV or Parquet (default)
    #[clap(
        short = 'T',
        long = "output_type",
        default_value = "parquet",
        value_parser
    )]
    pub otype: Otype,
    /// Location file path
    #[clap(short = 'C', long)]
    pub config: Option<String>,
    /// ICAO code for searches
    #[clap(short = 'I', long)]
    pub icao: Option<String>,
    /// Output file (mandatory)
    #[clap(short = 'o', long = "output", default_value = "output.parquet")]
    pub output: String,
    /// Detection range in nautical miles.
    #[clap(short = 'R', long, default_value = "70")]
    pub range: u32,
    #[clap(short = 'V')]
    pub version: bool,
    /// Start date (YYYY-MM-DD).
    #[clap(short = 'B', long)]
    pub begin: Option<String>,
    /// End date (YYYY-MM-DD).
    #[clap(short = 'E', long)]
    pub end: Option<String>,
    /// Location name (if in `locations.hcl`).
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, strum::Display, EnumString, strum::VariantNames, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum Otype {
    Csv,
    #[default]
    Parquet,
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
