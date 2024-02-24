//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::env;
use std::ops::Add;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use clap::Parser;
use duckdb::{params, Connection};
use eyre::Result;
use tracing::{info, trace};

use fetiche_common::{load_locations, Location};

use crate::cmds::{Stats, Status, ONE_DEG};
use crate::config::Context;

/// These are the options we pass to this command
///
#[derive(Clone, Debug, Parser)]
pub struct PlanesOpts {
    /// Do calculation on this date (day).
    pub date: String,
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
#[derive(Debug)]
struct Work {
    /// Name of site
    pub name: String,
    /// Coordinates of site
    pub loc: Location,
    /// Max distance we want to consider
    pub dist: f64,
    /// Specific day
    pub date: DateTime<Utc>,
    /// proximity
    pub separation: f64,
}

impl Work {
    /// Select a list of airplanes positions we will consider for distance calculations
    ///
    /// - 1st criteria date and time (unit is a given day)
    /// - define a bounding box around a specific site (default is 70nm) and use it as a filter
    ///
    #[tracing::instrument(skip(dbh))]
    fn select_planes(&self, dbh: &Connection) -> Result<usize> {
        let site = self.name.clone();
        let day = self.date.day();
        let month = self.date.month();
        let year = self.date.year();
        let lat = self.loc.lat;
        let lon = self.loc.lon;

        // Our distance in nm converted into degrees
        //
        let dist = self.dist * 1.852 / ONE_DEG;
        println!("{} nm as deg: {}", self.dist, dist);

        let time_from = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap();
        let time_to = time_from.add(Duration::days(1));

        println!("From {} to {}.", time_from, time_to);

        // Cleanup if needed
        //
        if dbh.execute("SHOW TABLE today", []).is_ok() {
            let _ = dbh.execute("DROP TABLE today", [])?;
        }

        // All flights for a given day in a table
        //
        // $1 = site
        // $2 = year
        // $3 = month
        // $4 = start of day
        // $5 = end of day
        // $6 = lon of site
        // $7 = lat of site
        // $8 = distance in degrees (== dist(nm) /  60)   1 deg ~ 60 nm ~111.1 km
        //
        //
        let r1 = r##"
CREATE TABLE today AS
SELECT
  TimeRecPosition AS time,
  AircraftAddress AS addr,
  Callsign AS callsign,
  Longitude AS px,
  Latitude AS py,
  CAST(GeometricAltitude AS DOUBLE) * 0.305 AS pz
FROM
  airplanes
WHERE
  site=? AND
  time >= ? AND
  time <= ? AND
  pz IS NOT NULL AND
  ST_DWithin(ST_point(?, ?), ST_Point(px, py), ?)
ORDER BY time
"##;
        let mut stmt = dbh.prepare(r1)?;
        let _ = stmt.query(params![site, time_from, time_to, lon, lat, dist])?;

        // Check how many
        //
        let count = dbh.query_row(
            "SELECT COUNT(*) FROM today",
            [],
            |row| Ok(row.get_unwrap(0)),
        )?;
        println!("Total number of planes: {}\n", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    fn select_drones(&self, dbh: &Connection) -> Result<usize> {
        // All drone points for the same day
        //
        // $1 = year
        // $2 = month
        // $3 = day
        // $4,$5 = (lon,lat) site
        // $6 = distance in degrees
        //
        let year = self.date.year();
        let month = self.date.month();
        let day = self.date.day();

        let lat = self.loc.lat;
        let lon = self.loc.lon;

        // Our distance in nm converted into degrees
        //
        let dist = self.dist * 1.852 / ONE_DEG;
        println!("{} nm as deg: {}", self.dist, dist);

        // Cleanup if needed
        //
        if dbh.execute("SHOW TABLE candidates", []).is_ok() {
            let _ = dbh.execute("DROP TABLE candidates", [])?;
        }

        let r2 = r##"
CREATE TABLE candidates AS
SELECT time,journey,ident,model,timestamp,latitude,longitude,altitude,home_lat,home_lon,home_distance_2d,home_distance_3d
FROM drones
WHERE
  CAST(to_timestamp(time) AS TIMESTAMP) <= make_timestamp(?,?,? + 1,0,0,0.0) AND
  CAST(to_timestamp(time) AS TIMESTAMP) >= make_timestamp(?,?,?,0,0,0.0) AND
  ST_DWithin(ST_point(?, ?), ST_Point(longitude, latitude), ?)
ORDER BY
  (time,journey)
    "##;

        let mut stmt = dbh.prepare(r2)?;
        let _ = stmt.query(params![year, month, day, year, month, day, lon, lat, dist])?;

        // Check how many
        //
        let count = dbh.query_row("SELECT COUNT(*) FROM candidates", [], |row| {
            let r: usize = row.get_unwrap(0);
            Ok(r)
        })?;
        println!("Total number of drones: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    fn find_close(&self, dbh: &Connection) -> Result<usize> {
        // Cleanup if needed
        //
        if dbh.execute("SHOW TABLE today_close", []).is_ok() {
            let _ = dbh.execute("DROP TABLE today_close", [])?;
        }

        // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
        // altitude diff is less than 3 nm. (parameter is `separation`).
        //
        // $1,$2 = lon,lat of site
        // $3 = timestamp of drone point
        //
        let r = r##"
CREATE TABLE today_close AS
SELECT
  c.time AS dt,
  c.journey,
  c.ident AS drone_id,
  c.model,
  c.timestamp as timestamp,
  c.longitude AS dx,
  c.latitude AS dy,
  c.altitude AS dz,
  t.addr AS addr,
  t.callsign,
  t.time AS pt,
  t.px AS px,
  t.py AS py,
  t.pz AS pz,
  dist_2d(dx,dy, px,py) AS dist2d,
  @(pz - dz) AS diff_alt
FROM
  today AS t,
  candidates AS c
WHERE
  pt > CAST(to_timestamp(dt-2) AS TIMESTAMP) AND
  pt < CAST(to_timestamp(dt+2) AS TIMESTAMP) AND
  dist2d <= ? AND
  diff_alt < ?
ORDER BY
  (dt, c.journey)
    "##;

        let proximity = self.separation;
        let mut stmt = dbh.prepare(r)?;
        let _ = stmt.query(params![proximity, proximity])?;

        // Check how many
        //
        let count = dbh.query_row("SELECT COUNT(*) FROM today_close", [], |row| {
            let r: usize = row.get_unwrap(0);
            Ok(r)
        })?;
        println!("Total number of potential encounters: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    fn calculate_distances(&self, dbh: &Connection) -> Result<usize> {
        // drop column if present
        //
        if dbh
            .execute("SELECT dist_drone_plane FROM today_close LIMIT 1", [])
            .is_ok()
        {
            let _ = dbh.execute("ALTER TABLE today_close DROP dist_drone_plane", [])?;
        }

        // Do calculations over all points in `today_close`.
        //
        trace!("add column dist_drone_plane");
        let _ = dbh.execute(
            r##"
ALTER TABLE today_close
ADD COLUMN dist_drone_plane FLOAT;
"##,
            [],
        )?;

        trace!("Do the math.");
        let count = dbh.execute(
            r##" 
UPDATE today_close
SET dist_drone_plane = dist_3d(px, py, pz, dx, dy, dz)
"##,
            [],
        )?;
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    fn save_encounters(&self, dbh: &Connection) -> Result<usize> {
        trace!("filter calculations, take min()");

        // We use a GROUP BY() clause to get the point where the distance between this drone and any surrounding planes
        // is minimal.  Gather more information about the encounter, `any_value()` is used to avoid "duplicates".
        // Then the result of this sub-query is inserted (or replaced if we re-ran the calculation) in the
        // `encounters` table.

        // Insert data into table `encounters`
        //
        let ins = r##"
INSERT OR IGNORE INTO encounters
BY NAME (
    SELECT
      any_value(dt) AS dt,
      journey, 
      any_value(drone_id) AS drone_id, 
      model, 
      timestamp AS time,
      any_value(callsign) as callsign,
      any_value(addr) as addr,
      MIN(dist_drone_plane) AS distance,
    FROM today_close
    WHERE
      dist_drone_plane < 1852
    GROUP BY ALL
)
        "##;

        let count = dbh.execute(ins, [])?;
        if count == 0 {
            info!("No new encounters.");
            return Ok(count);
        } else {
            info!("Inserted {} new encounters", count);
        }

        trace!("Generate en_id");
        let name = self.name.clone();
        let upd = r##"
UPDATE encounters AS old_e
SET en_id = (
    SELECT 
      encounter(CAST(old_e.time AS DATE), journey, id) AS en_id
    FROM
      encounters AS new_e
    WHERE
      old_e.dt = new_e.dt AND old_e.journey = new_e.journey
),
site = ?
WHERE en_id IS NULL OR site IS NULL
        "##;

        let count = dbh.execute(upd, [name])?;

        Ok(count)
    }
}

#[tracing::instrument(skip(ctx))]
pub fn planes_calculation(ctx: &Context, opts: &PlanesOpts) -> Result<Stats> {
    let dbh = ctx.db();

    // Load locations
    //
    let list = load_locations(None)?;
    let tm = dateparser::parse(&opts.date).unwrap();
    let day = Utc
        .with_ymd_and_hms(tm.year(), tm.month(), tm.day(), 0, 0, 0)
        .unwrap();
    info!("Running calculations for {}:", day);

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

    // Store our context
    //
    let work = Work {
        name: name.clone(),
        loc: current,
        dist: opts.distance,
        date: day,
        separation: opts.separation,
    };

    // Create table `today` with all identified plane points with the specified range
    //
    let c_planes = work.select_planes(&dbh)?;

    if c_planes == 0 {
        return Err(Status::NoPlanesFound(name).into());
    }

    // Create table `candidates` with all designated drone points
    //
    let c_drones = work.select_drones(&dbh)?;

    if c_drones == 0 {
        return Err(Status::NoDronesFound(name).into());
    }

    // Create table `today_close` with all designated drone points and airplanes in proximity
    //
    let c_potential = work.find_close(&dbh)?;

    if c_potential == 0 {
        return Err(Status::NoEncounters(name).into());
    }

    // Now, we have the `today_close`  table with all points within 3 nm of each-others in all dimensions
    //
    let _ = work.calculate_distances(&dbh)?;

    // Now we have the distance calculated.
    //
    let c_encounters = work.save_encounters(&dbh)?;

    if c_encounters == 0 {
        return Err(Status::NoEncounters(name).into());
    }

    // Create our stat struct
    //
    let stats = Stats::Planes {
        day: tm,
        distance: opts.distance,
        proximity: opts.separation,
        planes: c_planes,
        drones: c_drones,
        potential: c_potential,
        encounters: c_encounters,
    };

    info!("Done.");
    Ok(stats)
}
