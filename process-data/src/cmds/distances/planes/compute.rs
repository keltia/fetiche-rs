//! This is where all the main calculations are done.
//!
//! FIXME: at the moment, the pipe uses fixed names for the intermediate tables (today, candidates, etc.).
//!
use std::ops::Add;

use chrono::{Datelike, Days, TimeZone, Utc};
use clickhouse::Client;
use eyre::Result;
use tokio::time::{Duration, Instant, sleep};
use tracing::{debug, info, trace};
use fetiche_common::BB;

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
    async fn select_planes(&self, dbh: &Client) -> Result<usize> {
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
        let time_to = time_from.add(chrono::Duration::try_days(1).unwrap());

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
        let _ = dbh.query("DROP TABLE today").execute().await?;

        let r1 = r##"
CREATE TABLE today
ORDER BY time
AS SELECT
  (SELECT id FROM sites WHERE name = ?) AS id_site,
  time,
  prox_id AS addr,
  replaceRegexpOne(prox_callsign, '\'([0-9A-Z]+)\\s*\'', '\\1') AS callsign,
  prox_lon AS plon,
  prox_lat AS plat,
  CAST(prox_alt AS DOUBLE) * 0.305 AS palt
FROM
  airplanes AS a, sites AS s
WHERE
  id_site = site AND
  time BETWEEN timestamp(?) AND timestamp(?) AND
  palt IS NOT NULL AND
  pointInEllipses(plon, plat, ?, ?, ?, ?)
ORDER BY time
"##;

        // Given lat/lon and dist, we define the "ellipse" aka circle
        // cf. https://clickhouse.com/docs/en/sql-reference/functions/geo/coordinates#pointinellipses
        //
        debug!("ellipse=(center={},{},{},{})", lon, lat, dist, dist);

        let _ = dbh.query(r1)
            .bind(site)
            .bind(time_from)
            .bind(time_to)
            .bind(lon)
            .bind(lat)
            .bind(dist)
            .bind(dist)
            .execute()
            .await?;

        // Check how many
        //
        let count = dbh.query("SELECT count() FROM today").fetch_one::<usize>().await?;

        trace!("Total number of planes: {}\n", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    async fn select_drones(&self, dbh: &Client) -> Result<usize> {
        // All drone points for the same day
        //
        // $1 = date+1
        // $2 = date
        // $3,$4 = (lon,lat) site
        // $5 = distance in degrees
        //
        let lat = self.loc.lat;
        let lon = self.loc.lon;
        let dist = self.distance.ceil() as u32;

        // Generate the polygon we are checking whether the point is in or not
        //
        let poly = BB::from_lat_lon(lat, lon, dist).to_polygon()?;

        let start_day = self.date;
        let end_day = self.date.checked_add_days(Days::new(1)).unwrap();

        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let r2 = r##"
CREATE
OR REPLACE TABLE candidates Engine = 'Memory' AS
SELECT
    time,
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
  CAST(to_timestamp(timestamp) AS TIMESTAMP) BETWEEN ? AND ?
  AND
  pointInPolygon((longitude,latitude), ?) = 1
ORDER BY
  (time,journey)
    "##;

        let _ = dbh
            .query(r2)
            .bind(start_day)
            .bind(end_day)
            .bind(poly)
            .execute()
            .await?;

        // Check how many
        //
        let mut res = dbh
            .query("SELECT COUNT() FROM candidates")
            .fetch::<usize>()?;

        let count: usize = res.next().await?.unwrap_or(0);
        trace!("Total number of drones: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    async fn find_close(&self, dbh: &Client) -> Result<usize> {
        trace!("Find close encounters.");

        // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
        // altitude diff is less than 3 nm. (parameter is `separation`).
        //
        // $1,$2 = lon,lat of site
        // $3 = timestamp of drone point
        //
        let r = r##"
CREATE OR REPLACE TABLE today_close Engine = 'Memory' AS
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
  dist_2d(dlon, dlat, plon, plat) AS dist2d,
  dist_3d(dlon, dlat, dalt, plon, plat, palt) AS dist_drone_plane,
  ceil(@(palt - dalt)) AS diff_alt
FROM
  today AS t,
  candidates AS c
WHERE
  pt BETWEEN CAST(to_timestamp(c.time - 2) AS TIMESTAMP) AND CAST(to_timestamp(c.time + 2) AS TIMESTAMP) AND
  dist2d <= ? AND
  diff_alt < ?
ORDER BY
  (c.time, c.journey)
    "##;

        let proximity = self.separation;
        let _ = dbh
            .query(r)
            .bind(proximity)
            .bind(proximity)
            .execute()
            .await?;

        // Check how many
        //
        let count = dbh
            .query("SELECT COUNT() FROM today_close")
            .fetch_one::<usize>().await?;

        trace!("Total number of potential encounters: {}", count);
        Ok(count)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn create_seq_ids(dbh: &Client) -> Result<()> {
        Ok(dbh.query(r##"CREATE OR REPLACE SEQUENCE seq_ids"##).execute().await?)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn create_table_ids(dbh: &Client) -> Result<()> {
        Ok(dbh.query(
            r##"CREATE OR REPLACE TABLE ids (
    id INT DEFAULT nextval('seq_ids'),
    site STRING,
    date STRING,
    drone_id STRING,
    callsign STRING,
    journey INT,
    en_id STRING,
)"##)
            .execute()
            .await?)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn insert_ids(dbh: &Client, day_name: &str) -> Result<()> {
        Ok(dbh.query(
            r##"INSERT INTO ids BY NAME (
    SELECT
      any_value(site) AS site,
      ? AS date,
      journey,
      drone_id,
      callsign,
    FROM today_close
    WHERE
      dist_drone_plane < 1852
    GROUP BY ALL
)"##)
            .bind(day_name)
            .execute()
            .await?)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn update_seq_en_id(dbh: &Client) -> Result<()> {
        Ok(dbh.query(r##"UPDATE ids SET en_id = printf('%s-%s-%d-%d', site, date, journey, id) WHERE en_id = ''"##).execute().await?)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn cleanup_seq(dbh: &Client) -> Result<()> {
        let _ = dbh.query(r##"DROP TABLE ids"##).execute().await?;
        Ok(dbh.query(r##"DROP SEQUENCE seq_ids"##).execute().await?)
    }

    #[tracing::instrument(skip(dbh))]
    async fn select_encounters(&self, dbh: &Client) -> Result<usize> {
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
        Self::create_seq_ids(dbh).await?;
        Self::create_table_ids(dbh).await?;
        Self::insert_ids(dbh, &day_name).await?;
        Self::update_seq_en_id(dbh).await?;

        let r= r##"INSERT INTO airplane_prox
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
      any_value(dh) AS drone_height_m,
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
      di st_drone_plane < 1852
    AND
      ids.journey = tc.journey
    AND
      ids.callsign = tc.callsign
    GROUP BY ALL
)"##;
        dbh.query(r).execute().await?;

        Self::cleanup_seq(dbh).await?;

        // Now check how many
        //
        let pattern = format!("%{day_name}%");
        let count = dbh.query("SELECT COUNT(en_id) FROM airplane_prox WHERE en_id LIKE ?")
            .bind(pattern)
            .fetch_one::<usize>().await?;

        if count == 0 {
            info!("No new encounters.");
            return Ok(count);
        } else {
            info!("Inserted {} new encounters", count);
        }
        Ok(count)
    }
}

/// Interactive mode, pause to show results before running next phase. In millisec.
///
const DELAY: u64 = 200;

impl Calculate for PlaneDistance {
    /// Run the process for the given day.
    ///
    #[tracing::instrument(skip(dbh))]
    async fn run(&self, dbh: &Client) -> Result<Stats> {

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
        let c_planes = self.select_planes(dbh).await?;
        bar.inc(1);

        if c_planes == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No planes found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.planes = c_planes;
        bar.message(format!("{} planes.", c_planes));
        sleep(Duration::from_millis(DELAY)).await;

        // Create table `candidates` with all designated drone points
        //
        bar.message("Select drones.");
        let c_drones = self.select_drones(dbh).await?;
        bar.inc(1);

        if c_drones == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No drones found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.drones = c_drones;
        bar.message(format!("{} drones.", c_drones));
        sleep(Duration::from_millis(DELAY)).await;

        // Create table `today_close` with all designated drone points and airplanes in proximity
        //
        bar.message("Find close planes.");
        let c_potential = self.find_close(dbh).await?;
        bar.inc(1);

        if c_potential == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No potential airprox found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.potential = c_potential;
        bar.message(format!("{} potentials.", c_potential));
        sleep(Duration::from_millis(DELAY)).await;

        // Now we have the distance calculated.
        //
        bar.message("Find encounters.");
        let c_encounters = self.select_encounters(dbh).await?;
        bar.inc(1);

        stats.time = (Instant::now() - start).as_millis();
        if c_encounters == 0 {
            bar.message("No close encounters of any kind found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.encounters = c_encounters;
        bar.message(format!("{} encounters.", c_encounters));
        sleep(Duration::from_millis(DELAY)).await;
        bar.finish();

        eprintln!("Stats for {}: {}", self.date, stats);
        info!("Done.");
        Ok(Stats::Planes(stats.clone()))
    }
}


