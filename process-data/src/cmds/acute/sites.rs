use clap::Parser;
use eyre::Result;
use polars::prelude::*;

/// "acute sites"
///
#[derive(Debug, Parser)]
pub(crate) struct SiteOpts {
    #[clap(short = 'c', long)]
    pub csv: bool,
    #[clap(short = 'T', long, default_value = "true")]
    pub table: bool,
    #[clap(subcommand)]
    pub subcmd: SitesSubCommand,
}

#[derive(Debug, Default, Parser)]
pub enum SitesSubCommand {
    /// Add a something
    Add(AddSiteOpts),
    /// Modify a something
    Modify,
    /// Remove a something
    Remove,
    /// Default is listing everything
    #[default]
    List,
}

#[derive(Debug, Parser)]
pub struct FindOpts {
    pub name: String,
}

#[derive(Debug, Default, Parser)]
struct AddSiteOpts {
    name: String,
    lat: f64,
    lon: f64,
    alt: i32,
    basename: String,
}

fn insert_new_site(opts: &AddSiteOpts) -> Result<()> {
    Ok(())
}
