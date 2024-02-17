//! This is the module handling the `fetch` sub-command.
//!

use std::sync::Arc;

use chrono::{DateTime, Datelike, Days, TimeZone, Utc};
use eyre::{eyre, Result};
use tracing::{info, trace};

use fetiche_formats::{Container, Format};
use fetiche_sources::{Filter, Flow, Site};

use crate::{Convert, Engine, Fetch, FetchOpts, Save, Tee};

/// Actual fetching of data from a given site
///
#[tracing::instrument(skip(engine))]
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
    let input = if let Some(_into) = &fopts.into {
        let mut convert = Convert::new();
        convert.from(site.format()).into(Format::Cat21);
        job.add(Box::new(convert));

        // FIXME: convert does only Cat21 for now
        //
        Format::Cat21
    } else {
        site.format()
    };

    // If a final write format is requested, insert a `Save` task
    //
    let fmt = if let Some(write) = &fopts.write {
        trace!("Write as {}", write);

        // If this is requested, forbid stdout.
        //
        if fopts.output.is_none() {
            return Err(eyre!("you must specify -o/--output"));
        }
        if *write != Container::Parquet {
            return Err(eyre!("Only parquet supported"));
        }

        // Handle input format as the currently defined output one
        //
        Container::Parquet
    } else {
        trace!("No specific write format.");

        Container::Raw
    };

    // Are we writing to stdout?
    //
    let final_output = match &fopts.output {
        Some(fname) => fname.as_str(),
        None => "-",
    };

    info!("Writing to {final_output}");

    // Last task is `Save`
    //
    let mut save = Save::new(final_output, input, fmt);
    save.path(final_output);
    job.add(Box::new(save));

    // Launch it now
    //
    job.run(&mut data)?;

    // Remove job from engine and state
    //
    trace!("Job({}) done, removing it.", job.id);
    engine.remove_job(job)
}

/// From the CLI options
///
#[tracing::instrument]
fn filter_from_opts(opts: &FetchOpts) -> Result<Filter> {
    trace!("filter_from_opts");

    // Start of today
    //
    let t: DateTime<Utc> = Utc::now();
    let t = Utc
        .with_ymd_and_hms(t.year(), t.month(), t.day(), 0, 0, 0)
        .unwrap();

    // Calculate yesterday
    //
    let y = t.checked_sub_days(Days::new(1)).unwrap();

    // Calculate tomorrow
    //
    let tmr = t.checked_add_days(Days::new(1)).unwrap();

    if opts.today {
        // Build our own beginning & end
        //
        let begin = t;
        let end = tmr;

        Ok(Filter::interval(begin, end))
    } else if opts.yesterday {
        // Build our own beginning & end
        //
        let begin = y;
        let end = t;

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

    // Do we have options for filter?
    //
    if opts.today && opts.yesterday {
        return Err(eyre!("Can not specify --today and --yesterday"));
    }
    if opts.today && (opts.begin.is_some() || opts.end.is_some()) {
        return Err(eyre!("Can not specify --today and -B/-E"));
    }

    // (a & ^b) | (^a & b) => xor
    //
    if opts.begin.is_some() ^ opts.end.is_some() {
        return Err(eyre!("We need both -B/-E or none"));
    }

    Ok(())
}
