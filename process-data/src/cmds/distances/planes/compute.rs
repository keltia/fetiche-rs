//! This is where all the main calculations are done.
//!
//! XXX CH does not have the SQL sequences so we need to generate the en_id field ourselves
//!
use crate::cmds::{Calculate, PlaneDistance, PlanesStats, Stats, TempTables, ONE_DEG};
use eyre::Result;
use futures::future::try_join_all;
use klickhouse::{Client, QueryBuilder, RawRow, Row};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, info, trace};

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
    async fn select_planes(&mut self, dbh: &Client) -> Result<usize> {
        let site = self.site.clone();
        let name = site.name.clone();
        let lat = self.lat;
        let lon = self.lon;

        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let time_from = self.date.format("%Y-%m-%d 00:00:00").to_string();
        let time_to = self
            .date
            .add(chrono::Duration::try_days(1).unwrap())
            .format("%Y-%m-%d 00:00:00")
            .to_string();
        info!(
            "From {} to {} on {}/{}.",
            time_from, time_to, site.name, site.id
        );

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
        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{name}_{day_name}");

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
  site = $1 AND
  toStartOfInterval(time, toIntervalDay(1)) = toDateTime($2) AND
  palt IS NOT NULL AND
  pointInEllipses(plon, plat, $3, $4, $5, 6)
ORDER BY time
"##
        );

        // Given lat/lon and dist, we define the "ellipse" aka circle
        // cf. https://clickhouse.com/docs/en/sql-reference/functions/geo/coordinates#pointinellipses
        //
        debug!("ellipse=(center={},{},{},{})", lon, lat, dist, dist);

        let tm = Instant::now();
        let q = QueryBuilder::new(&r1)
            .arg(site.id)
            .arg(time_from)
            .arg(lon)
            .arg(lat)
            .arg(dist)
            .arg(dist);
        let _ = dbh.execute(q).await?;
        let tm = (Instant::now() - tm).as_millis();
        trace!("CREATE TABLE today{tag} took {tm} ms");

        // Check how many
        //
        let r1 = format!("SELECT count() FROM today{tag} AS count");
        let q = QueryBuilder::new(&r1);
        let mut count = dbh.query_one::<RawRow>(q).await?;

        self.state.push(TempTables::Today);
        let count: u32 = count.get("count");
        trace!("Total number of planes: {}\n", count);
        Ok(count as usize)
    }

    #[tracing::instrument(skip(dbh))]
    async fn select_drones(&mut self, dbh: &Client) -> Result<u32> {
        // All drone points for the same day
        //
        // $1 = date+1
        // $2 = date
        // $3,$4 = (lon,lat) site
        // $5 = distance in degrees
        //
        let lat = self.lat;
        let lon = self.lon;

        let site = self.site.clone();
        let name = site.name.clone();
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let time_from = self.date.format("%Y-%m-%d 00:00:00").to_string();

        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{name}_{day_name}");

        // Our distance in nm converted into degrees
        //
        let dist = self.distance * 1.852 / ONE_DEG;
        debug!("{} nm as deg: {}", self.distance, dist);

        let r2 = format!(
            r##"
CREATE OR REPLACE TABLE candidates{tag}
ENGINE = MergeTree
ORDER BY (time,journey)
AS SELECT
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
  toStartOfInterval(timestamp, toIntervalDay(1)) = toDateTime($1) AND
  pointInEllipses(longitude,latitude, $2, $3, $4, $5)
    "##
        );
        let q = QueryBuilder::new(&r2)
            .arg(time_from)
            .arg(lon)
            .arg(lat)
            .arg(dist)
            .arg(dist);
        let _ = dbh.execute(q).await?;

        // Check how many
        //
        let mut count = dbh
            .query_one::<RawRow>(QueryBuilder::new(&format!(
                "SELECT COUNT() FROM candidates{tag}"
            )))
            .await?;

        self.state.push(TempTables::Candidates);
        let count = count.get(0);
        trace!("Total number of drones: {}", count);
        Ok(count)
    }

    #[tracing::instrument(skip(dbh))]
    async fn find_close(&mut self, dbh: &Client) -> Result<usize> {
        trace!("Find close encounters.");

        let site = self.site.clone();
        let name = site.name.clone();
        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{name}_{day_name}");

        trace!("Removing old table today_close{tag}.");

        // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
        // altitude diff is less than 3 nm. (parameter is `separation`).
        //
        // $1,$2 = distance we consider as significan, 3nm for now approx 5,500 m.
        //
        let r = format!(
            r##"
CREATE OR REPLACE TABLE today_close{tag}
ENGINE = MergeTree
ORDER BY (journey, time)
AS SELECT
  c.journey AS journey,
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
  ceil(abs(palt - dalt)) AS diff_alt
FROM
  candidates{tag} AS c JOIN today{tag} AS t
ON
  toStartOfInterval(pt, toIntervalSecond(2)) = toStartOfInterval(c.timestamp, toIntervalSecond(2)) OR
  toStartOfInterval(pt, toIntervalSecond(2)) = toStartOfInterval(addSeconds(c.timestamp, 2), toIntervalSecond(2))
WHERE
  dist2d <= $1 AND
  diff_alt < $1
    "##
        );

        let proximity = self.separation;
        let q = QueryBuilder::new(&r).arg(proximity);
        dbh.execute(q).await?;

        // Check how many
        //
        let mut count = dbh
            .query_one::<RawRow>(&format!("SELECT COUNT() FROM today_close{tag}"))
            .await?;

        let count: u32 = count.get(0);

        self.state.push(TempTables::TodayClose);

        trace!("Total number of potential encounters: {}", count);
        Ok(count as usize)
    }

    #[tracing::instrument(skip(dbh))]
    async fn create_table_ids(&mut self, dbh: &Client, day_name: &str, site: &str) -> Result<()> {
        let tag = format!("_{site}_{day_name}");

        trace!("Create table ids{tag}.");
        let r = format!(
            r##"
CREATE OR REPLACE TABLE ids{tag} (
    drone_id VARCHAR,
    callsign VARCHAR,
    journey INT,
    en_id VARCHAR DEFAULT '',
) ENGINE = Memory
"##
        );
        self.state.push(TempTables::Ids);

        Ok(dbh.execute(&r).await?)
    }

    #[tracing::instrument(skip(dbh))]
    async fn insert_ids(&mut self, dbh: &Client, day_name: &str, site: &str) -> Result<usize> {
        let tag = format!("_{site}_{day_name}");

        let r = format!("SELECT count() FROM today_close{tag}");
        let mut total = dbh.query_one::<RawRow>(&r).await?;
        let total: u32 = total.get(0);

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
            r##"
    SELECT
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
        let all = dbh.query_collect::<Tc>(&r).await?;

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
        let _ = dbh
            .insert_native_block(&format!("INSERT INTO ids{tag} FORMAT native"), all)
            .await?;

        let mut count = dbh
            .query_one::<RawRow>(&format!("SELECT count() FROM today_close{tag}"))
            .await?;
        let count: u32 = count.get(0);
        trace!("Got {count} IDs");
        Ok(count as usize)
    }

    #[tracing::instrument(skip(dbh))]
    async fn select_encounters(&mut self, dbh: &Client) -> Result<usize> {
        trace!("select and record close points. ");

        // We use a GROUP BY() clause to get the point where the distance between this drone and any surrounding planes
        // is minimal.  Gather more information about the encounter, `any_value()` is used to avoid "duplicates".
        // Then the result of this sub-query is inserted (or replaced if we re-ran the calculation) in the
        // `encounters` table.

        let site = self.site.clone();
        let name = site.name.clone();
        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{name}_{day_name}");

        // Insert data into table `encounters`
        //
        // - create sequence
        // - create table for ids
        // - select unique encounter for id generation
        // - insert ids
        // - join today_close and ids to get all points with the right en_id
        //

        self.create_table_ids(dbh, &day_name, &name).await?;
        self.insert_ids(dbh, &day_name, &name).await?;

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
      any_value(ABS(dalt - dh)) AS drone_height_m,
      any_value(tc.callsign) AS prox_callsign,
      addr AS prox_id,
      any_value(plat) AS prox_lat,
      any_value(plon) AS prox_lon,
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
        dbh.execute(&r).await?;

        self.state.push(TempTables::Ids);

        // Now check how many
        //
        let pattern = format!("%{day_name}%");
        let q = QueryBuilder::new("SELECT COUNT(en_id) FROM airplane_prox WHERE en_id LIKE $1")
            .arg(pattern);
        let mut count = dbh.query_one::<RawRow>(q).await?;

        let count: u32 = count.get(0);
        if count == 0 {
            info!("No new encounters.");
            return Ok(count as usize);
        } else {
            info!("Inserted {} new encounters", count);
        }
        Ok(count as usize)
    }

    /// Remove temporary tables.
    ///
    #[tracing::instrument(skip(dbh))]
    async fn cleanup_temp_tables(&self, dbh: &Client) -> Result<()> {
        let site = self.site.clone();
        let name = site.name.clone();
        let day_name = self.date.format("%Y%m%d").to_string();
        let tag = format!("_{name}_{day_name}");

        let list = self.state.clone();
        let res = list
            .into_iter()
            .map(|t| {
                let tag = tag.clone();

                async move {
                    match t {
                        TempTables::Today => {
                            dbh.execute(&format!("DROP TABLE IF EXISTS today{tag}"))
                                .await
                        }
                        TempTables::Candidates => {
                            dbh.execute(&format!("DROP TABLE IF EXISTS candidates{tag}"))
                                .await
                        }
                        TempTables::TodayClose => {
                            dbh.execute(&format!("DROP TABLE IF EXISTS today_close{tag}"))
                                .await
                        }
                        TempTables::Ids => {
                            dbh.execute(&format!("DROP TABLE IF EXISTS ids{tag}")).await
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        let _ = try_join_all(res).await;
        Ok(())
    }
}

impl Calculate for PlaneDistance {
    /// Run the process for the given day.
    ///
    /// XXX Should we keep the interactive display or not.  When running parallel runs it messes
    /// up the output.
    ///
    #[tracing::instrument(skip(self, dbh))]
    async fn run(&mut self, dbh: &Client) -> Result<Stats> {
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
            self.cleanup_temp_tables(dbh).await?;
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
            self.cleanup_temp_tables(dbh).await?;
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
            self.cleanup_temp_tables(dbh).await?;
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
            self.cleanup_temp_tables(dbh).await?;
            return Ok(Stats::Planes(stats.clone()));
        }
        stats.encounters = c_encounters;
        bar.message(format!("{} encounters.", c_encounters));
        sleep(Duration::from_millis(self.wait)).await;

        info!("Stats for {}\n{}", self.date, stats);
        bar.message("Done.");
        bar.finish();

        self.cleanup_temp_tables(dbh).await?;

        debug!("timings={timings:?}");

        Ok(Stats::Planes(stats.clone()))
    }
}
