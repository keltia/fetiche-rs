//! Module computing the distance from a drone to the various planes around
//!

use chrono::{DateTime, Datelike, Utc};
use duckdb::Connection;
use eyre::{eyre, Result};

use crate::cli::PlanesOpts;
use crate::tasks::to_planes::location::{load_locations, Location};

mod location;

/// These are the options we pass to this command
///
#[derive(Debug, Parser)]
pub struct PlanesOpts {
    /// Do calculation on this date (day).
    pub date: DateTime<Utc>,
    /// Do calculations around this station.
    pub name: String,
    /// Distance in nm
    #[clap(default_value = "70.")]
    pub distance: f64,
}

/// One degree in *kilometers*
const ONE_DEG: f64 = 40_000. / 360.;

/// This is the struct in which we store the context of a given day work.
///
struct Context {
    pub name: Location,
    pub date: DateTime<Utc>,
}

impl Context {
    /// Select a list of airplanes positions we will consider for distance calculations
    ///
    /// - 1st criteria date and time (unit is a given day)
    /// - define a bounding box around a specific site (default is 70nm) and use it as a filter
    ///
    fn select_planes(&self, dbh: &Connection) -> Result<u32> {
        let day = self.date.day();
        let month = self.date.month();
        let year = self.date.year();

        // All flights for a given day in a table
        //
        // $1 = site
        // $2 = year
        // $3 = month
        // $4 = start of day
        // $5 = end of day
        // $6 = lon of site
        // $7 = lat of site
        // $8 = distance in degrees (== dist(nm) /  60)   1 deg ~ 60 nm
        //
        let r1 = r##"
CREATE TEMP TABLE today AS
SELECT
  TimeRecPosition AS time,
  AircraftAddress AS ident,
  Longitude AS px,
  Latitude AS py,
  CAST(GeometricAltitude AS DOUBLE) AS pz,
  Callsign AS callsign
FROM
  read_parquet('../adsb/**/*.parquet')
WHERE
  site=? AND
  year=? AND
  month=? AND
  time >= ? AND
  time <= ? AND
  ST_DWithin(ST_point(?, ?), ST_Point(px, py), ?)
"##;

        let mut stmt = dbh.prepare(r1)?;
        let list_planes = stmt.query_map([], |row| {})?;

        let rc = r##"
SELECT COUNT(*) FROM today
"##;
        let mut stmt = dbh.prepare(rc)?;
        let res = stmt.query_map([], |row| {
            let count = row.get_unwrap(0);
            count
        })?;
        Ok(res)
    }

    fn select_drones(&self, dbh: &Connection) -> Result<()> {
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

        let location = self.name.as_str();

        let r2 = r##"
CREATE TEMP TABLE candidates AS
SELECT *
FROM drones
WHERE
  to_timestamp(time) <= make_timestamp(?,?,? + 1,0,0,0.0) AND
  to_timestamp(time) >= make_timestamp(?,?,?,0,0,0.0) AND
  ST_DWithin(
    ST_point(?, ?), ST_Point(longitude, latitude), ?
  )
    "##;

        let mut stmt = dbh.prepare(r2)?;
        let list_drones = stmt.query_map([], |row| {})?;
        Ok(())
    }

    fn find_close(&self, dbh: &Connection) -> Result<()> {
        // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
        // altitude diff is less than 3 nm.
        //
        // $1,$2 = lon,lat of site
        // $3 = timestamp of drone point
        //
        let r = r##"
CREATE TEMP TABLE close AS
SELECT
  t.addr,
  t.callsign,
  t.time AS pt,
  t.px AS px,
  t.py AS py,
  t.pz as pz,
  c.ident,
  c.model,
  c.time AS dt,
  c.longitude AS dx,
  c.latitude AS dy,
  c.altitude AS dz,
  st_distance(st_point(dx,dy),st_point(px,py)) AS dist2d,
  @(pz - dz) AS diff_alt
FROM
  today AS t,
  candidates AS c
WHERE
  pt > to_timestamp(dt-2) AND
  pt < to_timestamp(dt+2) AND
  (@(pz - dz) < 5500.0)
    "##;
    }
}

pub fn planes_calculation(dbh: &Connection, opts: PlanesOpts) -> Result<()> {
    // Load locations
    //
    let list = load_locations(None)?;
    let day = opts.date;

    // Load parameters
    //
    let name = opts.location.clone();
    let current = if list.get(&opts.location).is_none() {
        return Err(eyre!("Unknown location: {}", opts.location));
    } else {
        list.get(&opts.location).unwrap().to_owned()
    };

    // Store our context
    //
    let ctx = Context {
        name: current.clone(),
        date: day,
    };

    // Create table `today` with all identified plane points with the specified range
    //
    let count = ctx.select_planes(&dbh)?;
    if count == 0 {
        return Err(eyre!("No planes found around {}.", &name));
    }

    // Create table `candidates` with all designated drone points
    //
    let count = ctx.select_drones(&dbh)?;

    Ok(())
}
