//! This is the module handling the `fetch` sub-command.
//!

use eyre::Result;
use indicatif::ProgressBar;
use std::time::Duration;
use tracing::{debug, info, trace};

use fetiche_common::DateOpts;
use fetiche_engine::{Engine, Filter, JobState};

use crate::FetchOpts;

#[tracing::instrument(skip(engine))]
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
    let final_output = fopts.output.clone().unwrap_or(String::from("-"));

    info!("Writing to {final_output}");
    eprintln!("Fetching into {final_output}");

    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));

    let script = format!(r##"
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
    "##);

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

    Ok(())
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

    #[test]
    fn test_filter_from_opts_with_dates() {
        let opts = FetchOpts {
            dates: Some(DateOpts::Day { date: "2024-01-01..2024-01-02".to_string() }),
            keyword: None,
            since: None,
            ..Default::default()
        };

        let filter = filter_from_opts(&opts).unwrap();
        match filter {
            Filter::Interval { begin, end } => {
                assert_eq!(begin.to_string(), "2024-01-01T00:00:00Z");
                assert_eq!(end.to_string(), "2024-01-02T00:00:00Z");
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
        assert!(matches!(filter, Filter::default()));
    }
}
