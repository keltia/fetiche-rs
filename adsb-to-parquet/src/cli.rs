use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Use arrow2 instead of datafusion?
    #[clap(short = 'A', long)]
    pub arrow2: bool,
    /// Has headers or not?
    #[clap(short = 'N', long = "no-header")]
    pub nh: bool,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Delimiter for csv files.
    #[clap(short, default_value = ",")]
    pub delim: String,
    /// Filename, can be just the basename and .csv/.parquet are implied
    pub name: String,
}
