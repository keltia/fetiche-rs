//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::env;

use chrono::{Datelike, TimeZone, Utc};
use clap::Parser;
use eyre::Result;
use tracing::info;

pub use compute::*;
use fetiche_common::{DateOpts, expand_interval, load_locations, Location};

use crate::cmds::{Batch, Calculate, Stats, Status};
use crate::config::Context;

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

/// Handle the `distances planes` command.
///
#[tracing::instrument(skip(ctx))]
pub fn planes_calculation(ctx: &Context, opts: &PlanesOpts) -> Result<Stats> {
    let dbh = ctx.db();

    // Move ourselves to the datalake.
    //
    let datalake = ctx.config.get("datalake").unwrap();

    info!("Datalake: {}", datalake);
    env::set_current_dir(datalake)?;

    // Load locations
    //
    let list = load_locations(None)?;

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
    dbg!(begin, end);

    let dates = expand_interval(begin, end)?;
    dbg!(&dates);
    info!("{} days to process.", dates.len());

    // Load parameters
    //
    let name = opts.name.clone();
    let current: Location = if list.get(&name).is_none() {
        return Err(Status::ErrUnknownSite(name).into());
    } else {
        list.get(&name).unwrap().clone()
    };

    // Build our set of batches
    //
    let worklist: Vec<_> = dates.into_iter().map(|day| {
//        let work = PlaneDistance::new(&name, current.clone(), day).;

        let work = PlaneDistanceBuilder::default()
            .name(opts.name.clone())
            .loc(current.clone())
            .distance(opts.distance)
            .date(day)
            .separation(opts.separation)
            .template("".to_string())
            .build().unwrap();
        dbg!(&work);

        work
    }).collect();
    dbg!(&worklist);

    let mut batch = Batch::from_vec(&dbh, &worklist);

    // Gather stats for the run
    //
    let stats = batch.execute()?;

    let stats = Stats::summarise(stats);

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use duckdb::Connection;

    use super::*;

    #[test]
    fn test_calculations() -> Result<()> {
        // Store our context
        //
        let dbh = Connection::open_in_memory()?;
        let day = Utc::now();
        let current = Location { lon: 0., lat: 0., code: "".to_string(), hash: Some("".to_string()) };
        let name = String::from("test1");

        let work1 = PlaneDistance::new(&name, current.clone(), day);
        let work2 = PlaneDistance::new(&name, current.clone(), day + Days::new(1));
        dbg!(&work1);
        dbg!(&work2);
        let tasks = [work1, work2];
        let mut list = Batch::from_vec(&dbh, &tasks);

        dbg!(&list);

        let v: Vec<Stats> = list.execute()?;

        let stats = Stats::summarise(list.execute()?);

        Ok(())
    }
}
