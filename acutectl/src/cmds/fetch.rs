use std::fs;
use std::io::{stdout, Write};
use std::sync::Arc;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use eyre::{eyre, Result};
use tracing::{info, trace};

use fetiche_engine::{Convert, Engine, Fetch, Filter, Flow, Format, Site, Tee};

use crate::FetchOpts;

/// Actual fetching of data from a given site
///
#[tracing::instrument]
pub fn fetch_from_site(engine: &mut Engine, fopts: &FetchOpts) -> Result<()> {
    trace!("fetch_from_site({:?})", fopts.site);

    check_args(fopts)?;

    let name = &fopts.site;
    let srcs = Arc::clone(&engine.sources());

    let site = match Site::load(name, &srcs)? {
        Flow::Fetchable(s) => s,
        _ => return Err(eyre!("this site is not fetchable")),
    };
    let filter = filter_from_opts(fopts)?;

    info!("Fetching from network site {}", name);

    // Full json array with all point
    //
    let mut task = Fetch::new(name, srcs);

    task.site(site.name()).with(filter);

    let mut data = vec![];

    let mut job = engine.create_job("fetch_from_site");
    job.add(Box::new(task));

    // Do we want a copy of the raw data (often before converting it)
    //
    if let Some(tee) = &fopts.tee {
        let copy = Tee::into(tee);
        job.add(Box::new(copy));
    }

    // If a conversion is requested, insert it
    //
    if let Some(_into) = &fopts.into {
        let mut convert = Convert::new();
        convert.from(site.format()).into(Format::Cat21);
        job.add(Box::new(convert));
    };

    // Launch it now
    //
    job.run(&mut data)?;

    let data = String::from_utf8(data)?;

    match &fopts.output {
        Some(output) => {
            info!("Writing into {:?}", output);
            fs::write(output, data)?
        }
        // stdout otherwise
        //
        _ => write!(stdout(), "{}", data)?,
    }

    // Remove job from engine and state
    //
    engine.remove_job(job)?;

    Ok(())
}

/// From the CLI options
///
#[tracing::instrument]
pub fn filter_from_opts(opts: &FetchOpts) -> Result<Filter> {
    trace!("filter_from_opts");

    let t: DateTime<Utc> = Utc::now();

    if opts.today {
        // Build our own begin, end
        //
        let begin: DateTime<Utc> = Utc
            .with_ymd_and_hms(t.year(), t.month(), t.day(), 0, 0, 0)
            .unwrap();
        let end: DateTime<Utc> = Utc
            .with_ymd_and_hms(t.year(), t.month(), t.day(), 23, 59, 59)
            .unwrap();

        Ok(Filter::interval(begin, end))
    } else if opts.begin.is_some() {
        // Assume both are there, checked elsewhere
        //
        // We have to parse both arguments ourselves because it uses its own formats
        //
        let begin = match &opts.begin {
            Some(begin) => dateparser::parse(begin).unwrap(),
            None => return Err(eyre!("bad -B parameter")),
        };
        let end = match &opts.end {
            Some(end) => dateparser::parse(end).unwrap(),
            None => return Err(eyre!("Bad -E parameter")),
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
    } else if opts.since.is_some() {
        let d = opts.since.unwrap();

        Ok(Filter::Duration(d))
    } else {
        Ok(Filter::default())
    }
}

/// Check the presence and validity of some of the arguments
///
#[tracing::instrument]
fn check_args(opts: &FetchOpts) -> Result<()> {
    trace!("check_args");

    // Do we have options for filter
    //
    if opts.today && (opts.begin.is_some() || opts.end.is_some()) {
        return Err(eyre!("Can not specify --today and -B/-E"));
    }

    if (opts.begin.is_some() && opts.end.is_none()) || (opts.begin.is_none() && opts.end.is_some())
    {
        return Err(eyre!("We need both -B/-E or none"));
    }

    Ok(())
}
