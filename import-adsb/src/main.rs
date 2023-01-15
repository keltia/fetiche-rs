use crate::cli::Opts;
use crate::version::version;

use clap::Parser;

mod cli;
mod config;
mod version;

fn main() {
    let opts = Opts::parse();

    println!("{}", version());
}
