use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use log::{info, trace};

use fetiche_engine::{Fetch, Job};
use fetiche_sources::{Filter, Site, Sources};

use crate::FetchOpts;

/// Actual fetching of data from a given site
///
pub fn fetch_from_site(cfg: &Sources, fopts: &FetchOpts) -> Result<String> {
    trace!("fetch_from_site({:?})", fopts.site);

    check_args(fopts)?;

    let name = &fopts.site;
    let site = Site::load(name, cfg)?;
    let filter = filter_from_opts(fopts)?;

    info!("Fetching from network site {}", name);

    // Full json array with all point
    //
    let mut task = Fetch::new(name);

    task.site(site).with(filter);

    let data = Job::new("fetch_from_site").add(Box::new(task)).run()?;

    trace!("data={}", data);
    Ok(data)
}

/// From the CLI options
///
pub fn filter_from_opts(opts: &FetchOpts) -> Result<Filter> {
    trace!("filter_from_opts");

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

        Ok(Filter::interval(begin, end))
    } else if opts.begin.is_some() {
        // Assume both are there, checked elsewhere
        //
        // We have to parse both arguments ourselves because it uses its own formats
        //
        let begin = match &opts.begin {
            Some(begin) => NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S")?,
            None => return Err(anyhow!("bad -B parameter")),
        };
        let end = match &opts.end {
            Some(end) => NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S")?,
            None => return Err(anyhow!("Bad -E parameter")),
        };

        Ok(Filter::interval(begin, end))
    } else if opts.keyword.is_some() {
        let keyword = opts.keyword.clone().unwrap();

        let v: Vec<_> = keyword.split(':').collect();
        let (k, v) = (v[0], v[1]);
        Ok(Filter::Keyword {
            name: k.to_string(),
            value: v.to_string(),
        })
    } else {
        Ok(Filter::default())
    }
}

/// Check the presence and validity of some of the arguments
///
fn check_args(opts: &FetchOpts) -> Result<()> {
    trace!("check_args");

    // Do we have options for filter
    //
    if opts.today && (opts.begin.is_some() || opts.end.is_some()) {
        return Err(anyhow!("Can not specify --today and -B/-E"));
    }

    if (opts.begin.is_some() && opts.end.is_none()) || (opts.begin.is_none() && opts.end.is_some())
    {
        return Err(anyhow!("We need both -B/-E or none"));
    }

    Ok(())
}
