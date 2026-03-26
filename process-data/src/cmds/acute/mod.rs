//! Module dealing with various ACUTE-specific data like our antennas, sites, etc.
//!
//! This provides a CRUD-like interface with subcommands like `add` & `delete`.
//!

pub(crate) use antennas::*;
pub(crate) use install::*;
pub(crate) use sites::*;

mod antennas;
mod install;
mod sites;

use crate::runtime::Context;

use clap::Parser;
use eyre::Result;

#[derive(Debug, Parser)]
pub struct AcuteOpts {
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: Option<String>,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    #[clap(subcommand)]
    pub subcmd: AcuteSubCommand,
}

#[derive(Debug, Parser)]
pub enum AcuteSubCommand {
    /// Display all antennas.
    #[clap(visible_alias = "a")]
    Antennas(AntennasOpts),
    /// Fetch which antenna was on a site and when.
    #[clap(visible_alias = "i", visible_alias = "inst")]
    Install(InstOpts),
    /// Display all sites.
    #[clap(visible_alias = "s")]
    Sites(SiteOpts),
}

// Sub-commands for all the categories.
//
#[derive(Debug, Default, Parser)]
pub enum CrudSubCommand {
    /// Add a something
    Add,
    /// Modify a something
    Modify,
    /// Remove a something
    Remove,
    /// Default is listing everything
    #[default]
    List,
}

// ----- Dispatching

#[tracing::instrument(skip(ctx))]
pub async fn run_acute_cmd(ctx: &Context, opts: &AcuteOpts) -> Result<()> {
    match &opts.subcmd {
        // List all antennas
        //
        AcuteSubCommand::Antennas(aopts) => {
            // No other command for now.
            //
            antennas_list(&ctx, &aopts).await?;
        }
        // List all installations
        //
        AcuteSubCommand::Install(iopts) => {
            // No other command for now.
            //
            install_list(ctx, iopts).await?;
        }
        AcuteSubCommand::Sites(sopts) => match &sopts.subcmd {
            SitesSubCommand::Add(_opts) => todo!(),
            SitesSubCommand::Modify => todo!(),
            SitesSubCommand::Remove => todo!(),
            SitesSubCommand::List(sopts) => sites_list(ctx, &sopts).await?,
        },
    }

    Ok(())
}
