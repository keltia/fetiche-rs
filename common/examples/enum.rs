//! Proof of concept about parsing date parameters with `clap` and parameters.
//!
//! Now integrated into `fetiche-common`.
//!

use clap::Parser;
use fetiche_common::DateOpts;

#[derive(Debug, Parser)]
struct Opts {
    #[clap(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Parser)]
enum Cmd {
    Fetch(FetchOpts),
}

#[derive(Debug, Parser)]
struct FetchOpts {
    #[clap(subcommand)]
    opts: DateOpts,
}

fn main() -> eyre::Result<()> {
    let opts: Opts = Opts::parse();

    match opts.cmd {
        Cmd::Fetch(opts) => {
            let _ = DateOpts::parse(opts.opts)?;
        }
    }
    Ok(())
}
