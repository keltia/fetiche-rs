//! Module handling date ranges
//!

use chrono::{DateTime, Duration, Utc};
use eyre::Result;

/// Expand from begin to end with all days in between
///
pub fn expand_interval(begin: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<DateTime<Utc>>> {
    let mut d = begin;
    let mut intv = vec![];

    while d < end {
        intv.push(d);
        d += Duration::try_days(1).unwrap();
    }
    Ok(intv)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(("2024-02-01", "2024-02-01"), ["2024-02-01"].to_vec())]
    #[case(("2024-02-01", "2024-02-02"), ["2024-02-01", "2024-02-02"].to_vec())]
    #[case(("2024-02-01", "2024-02-03"), ["2024-02-01", "2024-02-02", "2024-02-03"].to_vec())]
    fn test_expand_interval(#[case] b: (&str, &str), #[case] a: Vec<&str>) -> Result<()> {
        let bb = dateparser::parse(b.0).unwrap();
        let ee = dateparser::parse(b.1).unwrap();
        let aa: Vec<_> = a
            .iter()
            .map(|e| dateparser::parse(e).unwrap().date_naive())
            .collect::<Vec<_>>();

        let res = expand_interval(bb, ee);
        assert!(res.is_ok());
        let res = res
            .unwrap()
            .iter()
            .map(|e| e.date_naive())
            .collect::<Vec<_>>();
        assert_eq!(aa, res);
        Ok(())
    }
}
