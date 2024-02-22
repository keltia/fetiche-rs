
use clap::Parser;

use crate::cmds::CrudSubCommand;

/// "acute sites"
///
#[derive(Debug, Parser)]
pub(crate) struct SiteOpts {
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
}

