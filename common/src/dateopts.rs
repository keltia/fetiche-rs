use std::ops::{Add, Sub};

use chrono::{DateTime, Datelike, Days, TimeDelta, TimeZone, Utc};
use clap::Parser;
use thiserror::Error;

/// Enum of supported options for the date formats.
///
#[derive(Clone, Debug, Parser)]
pub enum DateOpts {
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

#[derive(Debug, Error)]
pub enum ErrDateOpts {
    #[error("bad date: {0}")]
    BadDate(String),
}

impl DateOpts {
    /// Parse options and return a time interval
    ///
    #[tracing::instrument]
    pub fn parse(opts: Self) -> Result<(DateTime<Utc>, DateTime<Utc>), ErrDateOpts> {
        Ok(match opts {
            DateOpts::Today => {
                eprintln!("got today true");
                let today = Utc::now();
                let begin = Utc
                    .with_ymd_and_hms(today.year(), today.month(), today.day(), 0, 0, 0)
                    .unwrap();
                let end = begin.add(Days::new(1));
                eprintln!("today gives from {} to {}", begin, end);
                (begin, end)
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
                (begin, end)
            }
            DateOpts::Day { date } => {
                let begin = match dateparser::parse(&date) {
                    Ok(date) => date,
                    Err(_) => return Err(ErrDateOpts::BadDate(date)),
                };
                let begin = Utc
                    .with_ymd_and_hms(begin.year(), begin.month(), begin.day(), 0, 0, 0)
                    .unwrap();
                let end = begin.add(Days::new(1));
                eprintln!("this day={} gives from {} to {}", date, begin, end);
                (begin, end)
            }
            DateOpts::Week { num } => {
                if num > 53 {
                    return Err(ErrDateOpts::BadDate(num.to_string()));
                }
                let week = Utc::now();
                let begin: DateTime<Utc> =
                    Utc.with_ymd_and_hms(week.year(), 1, 1, 0, 0, 0).unwrap();
                let begin = begin.checked_add_signed(TimeDelta::try_weeks(num - 1).unwrap()).unwrap();
                let end = begin.add(Days::new(7));
                eprintln!("week={} is from {} to {}", num, begin, end);
                (begin, end)
            }
            DateOpts::From { begin, end } => {
                let begin = match dateparser::parse(&begin) {
                    Ok(date) => date,
                    Err(_) => return Err(ErrDateOpts::BadDate(begin)),
                };
                let end = match dateparser::parse(&end) {
                    Ok(date) => date,
                    Err(_) => return Err(ErrDateOpts::BadDate(end)),
                };
                eprintln!("begin={} end={}", begin, end);
                (begin, end)
            }
        })
    }
}
