//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::env;
use std::sync::Arc;

use chrono::{Datelike, DateTime, TimeZone, Utc};
use clap::Parser;
use clickhouse::Row;
use derive_builder::Builder;
use eyre::Result;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tracing::{info, trace};

use fetiche_common::{DateOpts, expand_interval, load_locations, Location};

use crate::cmds::{Calculate, Stats};
use crate::config::Context;
use crate::error::Status;

mod compute;

/// These are the options we pass to this command
///
#[derive(Clone, Debug, Parser)]
pub struct PlanesOpts {
    /// Do calculation(s) on this/these day(s)
    #[clap(subcommand)]
    pub date: DateOpts,
    /// Do calculations around this station.
    pub name: String,
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
    /// Coordinates of site
    pub loc: Arc<Location>,
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
    /// table name template for a run
    #[builder(setter(into, strip_option), default = "None")]
    pub template: Option<String>,
}

// -----

/// Handle the `distances planes` command.
///
#[tracing::instrument(skip(ctx))]
pub async fn planes_calculation(ctx: &Context, opts: &PlanesOpts) -> Result<Stats> {
    let dbh = ctx.db();

    #[derive(Debug, Deserialize, Row, Serialize)]
    struct Site {
        pub id: u32,
        name: String,
        code: String,
        basename: String,
        latitude: f32,
        longitude: f32,
        ref_alt: f32,
    }

    // Move ourselves to the datalake.
    //
    let datalake = ctx.config.get("datalake").unwrap();

    info!("Datalake: {}", datalake);
    env::set_current_dir(datalake)?;

    // Load locations from DB
    //
    let r = r##"
    SELECT * from sites WHERE name = ?
    "##;
    let name = opts.name.clone();
    let site = match dbh.query(r).bind(&name).fetch_one::<Site>().await {
        Ok(site) => site,
        Err(e) => return Err(Status::UnknownSite(name).into()),
    };
    let current = Arc::new(Location {
        code: site.code.clone(),
        hash: None,
        lat: site.latitude as f64,
        lon: site.longitude as f64,
    });

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
    trace!("From {} to {} on site {}", begin, end, &opts.name);

    let dates = expand_interval(begin, end)?;
    trace!("all days: {:?}", dates);
    info!("{} days to process.", dates.len());

    // Build our set of batches
    //
    let worklist: Vec<_> = dates.into_iter().map(|day| {
        let work = PlaneDistanceBuilder::default()
            .name(opts.name.clone())
            .loc(current.clone())
            .distance(opts.distance)
            .date(day)
            .separation(opts.separation)
            .wait(ctx.wait)
            .build().unwrap();

        work
    }).collect();
    trace!("All tasks: {:?}", worklist);

    // We use rayon to reduce the overhead during parallel calculations
    //
    use rayon::prelude::*;

    let stats: Vec<_> = worklist
        .par_iter()
        .map(|task| async {
            task.run(&dbh.clone()).await
        }).collect();

    let stats = join_all(stats).await;
    trace!("All stats: {:?}", stats);

    let stats: Vec<_> = stats.par_iter().filter_map(|res| {
        match res {
            Ok(res) => Some(res.clone()),
            Err(e) => {
                eprintln!("Task failed: {}", e);
                None
            }
        }
    }).collect();
    let stats = Stats::summarise(stats);
    trace!("summary={stats:?}");

    Ok(stats)
}
