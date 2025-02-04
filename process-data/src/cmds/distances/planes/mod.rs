//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::env;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use derive_builder::Builder;
use eyre::Result;
use futures::future::join_all;
use itertools::Itertools;
use tracing::{debug, error, info, trace};

use fetiche_common::{expand_interval, normalise_day, DateOpts};

use crate::cmds::{enumerate_sites, find_site, Calculate, PlanesStats, Site, Stats};
use crate::error::Status;
use crate::runtime::Context;

mod compute;

/// Command-line options for the `distances planes` functionality.
///
/// This structure captures the user-provided options for executing distance
/// calculations between a drone and planes in specific geographical areas and
/// time intervals.
///
/// # Fields
///
/// - `date`: The day or interval of days for which to compute distances. This is
///   passed as a subcommand and typically provides a range of dates for processing.
/// - `name`: The name of the site or station around which to perform calculations.
///   If not provided, all stations will be processed by default.
/// - `distance`: The maximum range, in Nautical Miles, to consider for distance
///   calculations. Planes outside this range will not be included in the computation.
///   Defaults to `70.0` Nautical Miles if unspecified.
/// - `threshold`: The proximity threshold, in meters, to consider during calculations.
///   Planes closer than this threshold to the drone site are flagged for analysis.
///   Defaults to `1852.0` meters or a Nautical Mile.
/// - `factor`: We consider two circles around, one at threshold and one at factor * threshold.
///   Defaults to `3.0` if unspecified.
///
/// This structure is based on `clap::Parser` to provide seamless integration
/// with command-line argument parsing.
///
#[derive(Clone, Debug, Parser)]
pub struct PlanesOpts {
    /// Do calculation(s) on this/these day(s)
    #[clap(subcommand)]
    pub date: DateOpts,
    /// Do calculations around this station (default is all)
    pub name: Option<String>,
    /// Distance around the site in Nautical Miles.
    #[clap(short = 'D', long, default_value = "70.")]
    pub distance: f64,
    /// Proximity in Meters.
    #[clap(short = 't', long, default_value = "1852")]
    pub threshold: u32,
    /// Factor for proximity calculations.
    #[clap(short = 'f', long, default_value = "3")]
    pub factor: u32,
}

// -----

/// Represents the context for calculating the distance between a drone and planes.
///
/// This structure is designed to encapsulate the necessary information required to perform
/// distance calculations for a specific site on a given day. It includes parameters such as
/// geographical coordinates, maximum distances, proximity thresholds, and other auxiliary
/// data. Temporary tables created during computation are also managed within this structure.
///
/// # Fields
///
/// - `site`: Specifies the location (site) for which calculations are performed.
/// - `date`: The day for which distance calculations are to be executed.
/// - `wait`: An optional delay (in seconds) between tasks during processing.
/// - `distance`: Defines the maximum range of distance (in Nautical Miles) to be considered
///   during calculations. Defaults to `70.0`.
/// - `separation`: Specifies the proximity threshold (in meters). Defaults to `5500.0`.
/// - `lat`: Latitude of the antenna at the site.
/// - `lon`: Longitude of the antenna at the site.
/// - `state`: A record of temporary tables created during the calculation process for cleanup purposes.
///
/// # Notes
///
/// This structure uses the `derive_builder` crate to enable the construction of instances
/// through a builder pattern. Default values for certain fields (such as `distance`,
/// `separation`, and `state`) ensure that users can create instances without explicitly
/// specifying these values. The `state` field is particularly useful for tracking intermediate
/// computation data, which needs to be properly cleaned up to maintain system integrity.
///
#[derive(Builder, Debug)]
pub struct PlaneDistance {
    /// Name of site
    pub site: Site,
    /// Specific day
    pub date: DateTime<Utc>,
    /// Optional delay between tasks
    pub wait: u64,
    /// Max distance we want to consider
    #[builder(default = "70.")]
    pub distance: f64,
    /// Separation step.
    #[builder(default = "1852.")]
    pub threshold: f64,
    /// Separation factor
    #[builder(default = "3.")]
    pub factor: f64,
    /// Lat of antenna
    #[builder]
    pub lat: f64,
    /// Lon of antenna
    #[builder]
    pub lon: f64,
    /// List of temporary tables created along the way, for cleanup.
    #[builder(default = "vec![]")]
    state: Vec<TempTables>,
}

/// Temporary tables created during the processing of distance calculations.
///
/// These tables are used as intermediate storage at various stages of the computation.
/// Depending on the circumstances, one or more of these tables may be created, and they
/// must be cleaned up at the end of the process. Each variant represents a specific stage
/// in the calculation workflow:
///
/// - `Today`: Contains data relevant to the given day of the calculation.
/// - `Candidates`: Stores potential candidates that match certain criteria.
/// - `TodayClose`: Tracks close proximity calculations for a specific day.
/// - `Ids`: Keeps identifiers for entities involved in the computation.
///
/// The `state` attribute in the `PlaneDistance` structure maintains a record of these
/// tables to ensure proper cleanup during the process.
///
#[derive(Clone, Debug)]
pub enum TempTables {
    Today,
    Candidates,
    TodayClose,
    Ids,
}

// -----

const ALL_SITES: &str = "ALL"; // Introduced constant for clarity

/// Performs the calculation of distances between drones and planes for a specified set of
/// sites and dates. The function handles directory management, date parsing, work list
/// preparation, and concurrent computation of distance statistics.
///
/// # Arguments
///
/// * `ctx` - The execution context containing configuration and runtime details.
/// * `opts` - Options specifying the parameters for distance calculations, such as the site
///   name, date interval, maximum distance, and proximity threshold.
///
/// # Returns
///
/// This function returns a `Result` containing `Stats`, which encapsulates summarized
/// statistics from the distance calculations for all processed sites and dates. In the case
/// of an error, it provides relevant diagnostic details.
///
/// # Steps
///
/// 1. Set the working directory to the configured datalake.
/// 2. Parse the date interval specified in `opts`.
/// 3. Generate a flattened work list combining all relevant sites and dates.
/// 4. Process the work list in parallel by computing batches of distance statistics.
/// 5. Aggregate and summarize the results into final statistics.
///
/// This method uses asynchronous operations to handle potentially long-running tasks such
/// as reading site data and performing calculations in parallel. Tracing annotations provide
/// insight into the flow of execution for debugging and monitoring.
///
#[tracing::instrument(skip(ctx))]
pub async fn planes_calculation(ctx: &Context, opts: &PlanesOpts) -> Result<Stats> {
    // Step 1: Change working directory to the datalake
    //
    let datalake = ctx.config.get("datalake").unwrap();
    info!("Datalake: {}", datalake);
    env::set_current_dir(datalake)?;

    // Step 2: Parse dates
    //
    let (begin, end) = parse_date_interval(opts.date.clone())?;
    let dates = expand_interval(begin, end)?;
    eprintln!("{} days to process: from {begin} to {end}", dates.len());

    // Step 3: Create work list (combination of dates and sites)
    //
    let site_filter = opts.name.as_deref().unwrap_or("");
    let work_list = prepare_work_list(ctx, dates, site_filter).await?;

    // Pass down the parameters for calculations.
    //
    let threshold = match ctx.config.get("threshold") {
        Some(v) => v.parse::<f64>().unwrap_or(1852.),
        None => 1852.,
    };

    let factor = match ctx.config.get("factor") {
        Some(v) => v.parse::<f64>().unwrap_or(3.),
        None => 3.,
    };

    // Step 4: Process batches of computations in parallel
    //
    let all_stats = process_batches(ctx, work_list, opts.distance, threshold, factor).await;

    // Step 5: Gather and summarize statistics
    //
    let stats = Stats::summarise(all_stats);
    trace!("summary={stats:?}");
    Ok(stats)
}

/// Prepares the work list for distance calculations by generating combinations of
/// dates and sites to be processed. Depending on the provided site middle, the function
/// either targets a specific site or includes all available sites across the given dates.
///
/// # Arguments
///
/// * `ctx` - The execution context containing configuration and runtime details.
/// * `dates` - A vector of dates that define the period for distance calculation.
/// * `site_filter` - A middle specifying a particular site or all sites (`ALL` or `*`).
///
/// # Returns
///
/// Returns a `Result` containing a vector of tuples `(DateTime<Utc>, Site)` where each
/// tuple represents a specific date and site to process during the computation.
///
/// # Behavior
///
/// - If `site_filter` is set to a specific site name, the result contains combinations
///   of the specified site and all input dates.
/// - If `site_filter` is empty or set to `ALL_SITES`, the result includes combinations
///   of all available sites and the input dates (computed dynamically).
///
/// # Error Handling
///
/// Returns an error if the site resolution or site enumeration fails during the preparation process.
///
/// This function uses asynchronous operations to interact with the execution context for
/// resolving sites and enumerating available sites.
///
#[tracing::instrument(skip(ctx))]
async fn prepare_work_list(
    ctx: &Context,
    dates: Vec<DateTime<Utc>>,
    site_filter: &str,
) -> Result<Vec<(DateTime<Utc>, Site)>> {

    // Let us generate the list we want:
    //
    // if there is only one site we want then
    //     [(day0, site), (day1, site) .. (dayN, site)]
    // otherwise we want for each day, all sites
    //     [(day0, site0), (day0, site1) .. (day0, siteN), (day1, site0) ...]
    //
    // Basically zip together the two iterators, even if there is only one
    //
    // Goal is to have a flattened list of all combinations to run these in parallel
    //
    let name = match site_filter {
        ALL_SITES | "*" => "",
        _ => site_filter,
    };
    trace!("Site = {name} (all if empty)");

    let work_list: Vec<_> = dates
        .iter()
        .map(|&day| async move {
            // We have a specific site
            //
            if !name.is_empty() {
                let site = find_site(ctx, name).await.unwrap();
                let res = vec![(day, site)];
                res
            } else {
                // Process all sites
                //
                let list = enumerate_sites(ctx, day).await.unwrap();
                let list: Vec<_> = list.iter().map(|site| (day, site.clone())).collect();
                list
            }
        })
        .collect::<Vec<_>>();
    let work_list = join_all(work_list).await;

    // Now flatten it
    //
    let work_list: Vec<_> = work_list.into_iter().flatten().collect::<Vec<_>>();
    trace!("Work list len = {}", work_list.len());
    Ok(work_list)
}

/// Processes batches of distance and separation computations in parallel.
///
/// This function takes a list of `(DateTime<Utc>, Site)` tuples and processes
/// them in parallel using asynchronous tasks. Each batch size is limited by
/// the pool size defined in the context to optimize execution performance.
///
/// # Arguments
///
/// * `ctx` - The execution context containing runtime configurations such as pool size.
/// * `work_list` - A vector of tuples `(DateTime<Utc>, Site)` representing the work to process.
/// * `distance` - The maximum permissible distance used in the calculations.
/// * `separation` - The minimum required separation for valid calculations.
///
/// # Returns
///
/// Returns a vector of `Stats`, where each entry represents the computation results
/// for a specific day and site combination.
///
/// # Behavior
///
/// - The function processes the work list in batches according to the pool size.
/// - For each `(DateTime<Utc>, Site)` tuple, it spawns an asynchronous task to perform the computation.
/// - Errors during individual computations are logged, and default statistics are returned
///   for the corresponding entry.
///
/// # Error Handling
///
/// Errors in individual calculations are captured and logged, but they do not halt the
/// processing of other batches. Instead, default statistics are generated for the failed tasks.
///
/// # Execution
///
/// The asynchronous tasks are executed concurrently within each batch, and batches
/// are processed sequentially to avoid overloading the task scheduler or environment.
///
#[tracing::instrument(skip(ctx))]
async fn process_batches(
    ctx: &Context,
    work_list: Vec<(DateTime<Utc>, Site)>,
    distance: f64,
    threshold: f64,
    factor: f64,
) -> Vec<Stats> {

    // We have a potentially large set of day+site to compute.  Try to not batch more than out current
    // pool size
    //
    let mut all = vec![];
    for batch in &work_list.into_iter().chunks(ctx.pool_size) {
        let stats: Vec<_> = batch
            .into_iter()
            .map(|(day, site)| async move {
                trace!("Calculate for site {site} on day {day}");
                let current = site.clone();
                let ctx = ctx.clone();

                match tokio::spawn(async move {
                    calculate_one_day_on_site(&ctx, &current, &day, distance, threshold, factor)
                        .await
                        .unwrap()
                })
                    .await {
                    Ok(res) => res,
                    Err(e) => {
                        error!("Error for day {day} on {site}: {}", e.to_string(), day = day);
                        Stats::Planes(PlanesStats::default())
                    }
                }
            })
            .collect();
        let stats: Vec<_> = join_all(stats).await;
        all.push(stats);
    }
    let all = all.into_iter().flatten().collect::<Vec<_>>();
    debug!("all={all:?}");

    all
}

/// Parses the provided `DateOpts` into a start and end date range.
///
/// This function utilizes the `DateOpts::parse` method to interpret and convert
/// the given date options into a corresponding date interval `(start, stop)`
/// represented as `DateTime<Utc>`. If the parsing fails, it defaults to the
/// current day.
///
/// # Arguments
///
/// * `date_opts` - A `DateOpts` variant that specifies the desired date range or format.
///
/// # Returns
///
/// A `Result` containing:
/// - `Ok((start, stop))` if the date options are successfully parsed.
/// - `Err` if any error occurs while parsing or normalizing the dates.
///
/// # Behavior
///
/// - If `date_opts` is valid, the function logs the interval and returns
///   the calculated start and stop dates.
/// - If an error occurs, it defaults to the current day, logs the fallback, 
///   and returns both `start` and `stop` as the start of the current day.
///
/// # Examples
///
/// Valid date range:
/// ```rust
/// let date_opts = DateOpts::From { 
///     begin: "2023-10-01T00:00:00Z".to_string(),
///     end: "2023-10-10T00:00:00Z".to_string() 
/// };
/// let result = parse_date_interval(date_opts).unwrap();
/// assert_eq!(result.0, Utc.with_ymd_and_hms(2023, 10, 1, 0, 0, 0).unwrap());
/// assert_eq!(result.1, Utc.with_ymd_and_hms(2023, 10, 10, 0, 0, 0).unwrap());
/// ```
///
/// Invalid date fallback:
/// ```rust
/// let date_opts = DateOpts::Week { num: 66 };
/// let result = parse_date_interval(date_opts).unwrap();
/// let now = Utc::now();
/// let expected_day = Utc
///     .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
///     .unwrap();
/// assert_eq!(result.0.date_naive(), expected_day.date_naive());
/// assert_eq!(result.1.date_naive(), expected_day.date_naive());
/// ```
///
#[tracing::instrument]
fn parse_date_interval(date_opts: DateOpts) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    match DateOpts::parse(date_opts) {
        Ok((start, stop)) => {
            info!("Interval: from {} to {}", start, stop);
            Ok((start, stop))
        }
        Err(_) => {
            let tm = Utc::now();
            let day = Utc.with_ymd_and_hms(tm.year(), tm.month(), tm.day(), 0, 0, 0).unwrap();
            info!("Defaulting to current day: {}", day);
            Ok((tm, tm))
        }
    }
}


/// Perform the calculation for a specific day and a specific site.
///
/// This function is responsible for calculating the plane distances for
/// a given site on a specific day. It takes into account the provided
/// parameters such as distance and separation, and uses the database
/// connection to process and store the results. It builds the necessary
/// input using the `PlaneDistanceBuilder` and executes the calculation.
///
/// # Arguments
///
/// * `ctx` - The application context containing configurations and resources.
/// * `site` - A reference to the `Site` for which the calculation is being performed.
/// * `day` - The date for which the calculation is intended.
/// * `distance` - Maximum distance to be considered for calculations.
/// * `separation` - Minimum proximity for the calculations.
///
/// # Returns
///
/// Returns a `Result` containing `Stats` on success, or an error type if the calculation fails.
///
/// # Errors
///
/// This function will return an error in the following cases:
/// * Failure to acquire a database connection.
/// * Issues during the normalization of the provided day.
/// * Errors occurring during the building of the `PlaneDistance` object.
/// * Errors occurring while running the actual calculation process.
///
/// # Examples
///
/// ```rust
/// let stats = calculate_one_day_on_site(&ctx, &site, &day, 70.0, 5500.0).await?;
/// println!("Calculated stats: {:?}", stats);
/// ```
///
/// # Note
///
/// If the `dry_run` setting is enabled in the context, this function will not run real calculations.
/// Instead, it will return a default `PlanesStats` result without modifying any data.
///
#[tracing::instrument(skip(ctx))]
async fn calculate_one_day_on_site(
    ctx: &Context,
    site: &Site,
    day: &DateTime<Utc>,
    distance: f64,
    threshold: f64,
    factor: f64,
) -> Result<Stats> {
    let dbh = ctx
        .dbh
        .get()
        .await
        .map_err(|e| Status::ConnectionUnavailable(e.to_string()))?;

    let day = normalise_day(*day)?;

    let mut work = PlaneDistanceBuilder::default()
        .site(site.clone())
        .lat(site.latitude as f64)
        .lon(site.longitude as f64)
        .distance(distance)
        .date(day)
        .threshold(threshold)
        .factor(factor)
        .wait(ctx.wait)
        .build()?;

    trace!("worklist for {:?} on {}: {:?}", site.name, day, work);

    // We use rayon to reduce the overhead during parallel calculations
    //

    let stats = if !ctx.dry_run {
        work.run(&dbh).await?
    } else {
        trace!("dry run!");
        Stats::Planes(PlanesStats::default())
    };
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    use crate::cli::{Opts, SubCommand};
    use crate::cmds::{DistOpts, DistSubcommand};
    use crate::runtime::init_runtime;

    #[test]
    fn test_parse_date_interval_valid_range() {
        let start_date = "2023-10-01T00:00:00Z";
        let end_date = "2023-10-10T00:00:00Z";
        let date_opts = DateOpts::From { begin: start_date.to_string(), end: end_date.to_string() };

        let result = parse_date_interval(date_opts).unwrap();
        assert_eq!(result.0, Utc.with_ymd_and_hms(2023, 10, 1, 0, 0, 0).unwrap());
        assert_eq!(result.1, Utc.with_ymd_and_hms(2023, 10, 10, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_date_interval_invalid_range_defaults_to_now() {
        let invalid_date_opts = DateOpts::Week { num: 66 };
        let now = Utc::now();
        let expected_day = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .unwrap();

        let result = parse_date_interval(invalid_date_opts).unwrap();
        assert_eq!(result.0.date_naive(), expected_day.date_naive());
        assert_eq!(result.1.date_naive(), expected_day.date_naive());
    }

    // This test *requires* an configured account (various Clickhouse related environment variables, etc.)
    // and an active database, etc.
    //
    #[tokio::test]
    async fn test_prepare_work_list() -> Result<()> {
        let site = "*";

        // This is present but not really used.
        //
        let dopts = DistOpts {
            output: None,
            subcmd: DistSubcommand::Planes(PlanesOpts {
                date: DateOpts::Week { num: 1 },
                name: Some(site.to_string()),
                distance: 70.0,
                threshold: 1852,
                factor: 3,
            }),
        };
        let cmd = SubCommand::Distances(dopts);

        // Minimal "configuration"
        //
        let opts = Opts {
            config: None,
            database: Some("acute".into()),
            datalake: Some("/Users/acute".into()),
            wait: 0,
            pool_size: 1,
            use_telemetry: false,
            use_tree: true,
            dry_run: true,
            use_file: None,
            subcmd: cmd,
        };

        // Just to establish the context and database connection.
        //
        let ctx = init_runtime(&opts).await?;

        let b = dateparser::parse("2023-10-01T00:00:00Z").unwrap();
        let e = dateparser::parse("2023-10-02T00:00:00Z").unwrap();
        let dates = vec![b, e];

        let work_list = prepare_work_list(&ctx, dates, site).await?;
        dbg!(&work_list);
        assert_eq!(work_list.len(), 6);
        Ok(())
    }
}
