use clap::Parser;

use crate::cmds::CrudSubCommand;

/// "acute antennas"
///
#[derive(Debug, Parser)]
pub(crate) struct AntennasOpts {
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
}  

