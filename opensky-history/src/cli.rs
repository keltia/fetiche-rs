use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Location file path
    #[clap(short = 'C', long)]
    pub config: Option<String>,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Location latitude
    pub lat: f32,
    /// Location longitude
    pub lon: f32,
    /// Start date (YYYY-MM-DD).
    pub start: String,
    /// End date (YYYY-MM-DD).
    pub end: String,
}
