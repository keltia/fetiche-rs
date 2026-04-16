//! Module for all views creation
//!

use crate::cmds::DBVars;
use crate::make_query;
use crate::runtime::Context;

#[tracing::instrument(skip(ctx))]
async fn add_pbi_encounters_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let sq = make_query!(r##"
CREATE MATERIALIZED VIEW {workdb}.pbi_encounters
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (
SELECT
  en_id,
  installation_id,
  ap.site_id AS site_id,
  d.sitename AS sitename,
  d.antenna_name AS station_name,
  ap.time AS time,
  date_trunc('day', ap.time) AS `date`,
  formatDateTime(ap.time, '%T', 'UTC') AS `utc_time`,
  compute_localdate(toUnixTimestamp(ap.time), d.tzname) AS local_date,
  compute_localtime(toUnixTimestamp(ap.time), d.tzname) AS local_time,
  journey,
  drone_id,
  model,
  drone_lat,
  drone_lon,
  drone_alt_m,
  drone_height_m,
  prox_callsign,
  prox_id,
  prox_lat,
  prox_lon,
  prox_alt_m,
  prox_mode_a,
  prox_ecat,
  distance_slant_m,
  distance_hor_m,
  distance_vert_m,
  distance_home_m
FROM {workdb}.airplane_prox AS ap, {workdb}.pbi_deployments AS d
LEFT OUTER JOIN sites AS s
ON ap.site_id = s.id
WHERE s.name = d.sitename
)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance for PBI.';
    "##, dbvars);

    Ok(dbh.execute(&sq).await?)
}

/*
CREATE MATERIALIZED VIEW {workdb}.pbi_encounters
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (
SELECT
  en_id,
  installation_id,
  ap.site_id AS site_id,
  d.sitename AS sitename,
  d.antenna_name AS station_name,
  `time`,
  date_trunc('day', ap.time) AS `date`,
  formatDateTime(ap.time, '%T', 'UTC') AS `utc_time`,
  formatDateTime((ap.time + d.timezone * 3600), '%T', 'UTC') AS `local_time`,
  journey,
  drone_id,
  model,
  drone_lat,
  drone_lon,
  drone_alt_m,
  drone_height_m,
  prox_callsign,
  prox_id,
  prox_lat,
  prox_lon,
  prox_alt_m,
  prox_mode_a,
  prox_ecat,
  distance_slant_m,
  distance_hor_m,
  distance_vert_m,
  distance_home_m
FROM {workdb}.airplane_prox AS ap, {workdb}.pbi_deployments AS d
LEFT OUTER JOIN {workdb}.sites AS s
ON ap.site_id = s.id
WHERE s.name = d.sitename
)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance for PBI.';
    */

/// Drop the `pbi_encounters` table to store short air-prox points
///
#[tracing::instrument(skip(ctx))]
async fn drop_pbi_encounters_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let sq = make_query!(r##"
DROP VIEW IF EXISTS {workdb}.pbi_encounters;
    "##, dbvars);

    Ok(dbh.execute(&sq).await?)
}

// -----

#[tracing::instrument(skip(ctx))]
async fn add_pbi_encounters_summary_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r4 = make_query!(r##"
CREATE MATERIALIZED VIEW IF NOT EXISTS {workdb}.pbi_encounters_summary
ENGINE = ReplacingMergeTree
PRIMARY KEY (en_id)
POPULATE
AS (
  SELECT
      en_id,
      installation_id,
      site_id,
      sitename,
      station_name,
      time,
      date,
      utc_time,
      local_date,
      local_time,
      journey,
      drone_id,
      model,
      drone_lat,
      drone_lon,
      drone_alt_m,
      drone_height_m,
      prox_callsign,
      prox_id,
      prox_lat,
      prox_lon,
      prox_alt_m,
      prox_mode_a,
      prox_ecat,
      distance_slant_m,
      distance_hor_m,
      distance_vert_m,
      distance_home_m
  FROM
    {workdb}.pbi_encounters AS p JOIN {workdb}.airprox_summary AS s
    ON
        s.en_id = p.en_id AND
        s.journey = p.journey AND
        s.drone_id = p.drone_id
  WHERE
    p.distance_slant_m = s.distance_slant_m
  ORDER BY time
)
COMMENT 'Store all plane-drone encounters with less then 1nm distance for PBI, summarized by drone and encounter.'
"##, dbvars);

    Ok(dbh.execute(&r4).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_pbi_encounters_summary_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let rm5 = make_query!(r##"DROP VIEW IF EXISTS {workdb}.pbi_encounters_summary"##, dbvars);

    Ok(dbh.execute(&rm5).await?)
}

// -----

/// Create airplanes view
///
#[tracing::instrument(skip(ctx))]
pub async fn add_airplanes_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    // Calculations view
    //
    let r1 = make_query!(r##"
CREATE VIEW IF NOT EXISTS {planedb}.airplanes
AS
(
    SELECT
       site,
       EmitterCategory,
       (GBS == 1)                     AS GBS,
       ModeA,
       TimeRecPosition                AS time,
       AircraftAddress                AS prox_id,
       Latitude                       AS prox_lat,
       Longitude                      AS prox_lon,
       GeometricAltitude              AS prox_alt_m,
       FlightLevel                    AS flight_level,
       BarometricVerticalRate         AS baro_vert_rate,
       (GeoVertRateExceeded == '1')   AS geo_vert_exceeded,
       GeometricVerticalRate          AS geo_vert_rate,
       GroundSpeed                    AS ground_speed,
       TrackAngle,
       replaceRegexpOne(Callsign, '\'([0-9A-Z]+)\\s*\'', '\\1') AS prox_callsign,
       (AircraftStopped == '1')       AS stopped,
       (GroundTrackValid == '1')      AS GroundTrackValid,
       (GroundHeadingProvided == '1') AS GroundHeadingProvided,
       (MagneticNorth == '1')         AS MagneticNorth,
       SurfaceGroundSpeed,
       SurfaceGroundTrack
    FROM {planedb}.airplanes_raw AS f
    WHERE prox_lat != 0 AND prox_lon != '0'
    ORDER BY time
)
    COMMENT 'View for airplanes data.'
"##, dbvars);

    Ok(dbh.execute(&r1).await?)
}

/// Drop airplanes view
///
#[tracing::instrument(skip(ctx))]
pub async fn drop_airplanes_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let rm1 = make_query!(r##"
DROP VIEW IF EXISTS {planedb}.airplanes;
    "##, dbvars);

    Ok(dbh.execute(&rm1).await?)
}

// -----

/// Create drones view
///
/// XXX We use 0 for the drone height because we have no way to have the home altitude.
///
#[tracing::instrument(skip(ctx))]
pub async fn add_drones_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r2 = make_query!(r##"
CREATE MATERIALIZED VIEW {dronedb}.drones
    ENGINE = ReplacingMergeTree
    PRIMARY KEY (time, journey)
AS
(
    SELECT
        `journey`,
        `ident`,
        `model`,
        `source`,
        `timestamp`,
        `latitude`,
        `longitude`,
        `altitude`,
        ceil((CAST(altitude AS Float64)  + compute_height(latitude,longitude))) AS altitude_geo,
        `elevation`,
        `home_lat`,
        `home_lon`,
        `home_height`,
        `speed`,
        `heading`,
        `station_name`,
        `station_latitude`,
        `station_longitude`,
        toUnixTimestamp(timestamp) as time,
        dist_2d(longitude,latitude,home_lon,home_lat) AS home_distance_2d,
        dist_3d(longitude,latitude,0,home_lon,home_lat,home_height) AS home_distance_3d
    FROM {dronedb}.drones_raw
)
    COMMENT 'View for drones data with distances.'
"##, dbvars);

    Ok(dbh.execute(&r2).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_drones_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let rm2 = make_query!(r##"DROP VIEW IF EXISTS {dronedb}.drones"##, dbvars);

    Ok(dbh.execute(&rm2).await?)
}
// -----

/// Create PBI-specific drones view
///
/// XXX We use 0 for the drone height because we have no way to have the home altitude.
///
#[tracing::instrument(skip(ctx))]
async fn add_pbi_drones_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r2b = make_query!(r##"
CREATE MATERIALIZED VIEW  IF NOT EXISTS {workdb}.pbi_drones
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (SELECT `journey`,
      `ident`,
      `model`,
      d.installation_id,
      sitename,
      d.site_id,
      date_trunc('day', dr.timestamp) AS `date`,
      formatDateTime(dr.timestamp, '%T', 'UTC') AS `utc_time`,
      compute_localdate(toUnixTimestamp(dr.timestamp), d.tzname) AS local_date,
      compute_localtime(toUnixTimestamp(dr.timestamp), d.tzname) AS local_time,
      dr.latitude AS `drone_lat`,
      dr.longitude AS `drone_lon`,
      dr.altitude AS `drone_alt_m`,
      CEIL((CAST(dr.altitude AS Float64)  + compute_height(drone_lat,drone_lon))) AS `drone_alt_geo_m`,
      (dr.altitude - dr.elevation) AS `drone_height_m`,
      `elevation` AS `elevation_m`,
      `home_lat`,
      `home_lon`,
      `home_height` AS `drone_reported_height_m`,
      (`speed` / 3.6) AS `speed_m_s`,
      `heading`,
      `station_name`,
      `station_latitude`,
      `station_longitude`,
      toUnixTimestamp(timestamp) as time,
      dist_2d(dr.longitude,dr.latitude,home_lon,home_lat) AS home_distance_2d,
      dist_3d(dr.longitude,dr.latitude,0,home_lon,home_lat,drone_reported_height_m) AS home_distance_3d,
      dist_2d(dr.longitude,dr.latitude,station_longitude,station_latitude) AS antenna_distance_2d,
      dist_3d(dr.longitude,dr.latitude,dr.altitude,station_longitude,station_latitude, d.ref_altitude) AS antenna_distance_3d
    FROM {dronedb}.drones_raw AS dr LEFT OUTER JOIN {workdb}.pbi_deployments AS d
     ON dr.station_name = d.antenna_name and dr.timestamp between d.start_at and d.end_at
    WHERE dr.station_name != 'ASDSTATIONV1' AND sitename != ''
  )
  COMMENT 'PBI View for drones data with distances.'
"##, dbvars);

    Ok(dbh.execute(&r2b).await?)
}

/*
// Alternate version for later, with date & time separated for both UTC and Local time
//
    let r2b = r##"
CREATE MATERIALIZED VIEW  IF NOT EXISTS {workdb}.pbi_drones
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (SELECT `journey`,
      `ident`,
      `model`,
      d.installation_id,
      sitename,
      d.site_id,
      formatDateTime(dr.timestamp, '%F', 'UTC') AS `date`,
      formatDateTime(dr.timestamp, '%T', 'UTC') AS `utc_time`,
      compute_localdate(toUnixTimestamp(dr.timestamp), d.tzname) AS local_date,
      compute_localtime(toUnixTimestamp(dr.timestamp), d.tzname) AS local_time,
      dr.latitude AS `drone_lat`,
      dr.longitude AS `drone_lon`,
      dr.altitude AS `drone_alt_m`,
      CEIL((CAST(dr.altitude AS Float64)  + compute_height(drone_lat,drone_lon))) AS `drone_alt_geo_m`,
      (dr.altitude - dr.elevation) AS `drone_height_m`,
      `elevation` AS `elevation_m`,
      `home_lat`,
      `home_lon`,
      `home_height` AS `drone_reported_height_m`,
      (`speed` / 3.6) AS `speed_m_s`,
      `heading`,
      `station_name`,
      `station_latitude`,
      `station_longitude`,
      toUnixTimestamp(timestamp) as time,
      dist_2d(dr.longitude,dr.latitude,home_lon,home_lat) AS home_distance_2d,
      dist_3d(dr.longitude,dr.latitude,0,home_lon,home_lat,drone_reported_height_m) AS home_distance_3d,
      dist_2d(dr.longitude,dr.latitude,station_longitude,station_latitude) AS antenna_distance_2d,
      dist_3d(dr.longitude,dr.latitude,dr.altitude,station_longitude,station_latitude, d.ref_altitude) AS antenna_distance_3d
    FROM {dronedb}.drones_raw AS dr LEFT OUTER JOIN {workdb}.pbi_deployments AS d
     ON dr.station_name = d.antenna_name and dr.timestamp between d.start_at and d.end_at
    WHERE dr.station_name != 'ASDSTATIONV1' AND sitename != ''
  )
  COMMENT 'PBI View for drones data with distances.'
"##;

 */

#[tracing::instrument(skip(ctx))]
async fn drop_pbi_drones_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let rm2b = make_query!(r##"DROP VIEW IF EXISTS {workdb}.pbi_drones"##, dbvars);
    Ok(dbh.execute(&rm2b).await?)
}

// -----

#[tracing::instrument(skip(ctx))]
async fn add_deployments_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    // Deployments tracking view
    //
    let r3 = make_query!(r##"
 CREATE VIEW  IF NOT EXISTS {workdb}.deployments
 AS SELECT
    i.id AS install_id,
    i.start_at,
    i.end_at,
    a.type,
    a.name AS antenna_name,
    s.name AS site_name,
    s.id AS site_id,
    s.timezone AS timezone
 FROM {workdb}.installations AS i, {workdb}.antennas AS a, {workdb}.sites AS s
 WHERE (i.antenna_id = a.id) AND (s.id = i.site_id)
 COMMENT 'Find the site for each drone points.'
    "##, dbvars);

    Ok(dbh.execute(&r3).await?)
}

#[tracing::instrument(skip(ctx))]
async fn drop_deployments_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let rm3 = make_query!(r##"DROP VIEW IF EXISTS {workdb}.deployments"##, dbvars);

    Ok(dbh.execute(&rm3).await?)
}

// -----

#[tracing::instrument(skip(ctx))]
async fn add_pbi_deployments_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    // PBI-specific view
    //
    let r3b = make_query!(r##"
 CREATE VIEW  IF NOT EXISTS {workdb}.pbi_deployments
 AS SELECT
    i.id AS installation_id,
    i.start_at,
    i.end_at,
    a.type,
    a.name AS antenna_name,
    s.name AS sitename,
    s.id AS site_id,
    s.offset AS timezone,
    s.timezone AS tzname,
    s.latitude AS latitude,
    s.longitude AS longitude,
    s.ref_altitude AS ref_altitude
 FROM {workdb}.installations AS i, {workdb}.antennas AS a, {workdb}.sites AS s
 WHERE (i.antenna_id = a.id) AND (s.id = i.site_id)
 COMMENT 'Find the site for each drone points for PBI.'
    "##, dbvars);

    Ok(dbh.execute(&r3b).await?)
}

#[tracing::instrument(skip(ctx))]
async fn drop_pbi_deployments_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let rm4 = make_query!(r##"DROP VIEW IF EXISTS {workdb}.pbi_deployments"##, dbvars);

    Ok(dbh.execute(&rm4).await?)
}

// -----

#[tracing::instrument(skip(ctx))]
async fn add_airprox_summary_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r4 = make_query!(r##"
CREATE OR REPLACE VIEW  IF NOT EXISTS {workdb}.airprox_summary AS
(SELECT
        en_id,
        journey,
        drone_id,
        min(distance_slant_m) as distance_slant_m
    FROM
        {workdb}.airplane_prox
    GROUP BY
        en_id, journey, drone_id
    ORDER BY journey)
    COMMENT 'List all encounters ID with the minimum distance.'
    "##, dbvars);

    Ok(dbh.execute(&r4).await?)
}

#[tracing::instrument(skip(ctx))]
async fn drop_airprox_summary_view(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let rm4 = make_query!(r##"DROP VIEW IF EXISTS {workdb}.airprox_summary"##, dbvars);

    Ok(dbh.execute(&rm4).await?)
}

// -----

/// Create various views
///
#[tracing::instrument(skip(ctx))]
pub async fn create_work_views(ctx: &Context) -> eyre::Result<()> {
    add_deployments_view(ctx).await?;
    add_pbi_deployments_view(ctx).await?;
    add_pbi_drones_view(ctx).await?;
    add_airprox_summary_view(ctx).await?;
    add_pbi_encounters_view(ctx).await?;
    add_pbi_encounters_summary_view(ctx).await?;

    Ok(())
}

/// Drop all views
///
#[tracing::instrument(skip(ctx))]
pub async fn drop_work_views(ctx: &Context) -> eyre::Result<()> {
    drop_pbi_encounters_summary_view(ctx).await?;
    drop_pbi_encounters_view(ctx).await?;
    drop_airprox_summary_view(ctx).await?;
    drop_pbi_drones_view(ctx).await?;
    drop_pbi_deployments_view(ctx).await?;
    drop_deployments_view(ctx).await?;
    Ok(())
}

