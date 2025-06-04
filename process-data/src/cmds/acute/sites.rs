use clap::Parser;

use crate::cmds::CrudSubCommand;

/// "acute sites"
///
#[derive(Debug, Parser)]
pub(crate) struct SiteOpts {
    #[clap(short = 'c', long)]
    pub csv: bool,
    #[clap(short = 'T', long, default_value = "true")]
    pub table: bool,
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
}

