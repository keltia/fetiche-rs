use std::fs::File;
use std::io::stdout;
use std::sync::Arc;

use eyre::{eyre, Result};
use tracing::{info, trace};

use fetiche_formats::Format;
use fetiche_sources::{Filter, Flow, Site};

use crate::{Convert, Engine, Store, Stream, StreamOpts, Tee};

/// Actual fetching of data from a given site
///
#[tracing::instrument]
pub fn stream_from_site(engine: &mut Engine, sopts: &StreamOpts) -> Result<()> {
    trace!("stream_from_site({:?})", sopts.site);

    check_args(sopts)?;

    let name = &sopts.site;
    let srcs = Arc::clone(&engine.sources());
    let site = match Site::load(name, &engine.sources())? {
        Flow::Streamable(s) => s,
        _ => return Err(eyre!("this site is not fetchable")),
    };

    let filter = filter_from_opts(sopts)?;
    info!("Streaming from network site {}", name);

    // Full json array with all point
    //
    let mut task = Stream::new(name, srcs);
    task.site(site.name()).with(filter);

    // Create job with first task
    //
    let mut job = engine.create_job("stream_from_site");
    job.add(Box::new(task));

    // Do we want a copy of the raw data (often before converting it)
    //
    if let Some(tee) = &sopts.tee {
        let copy = Tee::into(tee);
        job.add(Box::new(copy));
    }

    // If a conversion is requested, insert it
    //
    if let Some(_into) = &sopts.into {
        let mut convert = Convert::new();
        convert.from(site.format()).into(Format::Cat21);
        job.add(Box::new(convert));
    };

    // If split is required, add a consumer for it at the end.
    //
    info!("Running job #{} with {} tasks.", job.id, job.list.len());
    if sopts.split.is_some() {
        let basedir = sopts.split.as_ref().unwrap();

        // Store must be the last one, it is a pure consumer
        //
        let store = Store::new(basedir, job.id);
        job.add(Box::new(store));

        job.run(&mut stdout())?;
    } else {
        // Handle output if no consumer is present at the end
        //
        if let Some(out) = &sopts.output {
            let mut out = File::create(out)?;

            job.run(&mut out)?;
        } else {
            job.run(&mut stdout())?;
        };
    }

    // Remove job from engine and state
    //
    engine.remove_job(job)
}

/// From the CLI options
///
#[tracing::instrument]
fn filter_from_opts(opts: &StreamOpts) -> Result<Filter> {
    trace!("filter_from_opts");

    // FIXME: only one argument
    //
    let filter = if opts.keyword.is_some() {
        let keyword = opts.keyword.clone().unwrap();

        let v: Vec<_> = keyword.split(':').collect();
        let (k, v) = (v[0], v[1]);
        Filter::Keyword {
            name: k.to_string(),
            value: v.to_string(),
        }
    } else {
        let duration = opts.duration;
        let delay = opts.delay;
        let from = opts.start.unwrap_or(0);

        Filter::stream(from, duration, delay)
    };
    Ok(filter)
}

/// Check the presence and validity of some of the arguments
///
#[tracing::instrument]
fn check_args(opts: &StreamOpts) -> Result<()> {
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
