//! Configuration module
//!
//! Configuration file:
//! - `%LOCALAPPDATA%\drone-utils`       Windows
//! - `$HOME/.config/drone-utils`   UNIX
//!
//! `database`:     default path for database
//!

use clap::Parser;

#[derive(Debug, Parser)]
pub struct ConfOpts {
    #[clap(subcommand)]
    pub subcmd: ConfSubCommand,
}

#[derive(Debug, Parser)]
pub enum ConfSubCommand {
    Get(String),
    List(ListOpts),
    Set,
}

#[derive(Debug, Parser)]
pub struct ListOpts {
    #[clap(short = 'a', long)]
    pub all: bool,
    pub name: Option<String>,
}

pub fn run_config_cmd(opts: ConfOpts) -> eyre::Result<()> {
    match opts.subcmd {
        ConfSubCommand::Get(name) => {
            todo!()
        }
        ConfSubCommand::Set => {
            todo!()
        }
        ConfSubCommand::List(opts) => {
            todo!()
        }
    }
    Ok(())
}
