use chrono::{DateTime, Utc};
use clap::Parser;

#[derive(Debug, Parser)]
struct Opts {
    #[clap(subcommand)]
    pub cmd: Try,
}

#[derive(Debug, Parser)]
enum Try {
    Single { flag: bool },
    Multiple { begin: DateTime<Utc>, end: DateTime<Utc> },
}

fn main() {
    let opts: Opts = Opts::parse();

    match opts.cmd {
        Try::Single { flag } => {
            eprintln!("got single: {}", flag);
        }
        Try::Multiple { begin, end } => {
            eprintln!("begin={} end={}", begin, end);
        }
    }
}
