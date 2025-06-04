use std::env;
use std::path::{absolute, Path};
use std::time::Duration;

use eyre::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{debug, info, trace};

use fetiche_client::{EngineSingle, Filter, Freq, JobBuilder, JobState};

use crate::{Status, StreamOpts};

#[tracing::instrument(skip(engine))]
pub async fn stream_from_site(engine: &mut EngineSingle, sopts: &StreamOpts) -> Result<()> {
    check_args(sopts)?;

    let name = &sopts.site;

    info!("Streaming from network site {}", name);
    let filter = hcl::to_string(&filter_from_opts(sopts)?)?;
    info!("filter: {:?}", filter);

    // Do we want a copy of the raw data (often before converting it)
    //
    let tee = if let Some(path) = &sopts.tee {
        let fname = Path::new(&path);
        let fname = if fname.is_absolute() {
            path.clone()
        } else {
            debug!("tee path: {:?}", fname);
            let fname = absolute(fname)?;
            let fname = env::current_dir()?.join(fname);
            fname.to_string_lossy().to_string()
        };
        Some(fname)
    } else {
        None
    };

    // Analyse our output strategy
    //
    let freq = sopts.frequency.clone().unwrap_or(Freq::Daily);
    let output = if let Some(split) = &sopts.store {
        format!(r##"
        output = {{
            "Freq" = {}
            "Store" = "{}"
        }}
        "##, freq, split)
    } else if let Some(fname) = &sopts.output {
        format!(r##"
        output = {{
            "Save" = "{}"
        }}
        "##, fname)
    } else {
        format!(r##"
        output = {{
            "Save" = "-"
        }}
        "##)
    };

    info!("Writing to {output}");
    eprintln!("Streaming into {output}");

    let bar = ProgressBar::new_spinner().with_style(
        ProgressStyle::default_spinner().template("{spinner:.green} [{elapsed_precise}] {msg}")?,
    );
    bar.enable_steady_tick(Duration::from_millis(100));

    let job = JobBuilder::new(&format!("Stream from {name}"))
        .stream(&sopts.site)
        .filter(filter_from_opts(sopts)?)
        .tee(tee)
        .store(&output, freq)
        .build();
    dbg!(&job);

    let job = engine.parse_job(job).await?;
    let id = job.id;
    trace!("Job running id #{id}");

    trace!("Job submitting id #{id}");
    assert_eq!(job.state(), JobState::Ready);
    let _ = engine.submit_job_and_wait(job).await?;

    bar.finish();

    trace!("Job({}) done, removing it.", id);

    Ok(())
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

    // Do we have options for middle
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
