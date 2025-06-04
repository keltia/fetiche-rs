//! This is the module handling the `fetch` sub-command.
//!

use std::env;
use std::path::{absolute, Path};
use std::time::Duration;

use eyre::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::fs;
use tracing::field::debug;
use tracing::{debug, info, trace};

use crate::FetchOpts;

use fetiche_common::DateOpts;
use fetiche_engine::{Engine, Filter, JobState};

/// Fetches data from a specified network site using the provided engine and options.
///
/// # Parameters
/// - `engine`: A mutable reference to the `Engine` instance that will execute the fetch operation
/// - `fopts`: A reference to `FetchOpts` containing the fetch configuration options
///
/// # Returns
/// Returns `Result<()>` which is:
/// - `Ok(())` if the fetch operation completed successfully
/// - `Err(_)` if any error occurred during the operation
///
/// # Errors
/// This function may return an error if:
/// - The filter creation from options fails
/// - The HCL conversion of the filter fails
/// - The job parsing fails
/// - The job execution fails
///
#[tracing::instrument(
    skip(engine),
    fields(
        site = %fopts.site,
        output = ?fopts.output,
        has_filter = %fopts.dates.is_some()
    )
)]
pub async fn fetch_from_site(engine: &mut Engine, fopts: &FetchOpts) -> Result<()> {
    trace!("fetch_from_site({:?})", fopts.site);

    let name = &fopts.site;

    info!("Fetching from network site {}", name);
    info!("args: {:?}", fopts.dates);
    let filter = hcl::to_string(&filter_from_opts(fopts)?)?;
    info!("filter: {:?}", filter);

    // Do we want a copy of the raw data (often before converting it)
    //
    let tee = fopts.tee.clone().unwrap_or(String::from(""));

    // Are we writing to stdout?
    //
    let final_output = match fopts.output.clone() {
        Some(path) => {
            let fname = Path::new(&path);
            if fname.is_absolute() {
                path.clone()
            } else {
                debug!("output path: {:?}", fname);
                let fname = absolute(fname)?;
                let fname = env::current_dir()?.join(fname);
                fname.to_string_lossy().to_string()
            }
        }
        None => String::from("-"),
    };
    info!("Writing to {final_output}");

    let bar = ProgressBar::new_spinner().with_style(
        ProgressStyle::default_spinner().template("{spinner:.green} [{elapsed_precise}] {msg}")?,
    );

    bar.enable_steady_tick(Duration::from_millis(100));

    // FIXME: only supports `Save`  as output consumer.
    //
    let script = format!(
        r##"
    name = "Fetch from {name}"
    producer = {{
      "Fetch" = [
        "{name}",
        {{
          {filter}
        }}
      ]
    }}
    middle = [ {tee} ]
    output = {{
      "Save" = "{final_output}"
    }}
    "##
    );

    debug!("script = {script}");

    let job = engine.parse_job(&script).await?;
    let id = job.id;
    trace!("Job parsed: {:?}", job);

    trace!("Job submitting id #{id}");
    assert_eq!(job.state(), JobState::Ready);
    let res = engine.submit_job_and_wait(job).await?;

    trace!("Job result: {:?}", res);
    bar.finish();

    // Remove job from engine and state
    //
    trace!("Job({}) done.", id);

    Ok(engine.cleanup().await?)
}

/// Generates a `Filter` from the provided `FetchOpts`.
///
/// # Parameters
/// - `opts`: A reference to `FetchOpts` containing the options for creating the middle.
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
    match &opts.dates {
        Some(dates) => {
            let (begin, end) = DateOpts::parse(dates.clone())?;
            Ok(Filter::Interval { begin, end })
        }
        None => {
            if opts.keyword.is_some() {
                let keyword = opts.keyword.clone().unwrap();

                let v: Vec<_> = keyword.split(':').collect();
                Ok(Filter::Keyword {
                    name: v[0].to_string(),
                    value: v[1].to_string(),
                })
            } else if let Some(d) = opts.since {
                Ok(Filter::Duration(d))
            } else {
                Ok(Filter::default())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assert, prop_assert_eq, proptest};

    #[test]
    fn test_filter_from_opts_with_dates() {
        let opts = FetchOpts {
            dates: Some(DateOpts::From {
                begin: "2024-01-01".into(),
                end: "2024-01-02".into(),
            }),
            keyword: None,
            since: None,
            ..Default::default()
        };

        let filter = filter_from_opts(&opts).unwrap();
        match filter {
            Filter::Interval { begin, end } => {
                assert_eq!(begin.to_rfc3339(), "2024-01-01T00:00:00+00:00");
                assert_eq!(end.to_rfc3339(), "2024-01-02T00:00:00+00:00");
            }
            _ => panic!("Expected Interval middle"),
        }
    }

    #[test]
    fn test_filter_from_opts_with_keyword() {
        let opts = FetchOpts {
            dates: None,
            keyword: Some("field:value".to_string()),
            since: None,
            ..Default::default()
        };

        let filter = filter_from_opts(&opts).unwrap();
        match filter {
            Filter::Keyword { name, value } => {
                assert_eq!(name, "field");
                assert_eq!(value, "value");
            }
            _ => panic!("Expected Keyword middle"),
        }
    }

    #[test]
    fn test_filter_from_opts_with_duration() {
        let opts = FetchOpts {
            dates: None,
            keyword: None,
            since: Some(3600),
            ..Default::default()
        };

        let filter = filter_from_opts(&opts).unwrap();
        match filter {
            Filter::Duration(d) => {
                assert_eq!(d, 3600);
            }
            _ => panic!("Expected Duration middle"),
        }
    }

    #[test]
    fn test_filter_from_opts_default() {
        let opts = FetchOpts {
            dates: None,
            keyword: None,
            since: None,
            ..Default::default()
        };

        let filter = filter_from_opts(&opts).unwrap();
        let def = Filter::default();
        assert_eq!(filter, def);
    }

    proptest! {
        #[test]
        fn test_filter_from_opts_proptest(
            keyword in "[a-zA-Z0-9]+:[a-zA-Z0-9]+",
            since in 1..100000_i32
        ) {
            let opts = FetchOpts {
                keyword: Some(keyword.clone()),
                since: Some(since),
                ..Default::default()
            };
            let filter = filter_from_opts(&opts).unwrap();
            match filter {
                Filter::Keyword { name, value } => {
                    let parts: Vec<_> = keyword.split(':').collect();
                    prop_assert_eq!(name, parts[0]);
                    prop_assert_eq!(value, parts[1]);
                }
                _ => prop_assert!(false),
            }
        }
    }
}
