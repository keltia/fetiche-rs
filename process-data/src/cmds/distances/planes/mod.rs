//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::env;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use derive_builder::Builder;
use eyre::Result;
use futures::future::{join_all, try_join_all};
use tracing::{info, trace};

use fetiche_common::{expand_interval, normalise_day, DateOpts};

use crate::cmds::{enumerate_sites, find_site, Calculate, PlanesStats, Site, Stats};
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

/// This is the list of temporary tables created during the calculations process.  As we can
/// bail out a certain points, the cleanup process may include one or more of these tables.
/// This is registered in the `state` attribute in `PlaneDistance`.
///
#[derive(Clone, Debug)]
pub enum TempTables {
    Today,
    Candidates,
    TodayClose,
    Ids,
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

    // Gather all sites to run calculations on for every day.
    //
    let stats: Vec<_> = work_list
        .into_iter()
        .map(|(day, site)| {
            trace!("Calculate for site {site} on day {day}");
            let site = site.clone();
            let ctx = ctx.clone();

            async move { calculate_one_day_on_site(&ctx, &site, &day, distance, separation).await }
        })
        .collect();
    let stats: Vec<_> = try_join_all(stats).await?;

    // Gather all statistics
    //
    let stats = Stats::summarise(stats);
    dbg!(&stats);
    trace!("summary={stats:?}");

    Ok(stats)
}

/// Does the calculation for one specific day on one specific site.
/// Could be merged with previous, but I think it might be too much overhead for just a few lines.
///
#[tracing::instrument(skip(ctx))]
async fn calculate_one_day_on_site(
    ctx: &Context,
    site: &Site,
    day: &DateTime<Utc>,
    distance: f64,
    separation: f64,
) -> Result<Stats> {
    let dbh = ctx.db().await;

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
        work.run(&dbh.clone()).await?
    } else {
        trace!("dry run!");
        Stats::Planes(PlanesStats::default())
    };
    Ok(stats)
}
