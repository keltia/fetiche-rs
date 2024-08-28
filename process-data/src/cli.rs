use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use clap_complete::Shell;

use crate::cmds::{AcuteOpts, DistOpts, ExportOpts, ImportOpts, SetupOpts};

/// Global (aka non-command-related) options.
///
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
    /// Datalake location to use
    #[clap(short = 'l', long)]
    pub datalake: Option<String>,
    /// Delay between task in ms
    #[clap(short = 'w', long, default_value = "100")]
    pub wait: u64,
    /// Enable telemetry with OTLP.
    #[clap(short = 'T', long)]
    pub use_telemetry: bool,
    /// Dry run
    #[clap(short = 'n', long)]
    pub dry_run: bool,
    /// Sub-commands (see below).
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Display data about Acute sites, etc.
    Acute(AcuteOpts),
    /// Build the ACUTE env. from the ground up.
    #[clap(visible_alias = "boot", visible_alias = "restart")]
    Bootstrap,
    /// Distance-related calculations.
    #[clap(visible_alias = "dist", visible_alias = "d")]
    Distances(DistOpts),
    /// Export results as CSV.
    #[clap(visible_alias = "exp", visible_alias = "e")]
    Export(ExportOpts),
    /// Import into a CH instance.
    #[clap(visible_alias = "imp")]
    Import(ImportOpts),
    /// Remove macros and other stuff
    #[clap(visible_alias = "clean", visible_alias = "cls")]
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
