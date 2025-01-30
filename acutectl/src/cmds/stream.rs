use std::fs::File;
use std::io::stdout;

use eyre::Result;
use fetiche_engine::{Convert, Engine, Store, Stream, Task, Tee};
use fetiche_formats::Format;
use fetiche_sources::{Filter, Flow};
use tracing::{debug, error, info, trace};

use crate::{Status, StreamOpts};

/// Stream data from a specified site, applying filters and handling job execution and output.
///
/// # Parameters
/// - `engine`: A mutable reference to the `Engine` instance managing tasks and jobs.
/// - `sopts`: Reference to `StreamOpts` containing the options for streaming (e.g., filters, output, format).
///
/// # Returns
/// - `Result<()>`: Indicates whether the streaming was successful or if an error was encountered.
///
/// # Description
/// This function handles the streaming of data from a given site. Key functionalities include:
/// - Validation of streamable sites using `Flow`.
/// - Creation of a `Job` containing one or more tasks such as:
///   - Streaming data from the site.
///   - Tee (copying data while processing).
///   - Conversion between formats (optional, deprecated).
///   - Storing split data, if requested.
/// - Managing output (to a file or stdout) based on the provided options.
///
/// The function ensures that the generated job is properly cleaned up from the engine after execution.
///
/// # Errors
/// This function will return an error if:
/// - The site specified in `sopts` is not streamable.
/// - Options provided in `sopts` are invalid or conflicting.
/// - File handling (e.g., creating output files) fails during execution.
/// - Any errors occur during task execution.
///
#[tracing::instrument(skip(engine))]
pub async fn stream_from_site(engine: &mut Engine, sopts: &StreamOpts) -> Result<()> {
    trace!("stream_from_site({:?})", sopts.site);

    check_args(sopts)?;

    let name = &sopts.site;
    let srcs = engine.sources().await?.clone();
    let site = srcs.load(name)?;
    debug!("{:?}", site);
    match site {
        Flow::Streamable(_) => (),
        Flow::AsyncStreamable(_) => (),
        _ => {
            error!("Site {} is not Streamable!", site.name());
            return Err(Status::SiteNotStreamable(site.name()).into());
        }
    };

    let filter = filter_from_opts(sopts)?;
    info!("Streaming from network site {}", name);

    // Full json array with all point
    //
    let mut task = Stream::new(name, srcs.into());
    task.site(name.to_string()).with(filter);
    let task = Task::from(task);

    // Create job with first task
    //
    let mut job = engine.create_job("stream_from_site").await?;
    let _ = job.add(task);

    // Do we want a copy of the raw data (often before converting it)
    //
    if let Some(tee) = &sopts.tee {
        let copy = Task::from(Tee::into(tee));
        let _ = job.add(copy);
    }

    // If a conversion is requested, insert it
    //
    // FIXME: DEPRECATED
    //
    if let Some(_into) = &sopts.into {
        let mut convert = Convert::new();
        convert.from(site.format()).into(Format::Cat21);
        let convert = Task::from(convert);
        let _ = job.add(convert);
    };

    // If split is required, add a consumer for it at the end.
    //
    info!("Running job #{} with {} tasks.", job.id, job.list.len());
    if sopts.split.is_some() {
        let basedir = sopts.split.as_ref().unwrap();

        // Store must be the last one, it is a pure consumer
        //
        let store = Task::from(Store::new(basedir, job.id)?);
        let _ = job.add(store);

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
    engine.remove_job(job.id)
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

/// Check the validity and mutual exclusivity of some of the streaming arguments.
///
/// # Parameters
/// - `opts`: A reference to `StreamOpts` containing the CLI options provided by the user.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` if all arguments are valid. Returns an error if there are conflicting or invalid arguments.
///
/// # Description
/// This function ensures that the arguments provided via `StreamOpts` are valid and do not conflict.
///
/// - Ensures that the `today` option is not used simultaneously with `begin` or `end`.
/// - Validates that `begin` and `end` are either both provided or neither is set.
///
/// # Errors
/// This function will return an error if:
/// - Both `today` and either `begin` or `end` options are specified.
/// - Only one of `begin` or `end` is provided (requires both or none).
///
#[tracing::instrument]
fn check_args(opts: &StreamOpts) -> Result<()> {
    trace!("check_args");

    // Do we have options for filter
    //
    if opts.today && (opts.begin.is_some() || opts.end.is_some()) {
        return Err(Status::TodayOrBeginEnd.into());
    }

    if (opts.begin.is_some() && opts.end.is_none()) || (opts.begin.is_none() && opts.end.is_some())
    {
        return Err(Status::BothOrNone.into());
    }

    Ok(())
}
