//! This is where all the main calculations are done.
//!
//! XXX CH does not have the SQL sequences so we need to generate the en_id field ourselves
//!
use crate::cmds::{Calculate, PlaneDistance, PlanesStats, Stats, ONE_DEG};
use chrono::{Datelike, Days, TimeZone, Utc};
use clickhouse::{Client, Row};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::ops::Add;
use tokio::time::{sleep, Duration, Instant};
use tracing::field::debug;
use tracing::{debug, error, info, trace};

#[derive(Debug, Default, Deserialize)]
struct Timings {
    select_planes: u128,
    select_drones: u128,
    find_close: u128,
    select_encounters: u128,
}

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
        let lat = self.lat;
        let lon = self.lon;

        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let time_from = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap();
        let time_to = time_from.add(chrono::Duration::try_days(1).unwrap());

        info!("From {} to {}.", time_from, time_to);

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
        trace!("Get site_id for {}", site);
        let id_site = dbh
            .query("SELECT id FROM sites WHERE name = ?")
            .bind(&site.name)
            .fetch_one::<u32>()
            .await?;

        trace!("site_id for {site} is {id_site}");

        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{site}_{day_name}");

        let r1 = format!(
            r##"
CREATE OR REPLACE TABLE today{tag}
ENGINE = MergeTree
PRIMARY KEY (site, time)
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
"##
        );

        // Given lat/lon and dist, we define the "ellipse" aka circle
        // cf. https://clickhouse.com/docs/en/sql-reference/functions/geo/coordinates#pointinellipses
        //
        debug!("ellipse=(center={},{},{},{})", lon, lat, dist, dist);

        let tm = Instant::now();
        dbh.query(&r1)
            .bind(id_site)
            .bind(time_from)
            .bind(time_to)
            .bind(lon)
            .bind(lat)
            .bind(dist)
            .bind(dist)
            .execute()
            .await?;
        let tm = (Instant::now() - tm).as_millis();
        trace!("CREATE TABLE today{tag} took {tm} ms");

        // Check how many
        //
        let r1 = format!("SELECT count() FROM today{tag}");
        let count = dbh.query(&r1).fetch_one::<usize>().await?;

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
        let lat = self.lat;
        let lon = self.lon;

        let site = &self.name;
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let start_day = self.date;
        let end_day = self.date.checked_add_days(Days::new(1)).unwrap();

        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{site}_{day_name}");

        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        trace!("Removing old table candidates{tag}.");
        let r1 = format!("DROP TABLE IF EXISTS candidates{tag}");
        dbh.query(&r1).execute().await?;

        let r2 = format!(
            r##"
CREATE TABLE candidates{tag}
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
  timestamp BETWEEN timestamp(?) AND timestamp(?)
  AND
  pointInEllipses(longitude,latitude, ?, ?, ?, ?)
ORDER BY
  (time,journey)
    "##
        );

        dbh.query(&r2)
            .bind(start_day)
            .bind(end_day)
            .bind(lon)
            .bind(lat)
            .bind(dist)
            .bind(dist)
            .execute()
            .await?;
        // Check how many
        //
        let count = dbh
            .query(&format!("SELECT COUNT() FROM candidates{tag}"))
            .fetch_one::<usize>()
            .await?;

        trace!("Total number of drones: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    async fn find_close(&self, dbh: &Client) -> Result<usize> {
        trace!("Find close encounters.");

        let site = &self.name;
        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{site}_{day_name}");

        trace!("Removing old table today_close{tag}.");

        // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
        // altitude diff is less than 3 nm. (parameter is `separation`).
        //
        // $1,$2 = lon,lat of site
        // $3 = timestamp of drone point
        //
        let r = format!(
            r##"
CREATE OR REPLACE TABLE today_close{tag}
ENGINE = Memory
AS (
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
  candidates{tag} AS c JOIN today{tag} AS t
ON
  toStartOfInterval(pt, toIntervalSecond(2)) = toStartOfInterval(c.timestamp, toIntervalSecond(2)) OR
  toStartOfInterval(pt, toIntervalSecond(2)) = toStartOfInterval(addSeconds(c.timestamp, 2), toIntervalSecond(2))
WHERE
  dist2d <= ? AND
  diff_alt < ?
)
    "##
        );

        debug!("q={r}");

        let proximity = self.separation;
        dbh.query(&r)
            .bind(proximity)
            .bind(proximity)
            .execute()
            .await?;

        // Check how many
        //
        let count = dbh
            .query(&format!("SELECT count() FROM today_close{tag}"))
            .fetch_one::<usize>()
            .await?;

        trace!("Total number of potential encounters: {}", count);
        Ok(count)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn create_table_ids(dbh: &Client, day_name: &str, site: &str) -> Result<()> {
        let tag = format!("_{site}_{day_name}");
        trace!("Drop table ids{tag}.");

        let r = format!("DROP TABLE IF EXISTS ids{tag}");
        dbh.query(&r).execute().await?;

        trace!("Create table ids{tag}.");
        let r = format!(
            r##"
        CREATE TABLE ids{tag} (
    drone_id VARCHAR,
    callsign VARCHAR,
    journey INT,
    en_id VARCHAR DEFAULT '',
) ENGINE = Memory
"##
        );

        Ok(dbh.query(&r).execute().await?)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn insert_ids(dbh: &Client, day_name: &str, site: &str) -> Result<usize> {
        let tag = format!("_{site}_{day_name}");

        let r = format!("SELECT count() FROM today_close{tag}");
        let total = dbh.query(&r).fetch_one::<usize>().await?;

        // This is for the query
        #[derive(Clone, Debug, Default, Serialize, Deserialize, Row)]
        struct Tc {
            journey: u32,
            drone_id: String,
            callsign: String,
        }

        // This is for the insert as the CH client does not support serde `skip_deserializing`.
        #[derive(Clone, Debug, Default, Serialize, Deserialize, Row)]
        struct Ids {
            en_id: String,
            journey: u32,
            drone_id: String,
            callsign: String,
        }

        let r = format!(
            r##"SELECT
      journey,
      drone_id,
      callsign,
    FROM today_close{tag}
    WHERE
      dist_drone_plane < 1852
    GROUP BY ALL
            "##
        );

        trace!("Fetch close encounters out of {total} from today_close.");
        let all = dbh.query(&r).fetch_all::<Tc>().await?;

        // No close encounters.
        //
        if all.is_empty() {
            return Ok(0);
        }

        trace!("Add en_id.");
        let all = all
            .iter()
            .enumerate()
            .map(|(id, elem): (usize, &Tc)| {
                let journey = elem.journey;
                let elem = Ids {
                    en_id: format!("{}-{}-{}-{}", site, day_name, journey, id),
                    journey: elem.journey,
                    drone_id: elem.drone_id.clone(),
                    callsign: elem.callsign.clone(),
                };
                debug!("{elem:?}");
                elem
            })
            .collect::<Vec<_>>();

        trace!("Insert updated records.");
        // Insert the records
        //
        let mut batch = dbh.insert(&format!("ids{tag}"))?;
        for item in all.iter() {
            batch.write(item).await?;
        }
        batch.end().await?;

        let count = dbh
            .query(&format!("SELECT count() FROM today_close{tag}"))
            .fetch_one::<usize>()
            .await?;
        trace!("Got {count} IDs");
        Ok(count)
    }

    #[inline]
    #[tracing::instrument(skip(dbh))]
    async fn cleanup_ids(dbh: &Client, day_name: &str, site: &str) -> Result<()> {
        let tag = format!("_{site}_{day_name}");

        Ok(dbh.query(&format!("DROP TABLE ids{tag}")).execute().await?)
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

        let site = &self.name;
        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{site}_{day_name}");

        // Insert data into table `encounters`
        //
        // - create sequence
        // - create table for ids
        // - select unique encounter for id generation
        // - insert ids
        // - join today_close and ids to get all points with the right en_id
        //

        Self::create_table_ids(dbh, &day_name, &site.name).await?;
        Self::insert_ids(dbh, &day_name, &site.name).await?;

        let r = format!(
            r##"INSERT INTO airplane_prox
     SELECT
      any_value(tc.site) AS site,
      id.en_id,
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
    FROM today_close{tag} AS tc JOIN ids{tag} AS id
      ON id.journey = tc.journey AND id.callsign = tc.callsign
    WHERE
      dist_drone_plane < 1852
    GROUP BY ALL
"##
        );
        trace!("Save encounters.");
        dbh.query(&r).execute().await?;

        Self::cleanup_ids(dbh, &day_name, &site.name).await?;

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

impl Calculate for PlaneDistance {
    /// Run the process for the given day.
    ///
    /// XXX Should we keep the interactive display or not.  When running parallel runs it messes
    /// up the output.
    ///
    #[tracing::instrument(skip(self, dbh))]
    async fn run(&self, dbh: &Client) -> Result<Stats> {
        info!("Running calculations for {}:", self.date);
        let bar = ml_progress::progress!(
            4;
            "[" percent "] " message_fill "(" eta_hms ")"
        )?;

        // Create our stat struct
        //
        let stats = &mut PlanesStats::new(self.date, self.distance, self.separation);
        let mut timings = Timings::default();

        // Create table `today` with all identified plane points with the specified range
        //
        bar.message("Select planes.");
        let start = Instant::now();
        let c_planes = self.select_planes(dbh).await?;
        timings.select_planes = (Instant::now() - start).as_millis();

        bar.inc(1);

        if c_planes == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No planes found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.planes = c_planes;
        bar.message(format!("{} planes.", c_planes));
        sleep(Duration::from_millis(self.wait)).await;

        // Create table `candidates` with all designated drone points
        //
        bar.message("Select drones.");
        let start = Instant::now();
        let c_drones = self.select_drones(dbh).await?;
        timings.select_drones = (Instant::now() - start).as_millis();
        bar.inc(1);

        if c_drones == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No drones found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.drones = c_drones as usize;
        bar.message(format!("{} drones.", c_drones));
        sleep(Duration::from_millis(self.wait)).await;

        // Create table `today_close` with all designated drone points and airplanes in proximity
        //
        bar.message("Find close planes.");
        let start = Instant::now();
        let c_potential = self.find_close(dbh).await?;
        timings.find_close = (Instant::now() - start).as_millis();
        bar.inc(1);

        if c_potential == 0 {
            stats.time = (Instant::now() - start).as_millis();
            bar.message("No potential airprox found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.potential = c_potential;
        bar.message(format!("{} potentials.", c_potential));
        sleep(Duration::from_millis(self.wait)).await;

        // Now we have the distance calculated.
        //
        bar.message("Find encounters.");
        let start = Instant::now();
        let c_encounters = self.select_encounters(dbh).await?;
        timings.select_encounters = (Instant::now() - start).as_millis();
        bar.inc(1);

        stats.time = (Instant::now() - start).as_millis();
        if c_encounters == 0 {
            bar.message("No close encounters of any kind found.");
            bar.finish();
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.encounters = c_encounters;
        bar.message(format!("{} encounters.", c_encounters));
        sleep(Duration::from_millis(self.wait)).await;

        info!("Stats for {}\n{}", self.date, stats);
        bar.message("Done.");
        bar.finish();

        dbg!(timings);

        Ok(Stats::Planes(stats.clone()))
    }
}
