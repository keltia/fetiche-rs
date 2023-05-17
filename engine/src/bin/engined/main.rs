use anyhow::Result;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, Parser, Subcommand, ValueEnum,
};
use log::{info, trace};

/// Binary
const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary-specific version
const DVERSION: &str = "0.1.0";

/// Version fro as package/version
///
fn version() -> String {
    format!("{}/{}", NAME, DVERSION)
}

/// CLI options
#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    /// Stay quiet
    #[clap(short = 'Q')]
    pub quiet: bool,
    /// Display our parameters
    #[clap(short = 'V', long)]
    pub version: bool,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    env_logger::init();

    trace!("{} {} starting.", version(), fetiche_engine::version());
    info!("Fetiche Engine starting.");

    Ok(())
}
