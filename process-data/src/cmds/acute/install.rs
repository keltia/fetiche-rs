
use clap::Parser;

use crate::cmds::CrudSubCommand;

/// "acute installations"
///
#[derive(Debug, Parser)]
pub(crate) struct InstOpts {
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
}

