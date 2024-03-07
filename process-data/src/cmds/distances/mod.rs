use std::fmt::Debug;

use clap::Parser;

pub use planes::*;

mod planes;

#[derive(Debug, Parser)]
pub(crate) struct DistOpts {
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// `distances` sub-commands
    #[clap(subcommand)]
    pub subcmd: DistSubcommand,
}

#[derive(Clone, Debug, Parser)]
pub(crate) enum DistSubcommand {
    /// drone to planes distance
    Planes(PlanesOpts),
}

// -----

