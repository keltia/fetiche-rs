//! The `prune` command removes encounter data from the database, like after some
//! metadata changes affecting past calculations, like fixing installation dates.
//!
use clap::Parser;

use crate::Context;
use fetiche_common::DateOpts;

#[derive(Debug, Parser)]
pub struct PruneOpts {
    /// Do not execute the command yet.
    #[clap(short = 'n', long)]
    pub dry_run: bool,
    /// Site (or ALL).
    #[clap(short = 's', long, default_value = "ALL")]
    pub site: Option<String>,
    /// Specific date or range to prune.
    #[clap(subcommand)]
    pub date: DateOpts,
}

#[tracing::instrument(skip(ctx))]
pub fn run_prune_cmd(ctx: &Context, opts: &PruneOpts) {
    todo!()
}
