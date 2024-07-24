use crate::cmds::{AcuteOpts, DistOpts, ExportOpts, SetupOpts};
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Alternate Configuration file
    #[clap(short = 'c', long)]
    pub config: Option<String>,
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: Option<String>,
    /// Delay between task in ms
    #[clap(short = 'w', long, default_value = "100")]
    pub wait: u64,
    /// Sub-commands (see below).
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Display data about Acute sites, etc.
    Acute(AcuteOpts),
    /// Build the ACUTE env. from the ground up.
    Bootstrap,
    /// Distance-related calculations.
    Distances(DistOpts),
    /// Export results as CSV.
    Export(ExportOpts),
    /// Remove macros and other stuff
    Cleanup(SetupOpts),
    /// Prepare the database environment with some tables and macros.
    Setup(SetupOpts),
    /// Generation completion stuff for shells.
    Completion(CompOpts),
    /// List all package versions.
    Version,
}

#[derive(Debug, Parser)]
pub struct CompOpts {
    #[clap(value_parser)]
    pub shell: Shell,
}
