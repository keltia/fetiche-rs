pub use adsb::*;

mod adsb;

use clap::{Parser, Subcommand};
use clickhouse::Row;
use serde::Deserialize;

use crate::cmds::import::adsb::AdsbOpts;

pub struct ImportCommand;

#[derive(Debug, Parser)]
pub struct ImportOpts {
    /// DB to import into
    pub name: String,
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
