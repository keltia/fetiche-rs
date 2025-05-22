use std::time::Duration;

use eyre::Result;
use indicatif::ProgressBar;
use tracing::{debug, info, trace};

use fetiche_engine::{Engine, Filter, Freq, JobState};

use crate::{Status, StreamOpts};

#[tracing::instrument(skip(engine))]
pub async fn stream_from_site(engine: &mut Engine, sopts: &StreamOpts) -> Result<()> {
    check_args(sopts)?;

    let name = &sopts.site;

    info!("Streaming from network site {}", name);
    let filter = hcl::to_string(&filter_from_opts(sopts)?)?;
    info!("filter: {:?}", filter);

    // Do we want a copy of the raw data (often before converting it)
    //
    let tee = sopts.tee.clone().unwrap_or(String::from(""));

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

    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));

    let script = format!(r##"
    name = "Stream from {name}"
    type = "stream"
    producer = {{
      "Stream" = [
        "{name}",
        {{
            {filter}
        }}
      ]
    }}
    middle = [ {tee} ]
    {output}
    "##);

    debug!("script = {script}");

    let job = engine.parse_job(&script).await?;
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
