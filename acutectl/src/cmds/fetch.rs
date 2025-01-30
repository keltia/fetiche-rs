//! This is the module handling the `fetch` sub-command.
//!

use eyre::Result;
use indicatif::ProgressBar;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info, trace};

use fetiche_common::{Container, DateOpts};
use fetiche_engine::{Convert, Engine, Fetch, Save, Task, Tee};
use fetiche_formats::Format;
use fetiche_sources::{Filter, Flow};

use crate::{FetchOpts, Status};

/// Fetch data from a specific site, applying the provided filters and options.
///
/// # Parameters
///
/// - `engine`: Reference to the `Engine` instance used for managing tasks and jobs.
/// - `fopts`: A reference to `FetchOpts` containing the options for fetching data.
///
/// # Returns
///
/// Result indicating success or any error encountered while processing the fetch request.
///
/// # Description
///
/// This function orchestrates the fetching of data from a network site by:
/// - Validating the site as fetchable.
/// - Applying the desired filters and transformations based on user options.
/// - Creating a job with necessary tasks such as fetch, optional conversion, and output saving.
/// - Managing the progress bar and job lifecycle.
///
/// # Errors
///
/// This function will return an error if:
/// - The specified site is not fetchable.
/// - There is an issue with applying file formatting options.
/// - The fetching or saving process encounters an issue.
///
#[tracing::instrument(skip(engine))]
pub async fn fetch_from_site(engine: &mut Engine, fopts: &FetchOpts) -> Result<()> {
    trace!("fetch_from_site({:?})", fopts.site);

    let name = &fopts.site;
    let srcs = engine.sources().await?;
    let site = srcs.load(name)?;
    match site {
        Flow::Fetchable(ref s) => s,
        _ => {
            error!("Site {} is not Fetchable!", site.name());
            return Err(Status::SiteNotFetchable(site.name()).into());
        }
    };

    let filter = filter_from_opts(fopts)?;

    info!("Fetching from network site {}", name);

    let mut task = Fetch::new(name, srcs.into());
    task.site(site.name()).with(filter);

    let task = Task::from(task);

    let mut data = vec![];

    let mut job = engine.create_job("fetch_from_site").await?;
    job.add(task);

    // Do we want a copy of the raw data (often before converting it)
    //
    if let Some(tee) = &fopts.tee {
        let copy = Task::from(Tee::into(tee));
        job.add(copy);
    }

    // If a conversion is requested, insert it
    //
    // FIXME: DEPRECATED
    //
    let input = if let Some(_into) = &fopts.into {
        let mut convert = Convert::new();
        convert.from(site.format()).into(Format::Cat21);
        let convert = Task::from(convert);
        job.add(convert);

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
    let save = Task::from(save);
    job.add(save);

    eprintln!("Fetching {final_output}");
    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));

    // Launch it now
    //
    job.run(&mut data)?;

    bar.finish();

    // Remove job from engine and state
    //
    trace!("Job({}) done, removing it.", job.id);
    engine.remove_job(job.id)
}

/// Generates a `Filter` from the provided `FetchOpts`.
///
/// # Parameters
/// - `opts`: A reference to `FetchOpts` containing the options for creating the filter.
///
/// # Returns
/// - `Result<Filter>`: A `Filter` object encapsulating the configured filtering options, or an error if the options are invalid.
///
/// # Description
/// This function processes CLI options to create the appropriate `Filter` object
/// based on user-provided flags and arguments such as date ranges, keywords, or duration.
///
/// # Errors
/// This function may return an error if:
/// - The `dates` argument cannot be parsed into a valid interval.
/// - Keyword arguments are improperly formatted (e.g., missing `:` separator).
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
