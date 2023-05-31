use std::fs::File;
use std::io::stdout;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::{info, trace};

use fetiche_engine::{Job, Stream};
use fetiche_sources::{Filter, Flow, Site, Sources};

use crate::StreamOpts;

/// Actual fetching of data from a given site
///
pub fn stream_from_site(cfg: &Sources, sopts: &StreamOpts) -> Result<()> {
    trace!("stream_from_site({:?})", sopts.site);

    check_args(sopts)?;

    let name = &sopts.site;
    let site = match Site::load(name, cfg)? {
        Flow::Streamable(s) => s,
        _ => return Err(anyhow!("this site is not fetchable")),
    };

    let filter = filter_from_opts(sopts)?;
    info!("Streaming from network site {}", name);

    // Full json array with all point
    //
    let mut task = Stream::new(name);

    task.site(site).with(filter);
    if let Some(out) = &sopts.output {
        let mut out = File::create(out)?;

        Job::new("stream_from_site")
            .add(Box::new(task))
            .run(&mut out)?;
    } else {
        Job::new("stream_from_site")
            .add(Box::new(task))
            .run(&mut stdout())?;
    };

    Ok(())
}

/// From the CLI options
///
pub fn filter_from_opts(opts: &StreamOpts) -> Result<Filter> {
    trace!("filter_from_opts");

    let t: DateTime<Utc> = Utc::now();

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
