use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use eyre::{eyre, Result};

pub use to_home::*;
pub use to_planes::*;

mod to_home;
mod to_planes;

#[derive(Debug, Parser)]
pub(crate) struct DistOpts {
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// `distances` sub-commands
    #[clap(subcommand)]
    pub subcmd: DistSubcommand,
}

#[derive(Clone, Debug, Parser)]
pub(crate) enum DistSubcommand {
    /// 2D/3D drone to operator distance.
    Home,
    /// drone to planes distance
    Planes(PlanesOpts),
}

// -----

fn parse_range(date: &str) -> Result<(String, String)> {
    let intv: Vec<&str> = date.split("..").collect();
    let (start, end) = match intv.len() {
        1 => {
            let start = intv[0];
            (start, start)
        }
        2 => {
            let start = intv[0];
            let end = intv[1];
            (start, end)
        }
        _ => {
            return Err(eyre!(
                "Bad interval, need single or couple dates.".to_string()
            ));
        }
    };
    // if end is empty, we had only "DDDD.." so return start both times
    //
    if end.is_empty() {
        Ok((start.to_string(), start.to_string()))
    } else {
        Ok((start.to_string(), end.to_string()))
    }
}

fn parse_interval(date: &str) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    // Parse and return both sides (or just the first twice)
    //
    let (start, end) = parse_range(date)?;

    // Parse and validate date
    //
    let start = dateparser::parse(&start).unwrap();
    let end = dateparser::parse(&end).unwrap();

    // Normalise dates at beginning of day
    //
    let start = Utc
        .with_ymd_and_hms(start.year(), start.month(), start.day(), 0, 0, 0)
        .unwrap();
    let end = Utc
        .with_ymd_and_hms(end.year(), end.month(), end.day(), 0, 0, 0)
        .unwrap();

    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("2024-02-01", ("2024-02-01", "2024-02-01"))]
    #[case("2024-02-01..2024-03-01", ("2024-02-01", "2024-03-01"))]
    #[case("2024-02-01..", ("2024-02-01", "2024-02-01"))]
    fn test_parse_range(#[case] inp: &str, #[case] out: (&str, &str)) {
        let (b, e) = parse_range(inp).unwrap();
        assert_eq!(out, (b.as_str(), e.as_str()));
    }

    #[rstest]
    #[should_panic]
    #[case("2024-65-01")]
    #[case("2024-02-01..787878787878-03-01")]
    fn test_parse_interval_bad(#[case] inp: &str) {
        let r = parse_range(inp);
        assert!(r.is_err());
        let (b, e) = r.unwrap();
        dbg!(&b, &e);
        let b = dateparser::parse(&b).unwrap();
        let e = dateparser::parse(&e).unwrap();
    }
}
