use clap::Parser;

use crate::cmds::CrudSubCommand;

/// "acute installations"
///
#[derive(Debug, Parser)]
pub(crate) struct InstOpts {
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
    #[clap(short = 'c', long)]
    pub csv: bool,
    #[clap(short = 'T', long, default_value = "true")]
    pub table: bool,
}

