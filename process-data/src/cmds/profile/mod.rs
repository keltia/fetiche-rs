//! `profile` sub-command
//!
use crate::{Context, Profile};

use clap::Parser;
use eyre::Result;
use farben::ceprintln;
use itertools::Itertools;

use std::collections::HashMap;

#[derive(Debug, Parser)]
pub enum ProfCmd {
    /// List all possible profiles.
    List,
}

#[derive(Debug, Parser)]
pub struct ProfileOpts {
    #[clap(subcommand)]
    pub subcmd: ProfCmd,
}

type Profiles = HashMap<String, Profile>;

#[tracing::instrument(skip(ctx))]
pub fn cmd_profile(ctx: &Context, opts: &ProfileOpts) -> Result<()> {
    // Check what profiles are available
    // At this point, it is either $CLICKHOUSE_PROFILE or "default"
    //
    // Priority:
    // - CLI option, if present,
    // - Environment variable, if present, defaults to "default"
    //
    let allp = ctx.config.get("allp").unwrap();
    let currp = ctx.config.get("profile").unwrap();

    let allp: Profiles = serde_json::from_str(allp)?;
    match opts.subcmd {
        ProfCmd::List => {
            ceprintln!("Current profile: [green]{}[/]", currp);
            eprintln!("All profiles:\n");
            allp.into_iter().sorted().for_each(|(name, p)| {
                if name == *currp {
                    ceprintln!("  [green]{:>10}: {}[/]", name, p)
                } else {
                    eprintln!("  {:>10}: {}", name, p)
                }
            });
        }
    }
    Ok(())
}
