use clap::Parser;

use crate::cmds::CrudSubCommand;

/// "acute antennas"
///
#[derive(Debug, Parser)]
pub(crate) struct AntennasOpts {
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
    #[clap(short = 'c', long)]
    pub csv: bool,
    #[clap(short = 'T', long, default_value = "true")]
    pub table: bool,
}

