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
use tracing::{error, info, trace};

use fetiche_common::{expand_interval, normalise_day, DateOpts};

use crate::cmds::{enumerate_sites, find_site, Calculate, PlanesStats, Site, Stats};
use crate::config::Context;
use crate::error::Status;

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
/// - `separation`: The proximity threshold, in meters, to consider during calculations.
///   Planes closer than this threshold to the drone site are flagged for analysis.
///   Defaults to `5500.0` meters.
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
    #[clap(short = 'p', long, default_value = "5500.")]
    pub separation: f64,
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
    /// proximity
    #[builder(default = "5500.")]
    pub separation: f64,
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

/// This function processes the `distances planes` command. It handles the setup and execution of
/// distance calculations between a drone and various planes around specific sites on given days.
/// The calculations can span multiple dates and sites, and the results depend on user-specified
/// options such as distance, proximity, and the selected site(s). The function operates in parallel
/// for efficiency and uses temporary tables to facilitate intermediate data storage.
///
/// The results of the calculations are gathered into a `Stats` structure.
///
/// # Parameters
///
/// - `ctx`: The execution context containing configurations and utilities for processing.
/// - `opts`: User-specified options for the command, including site name, dates, distances, and separation.
///
/// # Returns
///
/// - `Result<Stats>`: Returns the statistics of the computation wrapped in a result.
///
/// # Behavior
///
/// 1. The function begins by setting the working directory to the datalake specified in the context.
/// 2. It parses the dates provided in the options or defaults to the current day.
/// 3. A worklist is generated containing all combinations of sites and dates.
/// 4. The calculations run in parallel batches, with each batch processing a subset of the worklist.
/// 5. Finally, the results are collected and returned as `Stats`.
///
/// # Notes
///
/// - Ensure appropriate handling of options like "ALL" or no site name to process multiple sites.
/// - Parallel execution requires careful batching to avoid overwhelming the thread pool.
/// - Errors in individual site/day calculations are logged, and default statistics are returned for those failures.
///
#[tracing::instrument(skip(ctx))]
pub async fn planes_calculation(ctx: &Context, opts: &PlanesOpts) -> Result<Stats> {
    // Move ourselves to the datalake.
    //
    let datalake = ctx.config.get("datalake").unwrap();

    info!("Datalake: {}", datalake);
    env::set_current_dir(datalake)?;

    // Load parameters
    //
    let (begin, end) = match DateOpts::parse(opts.date.clone()) {
        Ok((start, stop)) => {
            info!("We have an interval: from {} to {}", start, stop);
            (start, stop)
        }
        Err(_) => {
            let tm = Utc::now();
            let day = Utc
                .with_ymd_and_hms(tm.year(), tm.month(), tm.day(), 0, 0, 0)
                .unwrap();
            info!("Running calculations for {}:", day);
            (tm, tm)
        }
    };

    let dates = expand_interval(begin, end)?;
    eprintln!("{} days to process, from {begin} to {end}", dates.len());

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
    let name = match &opts.name {
        Some(name) => {
            if name == "ALL" || name == "*" {
                ""
            } else {
                name.as_str()
            }
        }
        None => "",
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

    let distance = opts.distance;
    let separation = opts.separation;

    // We have a potentially large set of day+site to compute.  Try to not batch more than out current
    // pool size
    let mut all = vec![];
    for batch in &work_list.into_iter().chunks(ctx.pool_size) {
        let stats: Vec<_> = batch
            .into_iter()
            .map(|(day, site)| async move {
                trace!("Calculate for site {site} on day {day}");
                let lsite = site.clone();
                let ctx = ctx.clone();

                match tokio::spawn(async move {
                    calculate_one_day_on_site(&ctx, &lsite, &day, distance, separation)
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

    // Gather all statistics
    //
    let stats = Stats::summarise(all);
    trace!("summary={stats:?}");

    Ok(stats)
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
    separation: f64,
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
        .separation(separation)
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
