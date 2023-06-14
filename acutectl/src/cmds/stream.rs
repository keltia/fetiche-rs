use std::fs::File;
use std::io::stdout;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{info, trace};

use fetiche_engine::{Engine, Stream};
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
    if let Some(out) = &sopts.output {
        let mut out = File::create(out)?;

        engine
            .create_job("stream_from_site")
            .add(Box::new(task))
            .run(&mut out)?;
    } else {
        engine
            .create_job("stream_from_site")
            .add(Box::new(task))
            .run(&mut stdout())?;
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
