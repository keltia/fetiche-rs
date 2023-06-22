use std::fs::File;
use std::io::stdout;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{info, trace};

use fetiche_engine::{Convert, Engine, Stream, Tee};
use fetiche_formats::Format;
use fetiche_sources::{Filter, Flow, Site};

use crate::StreamOpts;

/// Actual fetching of data from a given site
///
pub fn stream_from_site(engine: &Engine, sopts: &StreamOpts) -> Result<()> {
    trace!("stream_from_site({:?})", sopts.site);

    check_args(sopts)?;

    let name = &sopts.site;
    let srcs = Arc::clone(&engine.sources());
    let site = match Site::load(name, &engine.sources())? {
        Flow::Streamable(s) => s,
        _ => return Err(anyhow!("this site is not fetchable")),
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

    // Handle output
    //
    if let Some(out) = &sopts.output {
        let mut out = File::create(out)?;

        job.run(&mut out)?;
    } else {
        job.run(&mut stdout())?;
    };

    Ok(())
}

/// From the CLI options
///
pub fn filter_from_opts(opts: &StreamOpts) -> Result<Filter> {
    trace!("filter_from_opts");

    // FIXME: only one argument
    //
    if opts.keyword.is_some() {
        let keyword = opts.keyword.clone().unwrap();

        let v: Vec<_> = keyword.split(':').collect();
        let (k, v) = (v[0], v[1]);
        Ok(Filter::Keyword {
            name: k.to_string(),
            value: v.to_string(),
        })
    } else {
        let duration = opts.duration;
        let delay = opts.delay;
        let from = opts.start.unwrap_or(0);

        Ok(Filter::stream(from, duration, delay))
    }
}

/// Check the presence and validity of some of the arguments
///
fn check_args(opts: &StreamOpts) -> Result<()> {
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
