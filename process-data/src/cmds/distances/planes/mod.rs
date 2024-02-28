//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

mod compute;

pub use compute::*;

use std::env;
use std::ops::Add;

use chrono::{Datelike, DateTime, Days, Duration, TimeZone, Utc};
use clap::Parser;
use duckdb::{Connection, params};
use eyre::Result;
use rayon::prelude::*;
use tracing::{error, info, trace};

use fetiche_common::{DateOpts, expand_interval, load_locations, Location};

use crate::cmds::{Batch, Calculate, HomeStats, ONE_DEG, PlanesStats, Stats, Status};
use crate::config::Context;

/// These are the options we pass to this command
///
#[derive(Clone, Debug, Parser)]
pub struct PlanesOpts {
    #[clap(subcommand)]
    pub date: DateOpts,
    /// Do calculation on this date (day).
    //pub date: String,
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

    // Load locations
    //
    let list = load_locations(None)?;

    let (begin, end) = match opts.date.parse() {
        Ok((start, stop)) => {
            info!("We have an interval: from {} to {}", start, stop);
            (start, stop)
        }
        Err(_) => {
            let tm = dateparser::parse(&opts.date).unwrap();
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

    // Move ourselves to the datalake.
    //
    let datalake = ctx.config.get("datalake").unwrap();
    info!("Datalake: {}", datalake);

    env::set_current_dir(datalake)?;

    // Load parameters
    //
    let name = opts.name.clone();
    let current: Location = if list.get(&name).is_none() {
        return Err(Status::ErrUnknownSite(name).into());
    } else {
        list.get(&name).unwrap().clone()
    };

    let work1 = PlaneDistance::new(&name, current.clone(), begin);
    dbg!(&work1);
    let mut batches = Batch::new(&dbh);

    batches.add(Box::new(work1));

    dbg!(&batches);

    let v: Vec<Stats> = batches.execute()?;

    Ok(stats);
}

#[cfg(test)]
mod tests {
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
        let mut list = Batch::new(&dbh);

        list.add(Box::new(work1)).add(Box::new(work2));

        dbg!(&list);

        let v: Vec<Stats> = list.execute()?;

        //let stats = work.calculate(&dbh)?;

        Ok(())
    }
}
