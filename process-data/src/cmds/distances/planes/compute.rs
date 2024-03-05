//! This is where all the main calculations are done.
//!
//! FIXME: at the moment, the pipe uses fixed names for the intermediate tables (today,candidates, etc.).
//!
use std::ops::Add;

use chrono::{Datelike, Duration, TimeZone, Utc};
use duckdb::{Connection, params};
use tokio::time::Instant;
use tracing::{info, trace};

use crate::cmds::{ONE_DEG, PlaneDistance, PlanesStats, Stats};
use crate::cmds::batch::Calculate;

impl PlaneDistance {
    // -- private

    /// Select a list of airplanes positions we will consider for distance calculations
    ///
    /// - 1st criteria date and time (unit is a given day)
    /// - define a bounding box around a specific site (default is 70nm) and use it as a filter
    ///
    #[tracing::instrument(skip(dbh))]
    fn select_planes(&self, dbh: &Connection) -> eyre::Result<usize> {
        let site = self.name.clone();
        let day = self.date.day();
        let month = self.date.month();
        let year = self.date.year();
        let lat = self.loc.lat;
        let lon = self.loc.lon;

        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        println!("{} nm as deg: {}", self.distance, dist);

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
    fn select_drones(&self, dbh: &Connection) -> eyre::Result<usize> {
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
        let dist = self.distance * 1.852 / ONE_DEG;
        println!("{} nm as deg: {}", self.distance, dist);

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
    fn find_close(&self, dbh: &Connection) -> eyre::Result<usize> {
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
    fn calculate_distances(&self, dbh: &Connection) -> eyre::Result<usize> {
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
    fn save_encounters(&self, dbh: &Connection) -> eyre::Result<usize> {
        trace!("filter calculations, take min()");

        // We use a GROUP BY() clause to get the point where the distance between this drone and any surrounding planes
        // is minimal.  Gather more information about the encounter, `any_value()` is used to avoid "duplicates".
        // Then the result of this sub-query is inserted (or replaced if we re-ran the calculation) in the
        // `encounters` table.
        //
        // FIXME: conflicts are not handled, not sure why.

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
      any_value(timestamp) AS time,
      any_value(callsign) AS callsign,
      any_value(addr) AS addr,
      any_value(dist2d) AS distancelat,
      any_value(@(pz - dz)) AS distancevert,
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

        let count = dbh.execute(upd, [self.name.clone()])?;

        Ok(count)
    }
}

impl Calculate for PlaneDistance {
    /// Run the process for the given day.
    ///
    #[tracing::instrument(skip(dbh))]
    fn run(&self, dbh: &Connection) -> eyre::Result<Stats> {
        info!("Running calculations for {}:", self.date);

        let start = Instant::now();
        // Create our stat struct
        //
        let stats = &mut PlanesStats::new(self.date, self.distance, self.separation);

        // Create table `today` with all identified plane points with the specified range
        //
        let c_planes = self.select_planes(&dbh)?;

        if c_planes == 0 {
            stats.time = (Instant::now() - start).as_millis();
            eprintln!("No planes found.");
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.planes = c_planes;

        // Create table `candidates` with all designated drone points
        //
        let c_drones = self.select_drones(&dbh)?;

        if c_drones == 0 {
            stats.time = (Instant::now() - start).as_millis();
            eprintln!("No drones found.");
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.drones = c_drones;

        // Create table `today_close` with all designated drone points and airplanes in proximity
        //
        let c_potential = self.find_close(&dbh)?;

        if c_potential == 0 {
            stats.time = (Instant::now() - start).as_millis();
            eprintln!("No potential airprox found.");
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.potential = c_potential;

        // Now, we have the `today_close`  table with all points within 3 nm of each-others in all dimensions
        //
        let _ = self.calculate_distances(&dbh)?;

        // Now we have the distance calculated.
        //
        let c_encounters = self.save_encounters(&dbh)?;

        if c_encounters == 0 {
            stats.time = (Instant::now() - start).as_millis();
            eprintln!("No close encounters of any kind found.");
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.encounters = c_encounters;
        stats.time = (Instant::now() - start).as_millis();

        eprintln!("Stats for {}: {}", self.date, stats);
        info!("Done.");
        Ok(Stats::Planes(stats.clone()))
    }
}


