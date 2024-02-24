//! This is the module handling the `fetch` sub-command.
//!

use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use eyre::{eyre, Result};
use fetiche_common::DateOpts;
use tracing::{info, trace};

use fetiche_formats::{Container, Format};
use fetiche_sources::{Filter, Flow, Site};

use crate::{Convert, Engine, Fetch, FetchOpts, Save, Tee};

/// Actual fetching of data from a given site
///
#[tracing::instrument(skip(engine))]
pub fn fetch_from_site(engine: &mut Engine, fopts: &FetchOpts) -> Result<()> {
    trace!("fetch_from_site({:?})", fopts.site);

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

    // Are we writing to stdout?
    //
    let final_output = match &fopts.output {
        Some(fname) => fname.as_str(),
        None => "-",
    };

    // Deduce format from file name if specified, otherwise it is raw output to stdout.
    //
    let fmt = match &fopts.output {
        Some(fname) => {
            let fname = fname.to_lowercase();
            let ext = Path::new(&fname)
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string();

            Container::from_str(&ext)?
        }
        None => Container::default(),
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

    match &opts.dates {
        Some(dates) => {
            let (begin, end) = DateOpts::parse(dates.clone())?;
            Ok(Filter::Interval { begin, end })
        }
        None => {
            if opts.keyword.is_some() {
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
    }
}
