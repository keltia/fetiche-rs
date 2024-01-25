use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: String,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Sub-commands (see below).
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Convert between formats.
    Distances,
    /// List all available modules.
    List,
    /// Various commands.
    Various,
    /// List all package versions.
    Version,
}
