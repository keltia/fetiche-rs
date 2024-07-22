//! This is where all the main calculations are done.
//!
//! FIXME: at the moment, the pipe uses fixed names for the intermediate tables (today, candidates, etc.).
//!
//! XXX CH does not have the SQL sequences so we need to generate the en_id field ourselves
//!
use std::ops::Add;

use chrono::{Datelike, Days, TimeZone, Utc};
use clickhouse::{Client, Row};
use eyre::{eyre, Result};
use futures::executor::block_on;
use hcl::BlockBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{Duration, Instant, sleep};
use tracing::{debug, info, trace};
use tracing::field::debug;

use fetiche_common::BB;

use crate::cmds::{Calculate, ONE_DEG, PlaneDistance, PlanesStats, Stats};

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
        let id_site = dbh
            .query("SELECT id FROM sites WHERE name = ?".into())
            .bind(&site)
            .fetch_one::<u32>()
            .await?;
        debug!("site_id for {site} is {id_site}");

        trace!("Removing old table today.");
        let _ = dbh
            .query("DROP TABLE IF EXISTS acute.today".into())
            .execute()
            .await?;

        let r1 = r##"
CREATE TABLE acute.today
ENGINE = Memory
AS SELECT
  site,
  time,
  prox_id AS addr,
  prox_callsign AS callsign,
  prox_lon AS plon,
  prox_lat AS plat,
  CAST(prox_alt AS DOUBLE) * 0.305 AS palt
FROM
  airplanes
WHERE
  site = ? AND
  time BETWEEN timestamp(?) AND timestamp(?) AND
  palt IS NOT NULL AND
  pointInEllipses(plon, plat, ?, ?, ?, ?)
ORDER BY time
"##;

        // Given lat/lon and dist, we define the "ellipse" aka circle
        // cf. https://clickhouse.com/docs/en/sql-reference/functions/geo/coordinates#pointinellipses
        //
        debug!("ellipse=(center={},{},{},{})", lon, lat, dist, dist);

        let _ = dbh
            .query(r1)
            .bind(id_site)
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
        let count = dbh
            .query("SELECT count() FROM today")
            .fetch_one::<usize>()
            .await?;

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
        debug!("poly={poly:?}");

        let start_day = self.date;
        let end_day = self.date.checked_add_days(Days::new(1)).unwrap();

        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        trace!("Removing old table candidates.");
        let _ = dbh
            .query("DROP TABLE IF EXISTS acute.candidates".into())
            .execute()
            .await?;

        let r2 = r##"
CREATE TABLE acute.candidates
ENGINE = Memory AS
SELECT
    time,
    journey,
    ident,
    model,
    timestamp,
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
  CAST(toUnixTimestamp(timestamp) AS TIMESTAMP) BETWEEN timestamp(?) AND timestamp(?)
  AND
  pointInPolygon((longitude,latitude), [?, ?, ?, ?]) = 1
ORDER BY
  (time,journey)
    "##;

        let _ = dbh
            .query(r2)
            .bind(start_day)
            .bind(end_day)
            .bind(poly[0])
            .bind(poly[1])
            .bind(poly[2])
            .bind(poly[3])
            .execute()
            .await?;

        // Check how many
        //
        let count = dbh
            .query("SELECT COUNT() FROM candidates")
            .fetch_one::<usize>()
            .await?;

        trace!("Total number of drones: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    async fn find_close(&self, dbh: &Client) -> Result<usize> {
        trace!("Find close encounters.");

        trace!("Removing old table today_close.");
        let _ = dbh
            .query("DROP TABLE IF EXISTS acute.today_close".into())
            .execute()
            .await?;

        // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
        // altitude diff is less than 3 nm. (parameter is `separation`).
        //
        // $1,$2 = lon,lat of site
        // $3 = timestamp of drone point
        //
        let r = r##"
CREATE TABLE today_close
ENGINE = Memory AS
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
  ceil(palt - dalt) AS diff_alt
FROM
  today AS t,
  candidates AS c
WHERE
  pt BETWEEN CAST(toUnixTimestamp(c.time - 2) AS TIMESTAMP) AND CAST(toUnixTimestamp(c.time + 2) AS TIMESTAMP) AND
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
            .query("SELECT count() FROM today_close")
            .fetch_one::<usize>()
            .await?;

        trace!("Total number of potential encounters: {}", count);
        Ok(count)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn create_table_ids(dbh: &Client) -> Result<()> {
        trace!("Drop table ids.");
        let _ = dbh
            .query("DROP TABLE IF EXISTS ids".into())
            .execute()
            .await?;

        trace!("Create table ids.");
        let r = r##"
        CREATE TABLE ids (
    drone_id VARCHAR,
    callsign VARCHAR,
    journey INT,
    en_id VARCHAR,
) ENGINE = Memory
"##;

        Ok(dbh.query(&r).execute().await?)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn insert_ids(dbh: &Client, day_name: &str, site: &str) -> Result<usize> {
        #[derive(Clone, Debug, Serialize, Deserialize, Row)]
        struct Tc {
            #[serde(skip_deserializing)]
            en_id: String,
            journey: u32,
            drone_id: String,
            callsign: String,
        }

        let total = dbh.query("SELECT count() FROM today_close").fetch_one::<usize>().await?;

        let r = r##"
    SELECT
      journey,
      drone_id,
      callsign,
    FROM today_close
    WHERE
      dist_drone_plane < 1852
    GROUP BY ALL
        "##;

        trace!("Fetch close encounters out of {total} from today_close.");
        let all = dbh.query(r).fetch_all::<Tc>().await?;

        if all.len() == 0 {
            return Err(eyre!("No encounters found out of {total}").into());
        }

        trace!("Insert updated records.");
        // Insert the records
        //
        let mut batch = dbh.insert("acute.ids")?;

        trace!("Add en_id.");
        all.iter()
            .enumerate()
            .for_each(|(id, elem): (usize, &Tc)| {
                let journey = elem.journey;
                let elem = Tc {
                    en_id: format!("{}-{}-{}-{}", site, day_name, journey, id),
                    journey: elem.journey,
                    drone_id: elem.drone_id.clone(),
                    callsign: elem.callsign.clone(),
                };
                debug!("{elem:?}");
                block_on(async { batch.write(&elem).await.unwrap(); });
            });
        let _ = batch.end().await?;

        let count = dbh.query("SELECT count() FROM today_close").fetch_one::<usize>().await?;
        trace!("Got {count} IDs");
        Ok(count as usize)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn cleanup_seq(dbh: &Client) -> Result<()> {
        let _ = dbh.query(r##"DROP TABLE ids"##).execute().await?;
        Ok(())
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
        let site = self.name.clone();

        Self::create_table_ids(dbh).await?;
        Self::insert_ids(dbh, &day_name, &site).await?;

        let r = r##"INSERT INTO airplane_prox
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
      any_value(CEIL(ABS(palt - dalt))) AS distance_vert_m,
      any_value(CEIL(hdist2d)) as distance_home_m,
      CEIL(dist_drone_plane) AS distance_slant_m
    FROM today_close AS tc, ids
    WHERE
      dist_drone_plane < 1852
    AND
      ids.journey = tc.journey
    AND
      ids.callsign = tc.callsign
    GROUP BY ALL
"##;
        trace!("Save encounters.");
        dbh.query(r).execute().await?;

        //Self::cleanup_seq(dbh).await?;

        // Now check how many
        //
        let pattern = format!("%{day_name}%");
        let count = dbh
            .query("SELECT COUNT(en_id) FROM airplane_prox WHERE en_id LIKE ?")
            .bind(pattern)
            .fetch_one::<usize>()
            .await?;

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
        stats.drones = c_drones as usize;
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
