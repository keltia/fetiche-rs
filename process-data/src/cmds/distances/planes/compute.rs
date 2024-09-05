//! This is where all the main calculations are done.
//!
//! FIXME: at the moment, the pipe uses fixed names for the intermediate tables (today, candidates, etc.).
//!
use std::ops::Add;

use chrono::{Datelike, Days, Duration, TimeZone, Utc};
use duckdb::{params, Connection};
use eyre::Result;
use tokio::time::Instant;
use tracing::{debug, info, trace};

use crate::cmds::batch::Calculate;
use crate::cmds::{PlaneDistance, PlanesStats, Stats, ONE_DEG};

impl PlaneDistance {
    // -- private

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
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let time_from = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap();
        let time_to = time_from.add(Duration::try_days(1).unwrap());

        println!("From {} to {}.", time_from, time_to);

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
CREATE OR REPLACE TABLE today AS
SELECT
  site,
  TimeRecPosition AS time,
  AircraftAddress AS addr,
  regexp_extract(Callsign, '([0-9A-Z]+)') AS callsign,
  Longitude AS plon,
  Latitude AS plat,
  CAST(GeometricAltitude AS DOUBLE) * 0.305 AS palt
FROM
  airplanes
WHERE
  site = ? AND
  time BETWEEN ? AND ? AND
  palt IS NOT NULL AND
  ST_DWithin(ST_point(?, ?), ST_Point(plat, plon), ?)
ORDER BY time
"##;
        let mut stmt = dbh.prepare(r1)?;
        let _ = stmt.query(params![site, time_from, time_to, lat, lon, dist])?;

        // Check how many
        //
        let count = dbh.query_row(
            "SELECT COUNT(*) FROM today",
            [],
            |row| Ok(row.get_unwrap(0)),
        )?;
        trace!("Total number of planes: {}\n", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    fn select_drones(&self, dbh: &Connection) -> Result<usize> {
        // All drone points for the same day
        //
        // $1 = date+1
        // $2 = date
        // $3,$4 = (lon,lat) site
        // $5 = distance in degrees
        //
        let lat = self.loc.lat;
        let lon = self.loc.lon;

        let day_start = self.date.timestamp();
        let day_end = self.date.checked_add_days(Days::new(1)).unwrap().timestamp();
        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let r2 = format!(r##"
CREATE OR REPLACE TABLE candidates AS
SELECT
    to_timestamp(timestamp) as time,
    journey,
    ident,
    model,
    to_timestamp(timestamp) as timestamp,
    latitude,
    longitude,
    altitude,
    elevation,
    home_lat,
    home_lon,
    home_distance_2d,
    home_distance_3d
FROM drones
WHERE
  to_timestamp(timestamp) BETWEEN to_timestamp({}) AND to_timestamp({}) AND
  ST_DWithin(ST_point({lat}, {lon}), ST_Point(latitude, longitude), {dist})
ORDER BY
  (time,journey)
    "##, day_start, day_end);
        debug!("{r2}");

        let mut stmt = dbh.prepare(&r2)?;
        let _ = stmt.query([])?;

        // Check how many
        //
        let count = dbh.query_row("SELECT COUNT(*) FROM candidates", [], |row| {
            let r: usize = row.get_unwrap(0);
            Ok(r)
        })?;
        trace!("Total number of drones: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    fn find_close(&self, dbh: &Connection) -> Result<usize> {
        trace!("Find close encounters.");

        let proximity = self.separation;

        // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
        // altitude diff is less than 3 nm. (parameter is `separation`).
        //
        // $1,$2 = lon,lat of site
        // $3 = timestamp of drone point
        //
        let r = format!(r##"
CREATE OR REPLACE TABLE today_close AS
SELECT
  c.journey,
  c.ident AS drone_id,
  c.model,
  c.timestamp AS time,
  c.longitude AS dlon,
  c.latitude AS dlat,
  c.altitude AS dalt,
  c.elevation AS dh,
  c.home_distance_2d AS hdist2d,
  c.home_distance_3d AS hdist3d,
  t.site,
  t.addr AS addr,
  t.callsign,
  t.time AS pt,
  t.plon AS plon,
  t.plat AS plat,
  t.palt AS palt,
  st_distance_spheroid(st_point(dlat,dlon), st_point(plat,plon)) AS dist2d,
  dist_3d(dlat, dlon, dalt, plat, plon, palt) AS dist_drone_plane,
  CEIL(@(palt - dalt)) AS diff_alt
FROM
  today AS t,
  candidates AS c
WHERE
  epoch(pt) BETWEEN (epoch(CAST(c.timestamp AS TIMESTAMP)) - 2) AND (epoch(CAST(c.timestamp AS TIMESTAMP)) + 2) AND
  dist2d <= {} AND
  diff_alt < {}
ORDER BY (c.timestamp, c.journey)
    "##, proximity, proximity);

        debug!("{r}");
        let mut stmt = dbh.prepare(&r)?;
        let _ = stmt.query([])?;

        // Check how many
        //
        let count = dbh.query_row("SELECT COUNT(*) FROM today_close", [], |row| {
            let r: usize = row.get_unwrap(0);
            Ok(r)
        })?;
        trace!("Total number of potential encounters: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    fn select_encounters(&self, dbh: &Connection) -> Result<usize> {
        trace!("select and record close points. ");

        // We use a GROUP BY() clause to get the point where the distance between this drone and any surrounding planes
        // is minimal.  Gather more information about the encounter, `any_value()` is used to avoid "duplicates".
        // Then the result of this sub-query is inserted (or replaced if we re-ran the calculation) in the
        // `encounters` table.
        //
        // FIXME: conflicts are not handled, not sure why.

        let day_name = self.date.format("%Y%m%d").to_string();

        // Insert data into table `encounters`
        //
        // - create sequence
        // - create table for ids
        // - select unique encounter for id generation
        // - insert ids
        // - join today_close and ids to get all points with the right en_id
        //

        let ins = format!(r##"
CREATE OR REPLACE SEQUENCE seq_ids;
CREATE OR REPLACE TABLE ids (
    id INT DEFAULT nextval('seq_ids'),
    site STRING,
    date STRING,
    drone_id STRING,
    callsign STRING,
    journey INT,
    en_id STRING,
);
INSERT INTO ids BY NAME (
    SELECT
      any_value(site) AS site,
      '{day_name}' AS date,
      journey,
      drone_id,
      callsign,
    FROM today_close
    WHERE
      dist_drone_plane < 1852
    GROUP BY ALL
);
UPDATE ids SET en_id = printf('%s-%s-%d-%d', site, date, journey, id);
INSERT INTO airplane_prox
BY NAME (
    SELECT
      ids.en_id,
      any_value(tc.site) AS site,
      any_value(time) AS time,
      tc.journey,
      tc.drone_id,
      any_value(model) AS model,
      any_value(dlon) AS drone_lon,
      any_value(dlat) AS drone_lat,
      any_value(dalt) AS drone_alt_m,
      any_value(ABS(dalt - dh)) AS drone_height_m,
      any_value(tc.callsign) AS prox_callsign,
      addr AS prox_id,
      any_value(plon) AS prox_lon,
      any_value(plat) AS prox_lat,
      any_value(palt) AS prox_alt_m,
      any_value(CEIL(dist2d)) AS distance_hor_m,
      any_value(CEIL(@(palt - dalt))) AS distance_vert_m,
      any_value(CEIL(hdist2d)) as distance_home_m,
      CEIL(dist_drone_plane) AS distance_slant_m,
    FROM today_close AS tc, ids
    WHERE
      dist_drone_plane < 1852
    AND
      ids.journey = tc.journey
    AND
      ids.callsign = tc.callsign
    GROUP BY ALL
);
DROP TABLE ids;
DROP SEQUENCE seq_ids;
        "##);

        let _ = dbh.execute_batch(&ins)?;

        // Now check how many
        //
        let count = dbh.query_row(&format!("SELECT COUNT(en_id) FROM airplane_prox WHERE en_id LIKE '%{}%'", day_name), [], |row| {
            let r: usize = row.get_unwrap(0);
            Ok(r)
        })?;
        if count == 0 {
            info!("No new encounters.");
            return Ok(count);
        } else {
            info!("Inserted {} new encounters", count);
        }
        Ok(count)
    }
}

const DELAY: u64 = 200;

impl Calculate for PlaneDistance {
    /// Run the process for the given day.
    ///
    #[tracing::instrument(skip(dbh))]
    fn run(&self, dbh: &Connection) -> Result<Stats> {
        info!("Running calculations for {}:", self.date);
        let bar = ml_progress::progress!(
            4;
            "[" percent "] " message_fill "(" eta_hms ")"
        )?;

        let start = Instant::now();
        // Create our stat struct
        //
        let stats = &mut PlanesStats::new(self.date, self.distance, self.separation);

        // Create table `today` with all identified plane points with the specified range
        //
        bar.message("Select planes.");
        let c_planes = self.select_planes(dbh)?;
        bar.inc(1);

        if c_planes == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No planes found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.planes = c_planes;
        bar.message(format!("{} planes.", c_planes));
        std::thread::sleep(std::time::Duration::from_millis(DELAY));

        // Create table `candidates` with all designated drone points
        //
        bar.message("Select drones.");
        let c_drones = self.select_drones(dbh)?;
        bar.inc(1);

        if c_drones == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No drones found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.drones = c_drones;
        bar.message(format!("{} drones.", c_drones));
        std::thread::sleep(std::time::Duration::from_millis(DELAY));

        // Create table `today_close` with all designated drone points and airplanes in proximity
        //
        bar.message("Find close planes.");
        let c_potential = self.find_close(dbh)?;
        bar.inc(1);

        if c_potential == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No potential airprox found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.potential = c_potential;
        bar.message(format!("{} potentials.", c_potential));
        std::thread::sleep(std::time::Duration::from_millis(DELAY));

        // Now we have the distance calculated.
        //
        bar.message("Find encounters.");
        let c_encounters = self.select_encounters(dbh)?;
        bar.inc(1);

        stats.time = (Instant::now() - start).as_millis();
        if c_encounters == 0 {
            bar.message("No close encounters of any kind found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.encounters = c_encounters;
        bar.message(format!("{} encounters.", c_encounters));
        std::thread::sleep(std::time::Duration::from_millis(DELAY));
        bar.finish();

        eprintln!("Stats for {}: {}", self.date, stats);
        info!("Done.");
        Ok(Stats::Planes(stats.clone()))
    }
}


