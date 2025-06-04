# Description of the various tables & queries and their implementation

We consider three databases:

- DuckDB: currently used as an embedded database, so it is fast as there is no latency due to network
- Databend: was the first candidate, but it is lacking in Geo functions
- Clickhouse: coming from Yandex & Cloudflare

## DuckDB

All tables and macros configuration [here](DUCKDB.md)

## Clickhouse

### Database

```sql
CREATE
DATABASE acute COMMENT 'ACUTE Project data.';
```

### Functions

Geodesic distance rounded to the nearest upper integer.

```sql
CREATE FUNCTION dist_2d AS(dx, dy, px, py) ->
    ceil(geoDistance(dx, dy, px, py));
```

> NOTE: `geoDistance` returns an FLOAT32, not FLOAT64.

3D distance using the 2D Geodesic distance and altitude, rounded to the nearest upper integer.

```sql
CREATE FUNCTION dist_3d AS(dx, dy, dz, px, py, pz) ->
    ceil(sqrt(pow(dist_2d(dx, dy, px, py), 2) + pow((dz - pz), 2)));
```

### Tables

```sql
-- Store data for the sites
--
CREATE TABLE acute.sites
(
    id           INTEGER,
    name         VARCHAR NOT NULL,
    code         VARCHAR NOT NULL,
    basename     VARCHAR NOT NULL,
    latitude     FLOAT   NOT NULL,
    longitude    FLOAT   NOT NULL,
    ref_altitude FLOAT   NOT NULL,
) ENGINE MergeTree PRIMARY KEY id ORDER BY id
      COMMENT 'All sites with an antenna in time.';
```

```sql
-- Store one antenna
--
CREATE TABLE acute.antennas
(
    id          INTEGER,
    type        VARCHAR,
    name        VARCHAR NOT NULL,
    owned       BOOLEAN,
    description VARCHAR,
) ENGINE MergeTree PRIMARY KEY id ORDER BY id
      COMMENT 'All known antennas.';
```

```sql
-- Store one installation of one antenna on one site 1:1
--
CREATE TABLE acute.installations
(
    id         INTEGER,
    site_id    INTEGER,
    antenna_id INTEGER,
    start_at   TIMESTAMP,
    end_at     TIMESTAMP,
    comment    VARCHAR,
) ENGINE MergeTree PRIMARY KEY id ORDER BY id
      COMMENT 'Which antenna on each site in time.';
```

`installations` is also the base for two views to help finding some info, `deployments` and `pbi_deployments`

```sql
 CREATE VIEW acute.deployments
AS
SELECT i.id       AS install_id,
       i.start_at,
       i.end_at,
       a.type,
       a.name     AS antenna_name,
       s.name     AS site_name,
       s.timezone AS timezone
FROM acute.installations AS i,
     acute.antennas AS a,
     acute.sites AS s
WHERE (i.antenna_id = a.id)
  AND (s.id = i.site_id) COMMENT 'Find the site for each drone points.'
```

and

```sql
 CREATE VIEW acute.pbi_deployments
AS
SELECT i.id           AS installation_id,
       i.start_at,
       i.end_at,
       a.type,
       a.name         AS antenna_name,
       s.name         AS sitename,
       s.offset       AS timezone s.latitude AS latitude, s.longitude AS longitude,
       s.ref_altitude AS ref_altitude,
FROM acute.installations AS i,
     acute.antennas AS a,
     acute.sites AS s
WHERE (i.antenna_id = a.id)
  AND (s.id = i.site_id) COMMENT 'Find the site for each drone points for PBI.'
```

```sql
CREATE
OR REPLACE TABLE acute.airplane_prox
(
    site             VARCHAR,
    en_id            VARCHAR,
    time             TIMESTAMP,
    journey          INT,
    drone_id         VARCHAR,
    model            VARCHAR,
    drone_lon        FLOAT,
    drone_lat        FLOAT,
    drone_alt_m      FLOAT,
    drone_height_m   FLOAT,
    prox_callsign    VARCHAR,
    prox_id          VARCHAR,
    prox_lon         FLOAT,
    prox_lat         FLOAT,
    prox_alt_m       FLOAT,
    distance_slant_m INT,
    distance_hor_m   INT,
    distance_vert_m  INT,
    distance_home_m  INT
)
    ENGINE = MergeTree PRIMARY KEY (time, journey)
        COMMENT 'Store all plane-drone encounters with less then 1nm distance.';
```

and we have a PBI-tailored view as well:

```sql
CREATE
MATERIALIZED VIEW acute.pbi_encounters
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (
SELECT
  en_id,
  installation_id,
  site,
  d.sitename,
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
  distance_slant_m,
  distance_hor_m,
  distance_vert_m,
  distance_home_m
FROM acute.airplane_prox AS ap, acute.pbi_deployments AS d
LEFT OUTER JOIN acute.sites AS s
ON ap.site = s.id
WHERE s.name = d.sitename
)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance for PBI.';
```

```sql
CREATE
OR REPLACE TABLE daily_stats
(
    date       DATE,
    planes     INT,
    drones     INT,
    potential  INT,
    encounters INT,
    distance   FLOAT,
    proximity  FLOAT
) ENGINE = MergeTree PRIMARY KEY (date) ORDER BY date
      COMMENT 'Statistics for a day run.'
```

This is the schema stored in the parquet files, extracted from the CSV. We will change a few things during import. As
we are putting everything inside CH, no need for year/month optimisations we got earlier from having a view upon the
parquet files.

```sql
CREATE
OR REPLACE TABLE acute.airplanes_raw
(
    site                   INT,
    EmitterCategory        INT DEFAULT 3,
    GBS                    INT,
    ModeA                  VARCHAR,
    TimeRecPosition        DATETIME64,
    AircraftAddress        VARCHAR,
    Latitude               DOUBLE,
    Longitude              DOUBLE,
    GeometricAltitude      DOUBLE,
    FlightLevel            DOUBLE,
    BarometricVerticalRate VARCHAR,
    GeoVertRateExceeded    VARCHAR,
    GeometricVerticalRate  VARCHAR,
    GroundSpeed            DOUBLE,
    TrackAngle             DOUBLE,
    Callsign               VARCHAR,
    AircraftStopped        VARCHAR,
    GroundTrackValid       VARCHAR,
    GroundHeadingProvided  VARCHAR,
    MagneticNorth          VARCHAR,
    SurfaceGroundSpeed     VARCHAR,
    SurfaceGroundTrack     VARCHAR
) ENGINE = MergeTree PRIMARY KEY (TimeRecPosition, AircraftAddress)
      COMMENT 'Table for raw ADS-B positions.';
```

Then we create the view with our more usable names.

```sql
CREATE VIEW acute.airplanes
AS
(
SELECT EmitterCategory,
       (GBS == 1) AS GBS,
       ModeA,
       TimeRecPosition AS time,
    AircraftAddress AS prox_id,
    Latitude AS prox_lat,
    Longitude AS prox_lon,
    truncate (GeometricAltitude * 0.305) AS prox_alt_m,
    FlightLevel AS flight_level,
    BarometricVerticalRate AS baro_vert_rate,
    (GeoVertRateExceeded == '1') AS geo_vert_exceeded,
    GeometricVerticalRate AS geo_vert_rate,
    GroundSpeed AS ground_speed,
    TrackAngle,
    Callsign AS prox_callsign,
    (AircraftStopped == '1') AS stopped,
    (GroundTrackValid == '1') AS GroundTrackValid,
    (GroundHeadingProvided == '1') AS GroundHeadingProvided,
    (MagneticNorth == '1') AS MagneticNorth,
    SurfaceGroundSpeed,
    SurfaceGroundTrack,
    site
FROM acute.airplanes_raw
    ) COMMENT 'View for airplanes data.'
```

```sql
CREATE
OR REPLACE TABLE acute.drones_raw
(
    journey           INT,
    ident             VARCHAR,
    model             VARCHAR,
    source            VARCHAR,
    location          INT,
    timestamp         TIMESTAMP,
    latitude          DOUBLE,
    longitude         DOUBLE,
    altitude          INTEGER,
    elevation         INTEGER,
    gps               INTEGER,
    rssi              INTEGER,
    home_lat          DOUBLE,
    home_lon          DOUBLE,
    home_height       INT,
    speed             INT,
    heading           INT,
    station_name      VARCHAR,
    station_latitude  DOUBLE,
    station_longitude DOUBLE
)
    ENGINE = MergeTree PRIMARY KEY (journey, timestamp)
        COMMENT 'Raw positions for drones on all sites.'
```

Initial data is loaded with:

```shell
clickhouse client -d acute -q "insert into acute.drones from infile 'data/drones/**/*.parquet' format parquet"
```

From `drones`, we derive two different materialized views, `drones` and `pbi_drones`.

```sql
CREATE
MATERIALIZED VIEW acute.drones
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
    FROM acute.drones_raw
)
    COMMENT 'View for drones data with distances.'
```

```sql
CREATE
MATERIALIZED VIEW acute.pbi_drones
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (SELECT `journey`,
      `ident`,
      `model`,
      d.installation_id,
      sitename,
      date_trunc('day', dr.timestamp) AS `date`,
      formatDateTime(dr.timestamp, '%T', 'UTC') AS `utc_time`,
      formatDateTime((timestamp + d.timezone * 3600), '%T', 'UTC') AS local_time,
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
    FROM acute.drones_raw AS dr LEFT OUTER JOIN acute.pbi_deployments AS d
    ON dr.station_name = d.antenna_name and dr.timestamp between d.start_at and d.end_at
    WHERE sitename = d.sitename AND dr.station_name != 'ASDSTATIONV1'
  )
  COMMENT 'PBI View for drones data with distances.'
```

AVIONIX streaming data:

```sql
CREATE
OR REPLACE TABLE acute.avionix_drones_raw
(
    uti  INT,
    dat  VARCHAR, ,
    hex  VARCHAR,
    tim  VARCHAR,
    fli  VARCHAR,
    lat  DOUBLE,
    lon  DOUBLE,
    gda  VARCHAR,
    src  VARCHAR,
    alt  INT,
    altg INT,
    hgt  INT,
    spd  INT,
    cat  VARCHAR,
    squ  VARCHAR,
    vrt  INT,
    trk  DOUBLE,
    mop  INT,
    lla  INT,
    tru  INT,
    dbm  INT,
    shd  INT,
    org  INT,
    dst  INT,
    opr  VARCHAR,
    typ  VARCHAR,
    reg  VARCHAR,
    cou  VARCHAR,
)
    ENGINE = MergeTree PRIMARY KEY (uti, fli)
        COMMENT 'Raw positions for drones from Cube.'
```

Updating the distances:

```sql
CREATE
MATERIALIZED VIEW acute.drones
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
    FROM acute.drones_raw
)
    COMMENT 'View for drones data with distances.'
```

Join the main metadata tables to identify the site on which every antenna was installed on in time.

```sql
 CREATE VIEW acute.deployments
AS
SELECT i.id       AS install_id,
       i.start_at,
       i.end_at,
       a.type,
       a.name     AS antenna_name,
       s.name     AS site_name,
       s.timezone AS timezone
FROM acute.installations AS i,
     acute.antennas AS a,
     acute.sites AS s
WHERE (i.antenna_id = a.id)
  AND (s.id = i.site_id) COMMENT 'Find the site for each drone points.'
```

And its PowerBI (pbi)-tailored version.

```sql
 CREATE VIEW acute.pbi_deployments
AS
SELECT i.id           AS installation_id,
       i.start_at,
       i.end_at,
       a.type,
       a.name         AS antenna_name,
       s.name         AS sitename,
       s.offset       AS timezone s.latitude AS latitude, s.longitude AS longitude,
       s.ref_altitude AS ref_altitude,
FROM acute.installations AS i,
     acute.antennas AS a,
     acute.sites AS s
WHERE (i.antenna_id = a.id)
  AND (s.id = i.site_id) COMMENT 'Find the site for each drone points for PBI.'
```

Table to store the history of `process-data distances` runs.

```sql
CREATE TABLE daily_stats
(
    day       DATE,
    site_id   INT,
    site_name VARCHAR,
    status    INT NOT NULL,
    stats     VARCHAR,
    comment   VARCHAR,
) ENGINE = ReplacingMergeTree PRIMARY KEY ( day, site_name)
    COMMENT 'Records the run history for all sites every day.';
```
