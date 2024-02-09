//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::ops::Add;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use clap::Parser;
use duckdb::{params, Connection};
use eyre::Result;
use thiserror::Error;
use tracing::{info, trace};

use crate::location::{load_locations, Location};
use crate::tasks::ONE_DEG;

/// These are the options we pass to this command
///
#[derive(Debug, Parser)]
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

#[derive(Debug, Error)]
pub enum CalcStatus {
    #[error("No planes were found around site {0} at this date")]
    NoPlanesFound(String),
    #[error("No drones in the {0} area")]
    NoDronesFound(String),
    #[error("No encounters found in the {0} area")]
    NoEncounters(String),
    #[error("Invalid site name {0}")]
    ErrUnknownSite(String),
}

// -----

/// Every time we run a calculation for any given day, we store the statistics for the run.
///
#[derive(Debug)]
struct Stats {
    /// Specific date
    pub day: DateTime<Utc>,
    /// Number of plane points
    pub planes: usize,
    /// Number of drone points
    pub drones: usize,
    /// Number of potential encounters
    pub potential: usize,
    /// Effective number of encounters after calculations
    pub encounters: usize,
    /// Distance used for calculations
    pub distance: f64,
    /// Proximity used for calculations
    pub proximity: f64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            day: chrono::DateTime::UNIX_EPOCH,
            planes: 0,
            drones: 0,
            potential: 0,
            encounters: 0,
            distance: 0.,
            proximity: 0.,
        }
    }
}

// -----

/// This is the struct in which we store the context of a given day work.
///
#[derive(Debug)]
struct Context {
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

impl Context {
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
        match dbh.execute("SHOW TABLE today", []) {
            Ok(_) => {
                let _ = dbh.execute("DROP TABLE today", [])?;
            }
            Err(_) => (),
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
        match dbh.execute("SHOW TABLE candidates", []) {
            Ok(_) => {
                let _ = dbh.execute("DROP TABLE candidates", [])?;
            }
            Err(_) => (),
        }

        let r2 = r##"
CREATE TABLE candidates AS
SELECT time,journey,ident,model,timestamp,latitude,longitude,altitude,home_lat,home_lon,home_distance_2d,home_distance_3d
FROM drones
WHERE
  to_timestamp(time) <= make_timestamp(?,?,? + 1,0,0,0.0) AND
  to_timestamp(time) >= make_timestamp(?,?,?,0,0,0.0) AND
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
        match dbh.execute("SHOW TABLE today_close", []) {
            Ok(_) => {
                let _ = dbh.execute("DROP TABLE today_close", [])?;
            }
            Err(_) => (),
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
  pt > to_timestamp(dt-2) AND
  pt < to_timestamp(dt+2) AND
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
        match dbh.execute("SELECT dist_drone_plane FROM today_close", []) {
            Ok(_) => {
                let _ = dbh.execute("ALTER TABLE today_close DROP dist_drone_plane", [])?;
            }
            Err(_) => (),
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
INSERT INTO encounters
BY NAME (
    SELECT
      any_value(dt) AS dt,
      journey, 
      any_value(drone_id) AS drone_id, 
      model, 
      timestamp AS time,
      callsign, 
      addr,
      MIN(dist_drone_plane) AS distance,
    FROM today_close
    WHERE
      dist_drone_plane < 1852
    GROUP BY ALL
)
ON CONFLICT (dt, journey)
  DO NOTHING
        "##;

        let count = dbh.execute(ins, [])?;
        info!("Inserted {} new encounters", count);

        if count == 0 {
            info!("No new encounters.");
            return Ok(count);
        }

        info!("Generate en_id");
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

#[tracing::instrument(skip(dbh))]
pub fn planes_calculation(dbh: &Connection, opts: PlanesOpts) -> Result<usize> {
    // Load locations
    //
    let list = load_locations(None)?;
    let tm = dateparser::parse(&opts.date).unwrap();
    let day = Utc
        .with_ymd_and_hms(tm.year(), tm.month(), tm.day(), 0, 0, 0)
        .unwrap();
    info!("Running calculations for {}:", day);

    // Load parameters
    //
    let name = opts.name.clone();
    let current = if list.get(&name).is_none() {
        return Err(CalcStatus::ErrUnknownSite(name).into());
    } else {
        list.get(&name).unwrap().to_owned()
    };

    // Create our stat struct
    //
    let mut stats = Stats::default();
    stats.distance = opts.distance;
    stats.proximity = opts.separation;

    // Store our context
    //
    let ctx = Context {
        name: name.clone(),
        loc: current.clone(),
        dist: opts.distance,
        date: day,
        separation: opts.separation,
    };

    // Create table `today` with all identified plane points with the specified range
    //
    let count = ctx.select_planes(&dbh)?;
    stats.planes = count;

    if count == 0 {
        return Err(CalcStatus::NoPlanesFound(name).into());
    }

    // Create table `candidates` with all designated drone points
    //
    let count = ctx.select_drones(&dbh)?;
    if count == 0 {
        return Err(CalcStatus::NoDronesFound(name).into());
    }

    // Create table `today_close` with all designated drone points and airplanes in proximity
    //
    let count = ctx.find_close(&dbh)?;
    if count == 0 {
        return Err(CalcStatus::NoEncounters(name).into());
    }

    // Now, we have the `today_close`  table with all points within 3 nm of each-others in all dimensions
    //
    let _ = ctx.calculate_distances(&dbh)?;

    // Now we have the distance calculated.
    //
    let count = ctx.save_encounters(&dbh)?;
    if count == 0 {
        return Err(CalcStatus::NoEncounters(name).into());
    }

    info!("Done.");
    Ok(count)
}
