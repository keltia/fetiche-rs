//! # Plane-Drone Distance Calculations Module
//!
//! This module is responsible for performing calculations related to determining distances between 
//! airplanes and drones based on geospatial proximity and temporal criteria. It interacts with a 
//! ClickHouse database to query and manipulate data, ensuring efficient and precise computations 
//! of distances and encounters.
//!
//! The primary objectives include:
//! - **Airplane Data Selection**: Filter airplanes positions for a specific site and time range 
//!   that fall within a defined proximity area.
//! - **Drone Data Selection**: Extract drone positions over a specific day, filtered using geospatial 
//!   proximity rules.
//! - **Distance Calculations**: Compute distances between airplanes and drones based on their 
//!   geospatial and temporal data.
//! - **Encounters Identification**: Identify close encounters based on predetermined thresholds 
//!   such as proximity and altitude differences.
//!
//! ## Database Interaction
//!
//! This module relies heavily on ClickHouse, leveraging features like temporary tables and geospatial 
//! functions such as `pointInEllipses`. Temporary tables are created to handle filtered airplane and 
//! drone data, which are then used in subsequent calculations.
//!
//! ## Key Features
//!
//! - **Robust Error Handling**: Ensures graceful handling of database or query failures and provides 
//!   informative logging for debugging.
//! - **High Performance**: Optimized SQL queries and efficient use of ClickHouse features to handle 
//!   large volumes of data with minimal latency.
//! - **Configurability**: Parameters such as distance (in nautical miles) and temporal ranges can be 
//!   customized.
//!
//! ## Components
//!
//! - **Timings Struct**: Tracks execution times for key operations such as selecting airplane and 
//!   drone data or computing close encounters.
//! - **PlaneDistance Methods**: The main driver for filtering and calculations, equipped with 
//!   functions for data extraction, proximity-based filtering, and interaction with ClickHouse queries.
//!
//! ## Usage
//!
//! The module is part of a larger command suite that utilizes structs like `Calculate` and `PlanesStats` 
//! to manage airplane and drone data. Each function is designed to be asynchronous to ensure 
//! non-blocking operations and scalability.
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
    /// This function gathers the list of airplanes within a given proximity area around a specified site.
    /// The filtering criteria include:
    ///
    /// - Specific date or time (bounded to a single day).
    /// - A defined proximity area (bounding ellipse), based on a configurable distance (default 70 nm).
    ///
    /// Steps performed:
    /// 1. Calculate the bounding ellipse using the site location and specified distance.
    /// 2. Create a temporary table (`today{tag}`) in the database for storing the filtered airplane data.
    /// 3. Populate the table with airplane data, such as location, altitude, and timestamp, extracted
    ///    from the `airplanes` table after applying the bounding filter and other conditions like altitude presence.
    /// 4. Validate table creation and count the selected entries.
    ///
    /// If the table fails to be created (e.g., no matching airplane data was found), the function will safely
    /// handle this and return a count of 0 entries.
    ///
    /// # Parameters
    ///
    /// - `dbh`: A reference to the database client used for executing queries.
    ///
    /// # Returns
    ///
    /// `Result<usize>`: The number of airplanes selected, or an error if the operation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following occurs:
    /// - Creating or populating the temporary table fails.
    /// - Querying the count of entries in the temporary table fails.
    ///
    /// The function is designed with proper error handling to ensure robustness in cases where
    /// the expected table or data is not present due to query constraints.
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
  prox_alt_m AS palt,
  ModeA AS prox_mode_a
FROM
  airplanes
WHERE
  site = $1 AND
  toStartOfInterval(time, toIntervalDay(1)) = toDateTime($2) AND
  palt IS NOT NULL AND
  NOT(palt = 0 AND flight_level != 0) AND
  pointInEllipses(plon, plat, $3, $4, $5, $6)
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
        dbh.execute(q).await?;
        let tm = (Instant::now() - tm).as_millis();
        trace!("CREATE TABLE today{tag} took {tm} ms");

        // If there were no planes, then the query succeeds, but the table was not created. Counting
        // WILL fail.  We need to handle that.
        //
        let mut count = match dbh
            .query_one::<RawRow>(&format!("SELECT count() FROM today{tag}"))
            .await
        {
            Ok(count) => count,
            Err(_) => {
                trace!("Table today{tag} was not created, assume 0");
                return Ok(0);
            }
        };

        self.state.push(TempTables::Today);
        let count: u64 = count.get(0);
        trace!("Total number of planes: {}\n", count);
        Ok(count as usize)
    }

    /// Selects drone points for a specific day based on geospatial and temporal proximity criteria.
    ///
    /// This function utilizes the geospatial function `pointInEllipses` to filter drones within a
    /// defined elliptical area around the site. The distance from the site is calculated in degrees,
    /// based on the nautical mile parameter specified in the struct.
    ///
    /// A temporary table `candidates{tag}` is created to store drone points matching the criteria,
    /// and its count is tracked for further calculations.
    ///
    /// Temporary table fields:
    /// - `time`: Timestamp of the drone point.
    /// - `journey`: Identifier for the drone's journey.
    /// - `ident`: Unique identifier for the drone.
    /// - `model`: Drone model.
    /// - `timestamp`: Timestamp of the drone point (duplicate of `time`).
    /// - `latitude`: Latitude of the drone point.
    /// - `longitude`: Longitude of the drone point.
    /// - `altitude_geo`: Geographical altitude of the drone.
    /// - `elevation`: Elevation of the drone.
    /// - `home_lat`: Latitude of the drone's home point.
    /// - `home_lon`: Longitude of the drone's home point.
    /// - `home_distance_2d`: 2D distance from the home point.
    /// - `home_distance_3d`: 3D distance from the home point.
    ///
    /// The function also keeps track of the created temporary table for cleanup in case of
    /// process interruption.
    ///
    /// # Returns
    /// `Ok(usize)` with the count of drones matching the criteria.
    /// `Err` in case of any errors during table creation or querying the table.
    ///
    #[tracing::instrument(skip(dbh))]
    async fn select_drones(&mut self, dbh: &Client) -> Result<usize> {
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
    altitude_geo,
    elevation,
    home_lat,
    home_lon,
    home_distance_2d,
    home_distance_3d
FROM drones
WHERE
  toStartOfInterval(timestamp, toIntervalDay(1)) = toDateTime($1) AND
  altitude_geo IS NOT NULL AND
  latitude IS NOT NULL AND
  longitude IS NOT NULL AND
  pointInEllipses(longitude,latitude, $2, $3, $4, $5)
    "##
        );
        let q = QueryBuilder::new(&r2)
            .arg(time_from)
            .arg(lon)
            .arg(lat)
            .arg(dist)
            .arg(dist);
        dbh.execute(q).await?;

        // Check how many
        //
        let mut count = match dbh
            .query_one::<RawRow>(&format!("SELECT COUNT() FROM candidates{tag}"))
            .await
        {
            Ok(count) => count,
            Err(_) => {
                trace!("Table candidates{tag} was not created, assume 0");
                return Ok(0);
            }
        };

        self.state.push(TempTables::Candidates);
        let count: u64 = count.get(0);
        trace!("Total number of drones: {}", count);
        Ok(count as usize)
    }

    /// Identifies and creates a table for potential close encounters between planes
    /// and drones within specified proximity and altitude difference.
    ///
    /// # Description
    /// This method analyzes drone and plane data to detect geospatial and temporal
    /// proximity between them. The function creates a temporary table (`today_close`)
    /// to store these encounters for further processing or reporting.
    ///
    /// Proximity is defined as:
    /// - 2D geospatial distance between plane and drone locations (`dist2d`).
    /// - The absolute altitude difference between the plane and drone (`diff_alt`).
    ///
    /// Parameters passed to the query:
    /// - `$1`: Proximity distance in meters (e.g., 5500 meters).
    ///
    /// The temporary table `today_close` will include:
    /// - Drone journey information such as ID, model, and geolocation (lat, lon, alt).
    /// - Plane information such as site, callsign, and geolocation (lat, lon, alt).
    /// - Calculated fields like 2D and 3D distances, and altitude differences.
    ///
    /// The function also keeps track of the temporary tables created in the `state`
    /// attribute to ensure proper cleanup processes.
    ///
    /// # Returns
    /// `Ok(usize)` with the count of encounters found.
    /// `Err` in case of any errors during table creation or querying.
    ///
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
        // $1,$2 = distance we consider as significant, 3nm for now approx 5,500 m.
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
  c.altitude_geo AS dalt,
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
  t.prox_alt_a,
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

        let separation = self.threshold * self.factor;
        let q = QueryBuilder::new(&r).arg(separation);
        dbh.execute(q).await?;

        // Check how many
        //
        let mut count = match dbh
            .query_one::<RawRow>(&format!("SELECT COUNT() FROM today_close{tag}"))
            .await
        {
            Ok(count) => count,
            Err(_) => {
                error!("today_close{tag} was not created, assume 0.");
                return Ok(0);
            }
        };

        let count: u64 = count.get(0);

        self.state.push(TempTables::TodayClose);

        trace!("Total number of potential encounters: {}", count);
        Ok(count as usize)
    }

    /// Creates a temporary table `ids` to store unique drone and plane encounter IDs for a given site and day.
    ///
    /// This function initializes the `ids` table structure with relevant fields such as `drone_id`, `callsign`, and
    /// `journey`. It also adds an optional `en_id` field (encounter ID) which is populated later after identifying
    /// close encounters. The table is created in memory for quick access and temporary usage.
    ///
    /// The table name is dynamically generated based on the site's name and the day's date to ensure uniqueness
    /// and avoid conflicts with other calculations.
    ///
    /// # Parameters
    /// - `dbh`: Database client used to execute the table creation query.
    /// - `day_name`: String representation of the day (format: YYYYMMDD) to include in the table name.
    /// - `site`: Name of the site to include in the table name.
    ///
    /// # Returns
    /// - `Ok(())` if the table is successfully created.
    /// - `Err` if there is any error during the table creation process.
    ///
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

    /// Inserts drones and planes' encounter IDs into the temporary `ids` table.
    ///
    /// This function retrieves close drone-plane encounters from the `today_close` table and assigns
    /// them unique encounter IDs based on the day, site, and a per-journey increment. The newly created
    /// IDs are structured for each encounter and inserted into the `ids` table.
    ///
    /// # Parameters
    /// - `dbh`: Database client used for query execution.
    /// - `day_name`: String representation of the day (format: YYYYMMDD) used in table tagging and identification.
    /// - `site`: The associated site name for table tagging and identification.
    ///
    /// # Returns
    /// - `Ok(usize)` representing the number of records successfully inserted into the `ids` table.
    /// - `Err` if any error occurs during query execution or insertion.
    ///
    #[tracing::instrument(skip(dbh))]
    async fn insert_ids(&mut self, dbh: &Client, day_name: &str, site: &str) -> Result<usize> {
        let tag = format!("_{site}_{day_name}");

        let r = format!("SELECT count() FROM today_close{tag}");
        let mut total = dbh.query_one::<RawRow>(&r).await?;
        let total: u64 = total.get(0);

        // This is for the query
        #[derive(Clone, Debug, Default, Serialize, Deserialize, Row)]
        struct Tc {
            journey: i32,
            drone_id: String,
            callsign: String,
        }

        // This is for the insert as the CH client does not support serde `skip_deserializing`.
        #[derive(Clone, Debug, Default, Serialize, Deserialize, Row)]
        struct Ids {
            en_id: String,
            journey: i32,
            drone_id: String,
            callsign: String,
        }

        let separation = self.threshold * self.factor;

        let r = format!(
            r##"
    SELECT
      journey,
      drone_id,
      callsign,
    FROM today_close{tag}
    WHERE
      dist_drone_plane < {separation}
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
        dbh.insert_native_block(&format!("INSERT INTO ids{tag} FORMAT native"), all)
            .await?;

        let mut count = dbh
            .query_one::<RawRow>(&format!("SELECT count() FROM today_close{tag}"))
            .await?;
        let count: u64 = count.get(0);
        trace!("Got {count} IDs");
        Ok(count as usize)
    }

    /// Selects and records close encounters between drones and planes.
    ///
    /// This function identifies points of minimal distance between a drone and any surrounding plane
    /// and records relevant encounter information. The process involves grouping and filtering data to
    /// extract unique encounters, and subsequently storing this data in the `airplane_prox` table for
    /// further analysis.
    ///
    /// # Returns
    /// - `Ok(usize)` indicating the number of new encounters inserted.
    /// - `Err` if an error occurs during table creation, data selection, or insertions.
    ///
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

        let threshold = self.threshold;

        let r = format!(
            r##"INSERT INTO airplane_prox
     SELECT
      any_value(tc.site) AS site,
      id.en_id AS en_id,
      any_value(time) AS time,
      tc.journey AS journey,
      tc.drone_id AS drone_id,
      any_value(model) AS model,
      any_value(dlat) AS drone_lat,
      any_value(dlon) AS drone_lon,
      any_value(dalt) AS drone_alt_m,
      any_value(ABS(dalt - dh)) AS drone_height_m,
      any_value(tc.callsign) AS prox_callsign,
      addr AS prox_id,
      any_value(plat) AS prox_lat,
      any_value(plon) AS prox_lon,
      any_value(palt) AS prox_alt_m,
      any_value(prox_mode_a) AS prox_mode_a,
      CEIL(dist_drone_plane) AS distance_slant_m,
      any_value(CEIL(dist2d)) AS distance_hor_m,
      any_value(CEIL(ABS(palt - dalt))) AS distance_vert_m,
      any_value(CEIL(hdist2d)) as distance_home_m
    FROM today_close{tag} AS tc JOIN ids{tag} AS id
      ON id.journey = tc.journey AND id.callsign = tc.callsign
    WHERE
      dist_drone_plane < {threshold}
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

        let count: u64 = count.get(0);
        if count == 0 {
            info!("No new encounters.");
        } else {
            info!("Inserted {} new encounters", count);
        }
        Ok(count as usize)
    }

    /// This function removes all temporary tables created during the calculation process.
    ///
    /// Temporary tables are dropped based on their type (`Today`, `Candidates`, `TodayClose`, `Ids`)
    /// and the associated tag, which is a combination of the site name and date.
    ///
    /// This cleanup step ensures that no leftover resources remain in the database after processing.
    ///
    /// # Parameters
    /// - `dbh`: A reference to the database client used to execute the table removal queries.
    ///
    /// # Returns
    /// - `Ok(())` if all temporary tables are successfully dropped.
    /// - `Err` if an error occurs while dropping any of the tables.
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
    /// This function orchestrates the calculations for planes and drones within
    /// a predefined distance and separation on a particular day. It performs a
    /// step-by-step process:
    ///
    /// 1. Select planes within the specified range.
    /// 2. Select drones within the specified range.
    /// 3. Identify potential close encounters between planes and drones.
    /// 4. Calculate final close encounters based on proximity thresholds.
    ///
    /// Each step is accompanied by progress updates via `ml_progress` for better
    /// visibility during execution. Temporary tables are created during the process
    /// and are cleaned up afterwards.
    ///
    /// FIXME: parallel processing and progress bar is messed up
    ///
    /// # Parameters
    /// - `dbh`: A reference to the database client used for query execution.
    ///
    /// # Returns
    /// - `Ok(Stats)` containing the statistics of planes, drones, and encounters.
    /// - `Err` if any step in the process encounters an error.
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
        let separation = self.threshold * self.factor;
        let stats = &mut PlanesStats::new(self.date, self.distance, separation);
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
