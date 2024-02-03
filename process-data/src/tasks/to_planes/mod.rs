//! Module computing the distance from a drone to the various planes around
//!
//! XXX be extra careful when dealing with degrees, meters and nautical miles.
//!

use std::ops::Add;

use crate::tasks::ONE_DEG;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use clap::Parser;
use duckdb::{params, Connection};
use eyre::{eyre, Result};
use rayon::prelude::*;
use tracing::{info, trace};

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

/// This is the struct in which we store the context of a given day work.
///
struct Context {
    /// Name of site
    pub name: String,
    /// Coordinates of site
    pub loc: Location,
    /// Max distance we want to consider
    pub dist: f64,
    /// Specific day
    pub date: DateTime<Utc>,
}

impl Context {
    /// Select a list of airplanes positions we will consider for distance calculations
    ///
    /// - 1st criteria date and time (unit is a given day)
    /// - define a bounding box around a specific site (default is 70nm) and use it as a filter
    ///
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
  AircraftAddress AS ident,
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
        let count = dbh.query_row("SELECT COUNT(*) FROM today", [], |row| {
            let r: usize = row.get_unwrap(0);
            Ok(r)
        })?;
        println!("Total number of planes: {}\n", count);
        Ok(count)
    }

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
SELECT time,journey,ident,model,latitude,longitude,altitude,home_lat,home_lon,home_distance_2d,home_distance_3d
FROM drones
WHERE
  to_timestamp(time) <= make_timestamp(?,?,? + 1,0,0,0.0) AND
  to_timestamp(time) >= make_timestamp(?,?,?,0,0,0.0) AND
  ST_DWithin(ST_point(?, ?), ST_Point(longitude, latitude), ?) = TRUE
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
        // altitude diff is less than 3 nm.
        //
        // $1,$2 = lon,lat of site
        // $3 = timestamp of drone point
        //
        let r = r##"
CREATE TABLE today_close AS
SELECT
  c.time AS dt,
  c.journey,
  c.ident,
  c.model,
  c.longitude AS dx,
  c.latitude AS dy,
  c.altitude AS dz,
  t.ident,
  t.callsign,
  t.time AS pt,
  t.px AS px,
  t.py AS py,
  t.pz as pz,
  st_distance(st_point(dx,dy),st_point(px,py)) * 111111 AS dist2d,
  @(pz - dz) AS diff_alt
FROM
  today AS t,
  candidates AS c
WHERE
  pt > to_timestamp(dt-2) AND
  pt < to_timestamp(dt+2) AND
  dist2d <= 5500.0 AND
  diff_alt < 5500.0
ORDER BY
  (dt, c.journey)
    "##;

        let mut stmt = dbh.prepare(r)?;
        let _ = stmt.query([])?;

        // Check how many
        //
        let count = dbh.query_row("SELECT COUNT(*) FROM today_close", [], |row| {
            let r: usize = row.get_unwrap(0);
            Ok(r)
        })?;
        println!("Total number of potential encounters: {}", count);
        Ok(count)
    }

    fn calculate_distances(&self, dbh: &Connection) -> Result<usize> {
        // Do calculations over all points in `today_close`.
        //
        trace!("add column dist_drone_plane");
        let _ = dbh.execute(
            r##"
ALTER TABLE today_close
ADD COLUMN dist_drone_plane FLOAT
"##,
            [],
        )?;

        let r = r##"
SELECT *
FROM today_close
ORDER BY (time, journey)
        "##;
        let mut stmt = dbh.prepare(r)?;
        let list = stmt.query_map([], |row| {
            let time: u32 = row.get_unwrap(0);
            let journey: u32 = row.get_unwrap(1);

            let dx: f64 = row.get_unwrap(4);
            let dy: f64 = row.get_unwrap(5);
            let dz: f64 = row.get_unwrap(6);

            let px: f64 = row.get_unwrap(10);
            let py: f64 = row.get_unwrap(11);
            let pz: f64 = row.get_unwrap(12);

            let drone = Point2D::new(dx, dy);
            let plane = Point2D::new(px, py);

            // 2D projected distance in METERS
            //
            // 2D dist is √(Δx𝟤 + Δy𝟤), cache Δx𝟤 + Δy𝟤 for later
            //
            let a2b2 = (plane.x - drone.x).powi(2) + (plane.y - drone.y).powi(2);

            // 3D distance
            //
            // Into degrees
            //
            let plane_z = pz / (1_000. * ONE_DEG);
            let drone_z = dz / (1_000. * ONE_DEG);

            // Calculate the 3D distance from plane to drone in METERS
            //
            // 3D dist is √(Δx𝟤 + Δy𝟤 + Δz𝟤)
            //
            let dist3d = (a2b2 + (plane_z - drone_z).powi(2)).sqrt();

            // Transform into meters
            //
            let dist3d = dist3d * (1_000. * ONE_DEG);

            Ok((time, journey, dist3d))
        })?;

        // Now update table with the distance
        //
        let sql_update = r##"
UPDATE
  today_close
SET
  dist_drone_plane = ?
WHERE
  time = ? AND journey = ?
        "##;

        let mut stmt = dbh.prepare(sql_update)?;
        let mut p = progress::SpinningCircle::new();
        p.set_job_title("updating plane-drone distances");
        list.for_each(|row| match row {
            Ok((time, journey, dist3d)) => {
                let _ = stmt.execute(params![dist3d, time, journey]).unwrap();
                p.tick();
            }
            Err(_) => (),
        });
        p.jobs_done();

        Ok(0)
    }
}

pub fn planes_calculation(dbh: &Connection, opts: PlanesOpts) -> Result<()> {
    // Load locations
    //
    let list = load_locations(None)?;
    let day = opts.date;

    // Load parameters
    //
    let current = if list.get(&opts.name).is_none() {
        return Err(eyre!("Unknown location: {}", opts.name));
    } else {
        list.get(&opts.name).unwrap().to_owned()
    };

    // Store our context
    //
    let ctx = Context {
        name: opts.name.clone(),
        loc: current.clone(),
        dist: opts.distance,
        date: day,
    };

    // Create table `today` with all identified plane points with the specified range
    //
    let count = ctx.select_planes(&dbh)?;
    if count == 0 {
        return Err(eyre!("No planes found around {}.", &opts.name));
    }

    // Create table `candidates` with all designated drone points
    //
    let count = ctx.select_drones(&dbh)?;
    if count == 0 {
        return Err(eyre!("No drones found around {}.", &opts.name));
    }

    // Create table `today_close` with all designated drone points and airplanes in proximity
    //
    let count = ctx.find_close(&dbh)?;
    if count == 0 {
        return Err(eyre!("Potential encounters {}.", count));
    }

    // Now, we have the `today_close`  table with all points within 3 nm of each-others in all dimensions
    //
    let _ = ctx.calculate_distances(&dbh)?;

    info!("Done.");
    Ok(())
}
