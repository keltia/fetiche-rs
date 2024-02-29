//! Module handling date ranges
//!

use chrono::{Datelike, DateTime, Duration, TimeZone, Utc};
use eyre::{eyre, Result};

pub fn parse_range(date: &str) -> Result<(String, String)> {
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

/// Parse and return both sides (or just the first twice)
///
pub fn parse_interval(date: &str) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
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

/// Expand from begin to end with all days in between
///
pub fn expand_interval(begin: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<DateTime<Utc>>> {
    let mut d = begin;
    let mut intv = vec![];

    while d <= end {
        intv.push(d);
        d += Duration::days(1);
    }
    Ok(intv)
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};
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
    #[case("2024-65-01")]
    #[case("2024-02-01..787878787878-03-01")]
    #[should_panic]
    fn test_parse_interval_bad(#[case] inp: &str) {
        let r = parse_interval(inp);
        assert!(r.is_err());
    }

    #[rstest]
    #[case(("2024-02-01", "2024-02-01"), ["2024-02-01"].to_vec())]
    #[case(("2024-02-01", "2024-02-02"), ["2024-02-01", "2024-02-02"].to_vec())]
    #[case(("2024-02-01", "2024-02-03"), ["2024-02-01", "2024-02-02", "2024-02-03"].to_vec())]
    fn test_expand_interval(#[case] b: (&str, &str), #[case] a: Vec<&str>) -> Result<()> {
        let bb = dateparser::parse(b.0).unwrap();
        let ee = dateparser::parse(b.1).unwrap();
        let aa: Vec<_> = a
            .iter()
            .map(|e|
                dateparser::parse(e).unwrap().date_naive())
            .collect::<Vec<_>>();

        let res = expand_interval(bb, ee);
        assert!(res.is_ok());
        let res = res.unwrap().iter().map(|e| e.date_naive()).collect::<Vec<_>>();
        assert_eq!(aa, res);
        Ok(())
    }
}
