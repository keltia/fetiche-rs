//! Proof of concept about parsing date parameters with `clap` and parameters.
//!

use chrono::{DateTime, Datelike, Days, TimeDelta, TimeZone, Utc};
use clap::Parser;
use eyre::bail;
use std::ops::{Add, Sub};

#[derive(Debug, Parser)]
struct Opts {
    #[clap(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Parser)]
enum Cmd {
    Fetch(FetchOpts),
    Compute(CompOpts),
}

#[derive(Debug, Parser)]
struct FetchOpts {
    #[clap(subcommand)]
    opts: DateOpts,
}

#[derive(Clone, Debug, Parser)]
enum DateOpts {
    /// Basic from to
    From { begin: String, end: String },
    /// Specific day
    Day { date: String },
    /// Specific week
    Week { num: i64 },
    /// Shortcut for today
    Today,
    /// Shortcut to yesterday
    Yesterday,
}

#[derive(Debug, Parser)]
struct CompOpts {
    #[clap(long)]
    pub now: bool,
}

fn main() -> eyre::Result<()> {
    let opts: Opts = Opts::parse();

    match opts.cmd {
        Cmd::Fetch(opts) => match opts.opts {
            DateOpts::Today => {
                eprintln!("got today true");
                let today = Utc::now();
                let begin = Utc
                    .with_ymd_and_hms(today.year(), today.month(), today.day(), 0, 0, 0)
                    .unwrap();
                let end = begin.add(Days::new(1));
                eprintln!("today gives from {} to {}", begin, end);
            }
            DateOpts::Yesterday => {
                eprintln!("got yesterday true");
                let today = Utc::now();
                let yest = today.sub(Days::new(1));
                let begin = Utc
                    .with_ymd_and_hms(yest.year(), yest.month(), yest.day(), 0, 0, 0)
                    .unwrap();
                let end = Utc
                    .with_ymd_and_hms(today.year(), today.month(), today.day(), 0, 0, 0)
                    .unwrap();
                eprintln!("yesterday gives from {} to {}", begin, end);
            }
            DateOpts::Day { date } => {
                let begin = match dateparser::parse(&date) {
                    Ok(date) => date,
                    Err(_) => return bail!("Bad date."),
                };
                let begin = Utc
                    .with_ymd_and_hms(begin.year(), begin.month(), begin.day(), 0, 0, 0)
                    .unwrap();
                let end = begin.add(Days::new(1));
                eprintln!("this day={} gives from {} to {}", date, begin, end);
            }
            DateOpts::Week { num } => {
                let week = Utc::now();
                let begin: DateTime<Utc> =
                    Utc.with_ymd_and_hms(week.year(), 1, 1, 0, 0, 0).unwrap();
                let target = begin.checked_add_signed(TimeDelta::weeks(num - 1)).unwrap();
                eprintln!("this week={} is {}", num, target);
            }
            DateOpts::From { begin, end } => {
                let begin = match dateparser::parse(&begin) {
                    Ok(date) => date,
                    Err(_) => return bail!("Bad date."),
                };
                let end = match dateparser::parse(&end) {
                    Ok(date) => date,
                    Err(_) => return bail!("Bad date."),
                };
                eprintln!("begin={} end={}", begin, end);
            }
        },
        Cmd::Compute(opts) => {
            todo!()
        }
    }
    Ok(())
}
