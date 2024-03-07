use std::fmt::Debug;

use clap::Parser;

pub use home::*;
pub use planes::*;

mod home;
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
    /// 2D/3D drone to operator distance.
    Home,
    /// drone to planes distance
    Planes(PlanesOpts),
}

// -----

