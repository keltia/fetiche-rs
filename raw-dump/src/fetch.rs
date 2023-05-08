use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use log::{info, trace};
use sources::{Filter, Site};

use crate::cli::FetchOpts;
use crate::Task;

/// From the CLI options
///
pub fn filter_from_opts(opts: &FetchOpts) -> Result<Filter> {
    let t: DateTime<Utc> = Utc::now();

    if opts.today {
        // Build our own begin, end
        //
        let begin = NaiveDate::from_ymd_opt(t.year(), t.month(), t.day())
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(t.year(), t.month(), t.day())
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        Ok(Filter::from(begin, end))
    } else if opts.begin.is_some() {
        // Assume both are there, checked elsewhere
        //
        // We have to parse both arguments ourselves because it uses its own format-specs
        //
        let begin = match &opts.begin {
            Some(begin) => NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S")?,
            None => return Err(anyhow!("bad -B parameter")),
        };
        let end = match &opts.end {
            Some(end) => NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S")?,
            None => return Err(anyhow!("Bad -E parameter")),
        };

        Ok(Filter::from(begin, end))
    } else {
        Ok(Filter::default())
    }
}

/// Check the presence and validity of some of the arguments
///
pub fn check_args(opts: &FetchOpts) -> Result<()> {
    // Do we have options for filter

    if opts.today && (opts.begin.is_some() || opts.end.is_some()) {
        return Err(anyhow!("Can not specify --today and -B/-E"));
    }

    if (opts.begin.is_some() && opts.end.is_none()) || (opts.begin.is_none() && opts.end.is_some())
    {
        return Err(anyhow!("We need both -B/-E or none"));
    }

    Ok(())
}
