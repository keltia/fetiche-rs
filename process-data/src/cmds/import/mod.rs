pub use adsb::*;

mod adsb;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct ImportOpts {
    /// Sub-command
    #[clap(subcommand)]
    pub subcmd: ImportSubcommand,
}

#[derive(Debug, Parser)]
pub enum ImportSubcommand {
    #[clap(visible_alias = "a")]
    Adsb(AdsbOpts),
}

// -----
