//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::env;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use derive_builder::Builder;
use eyre::{eyre, Result};
use futures::future::try_join_all;
use tracing::{debug, info, trace};

use fetiche_common::{expand_interval, normalise_day, DateOpts};

use crate::cmds::{enumerate_sites, find_site, Calculate, PlanesStats, Stats};
use crate::config::Context;

mod compute;

/// These are the options we pass to this command
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

/// This is the struct in which we store the context of a given day work.
///
#[derive(Builder, Debug)]
pub struct PlaneDistance {
    /// Name of site
    pub name: String,
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
    // Lat of antenna
    #[builder]
    pub lat: f64,
    // Lon of antenna
    #[builder]
    pub lon: f64,
}

// -----

/// Handle the `distances planes` command.
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
    info!("{} days to process.", dates.len());

    trace!("From {} to {}", begin, end);

    // Gather all sites to run calculations on for every day.
    //
    let stats = match &opts.name {
        Some(site) => {
            trace!("Calculate for site {site}");
            let day_stats = try_join_all(dates.iter().inspect(|&day| debug!("day = {day}")).map(
                |&day| async move {
                    debug!("site = [{site:?}]");
                    calculate_one_day_on_site(ctx, site, day, opts.distance, opts.separation).await
                },
            ))
            .await
            .map_err(|e| eyre!("error: {}", e))?;

            debug!("days({dates:?}) stats={day_stats:?}");
            day_stats
        }
        None => {
            trace!("Calculate for all valid sites.");
            let day_stats =
                try_join_all(dates.iter().inspect(|&day| debug!("day = {day}")).map(
                    |&day| async move {
                        calculate_one_day(ctx, day, opts.distance, opts.separation).await
                    },
                ))
                .await
                .map_err(|e| eyre!("error: {}", e))?;

            let stats = day_stats.into_iter().flatten().collect::<Vec<_>>();
            debug!("days({dates:?}) stats={stats:?}");
            stats
        }
    };

    // Gather all statistics
    //
    let stats = Stats::summarise(stats);
    trace!("summary={stats:?}");

    Ok(stats)
}

/// Does the calculation for one specific day on one specific site.
/// Find all sites for which the day is valid and run these.
///
#[tracing::instrument(skip(ctx))]
async fn calculate_one_day(
    ctx: &Context,
    day: DateTime<Utc>,
    distance: f64,
    separation: f64,
) -> Result<Vec<Stats>> {
    let dbh = ctx.db();

    // Build our set of batches
    //
    let day = normalise_day(day)?;
    let sites = enumerate_sites(ctx, day).await?;

    let worklist = sites
        .iter()
        .map(|site| {
            let name = site.name.clone();
            let work = PlaneDistanceBuilder::default()
                .name(name)
                .lat(site.latitude as f64)
                .lon(site.longitude as f64)
                .distance(distance)
                .date(day)
                .separation(separation)
                .wait(ctx.wait)
                .build()
                .unwrap();

            work
        })
        .collect::<Vec<_>>();

    trace!("worklist for {:?}: {:?}", day, worklist);

    let stats: Vec<Stats> = try_join_all(worklist.iter().map(|task| {
        let dbh = dbh.clone();
        async move {
            if !ctx.dry_run {
                task.run(&dbh.clone())
                    .await
                    .map_err(|e| eyre!("error: {e}"))
            } else {
                trace!("dry run!");
                Ok(Stats::Planes(PlanesStats::default()))
            }
        }
    }))
    .await?;

    trace!("All stats: {:?}", stats);

    Ok(stats)
}

/// Does the calculation for one specific day on one specific site.
/// Could be merged with previous, but I think it might be too much overhead for just a few lines.
///
#[tracing::instrument(skip(ctx))]
async fn calculate_one_day_on_site(
    ctx: &Context,
    site: &str,
    day: DateTime<Utc>,
    distance: f64,
    separation: f64,
) -> Result<Stats> {
    let dbh = ctx.db();

    let day = normalise_day(day)?;
    let site = find_site(ctx, site).await?;

    let name = site.name.clone();
    let work = PlaneDistanceBuilder::default()
        .name(name)
        .lat(site.latitude as f64)
        .lon(site.longitude as f64)
        .distance(distance)
        .date(day)
        .separation(separation)
        .wait(ctx.wait)
        .build()
        .unwrap();

    trace!("worklist for {:?} on {}: {:?}", site.name, day, work);

    // We use rayon to reduce the overhead during parallel calculations
    //

    let stats = if !ctx.dry_run {
        work.run(&dbh.clone()).await?
    } else {
        trace!("dry run!");
        Stats::Planes(PlanesStats::default())
    };
    Ok(stats)
}
