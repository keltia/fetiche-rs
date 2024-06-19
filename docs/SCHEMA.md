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
CREATE FUNCTION dist_2d AS (dx, dy, px, py) ->
  ceil(geoDistance(dx,dy,px,py));
```

> NOTE: `geoDistance` returns an FLOAT32, not FLOAT64.

3D distance using the 2D Geodesic distance and altitude, rounded to the nearest upper integer.

```sql
CREATE FUNCTION dist_3d AS (dx, dy, dz, px, py, pz) ->
  ceil(sqrt(pow(dist_2d(dx,dy,px,py), 2) + pow((dz-pz), 2)));
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
    ref_altitude FLOAT NOT NULL,
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

```sql
CREATE
OR REPLACE TABLE acute.airplane_prox (
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

```sql
CREATE
OR REPLACE TABLE daily_stats (
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
CREATE
OR REPLACE VIEW acute.airplanes 
AS
(
    SELECT EmitterCategory,
       (GBS == 1)                     AS GBS,
       ModeA,
       TimeRecPosition                AS time,
       AircraftAddress                AS prox_id,
       Latitude                       AS prox_lat,
       Longitude                      AS prox_lon,
       GeometricAltitude              AS prox_alt,
       FlightLevel                    AS flight_level,
       BarometricVerticalRate         AS baro_vert_rate,
       (GeoVertRateExceeded == '1')   AS geo_vert_exceeded,
       GeometricVerticalRate          AS geo_vert_rate,
       GroundSpeed                    AS ground_speed,
       TrackAngle,
       Callsign                       AS prox_callsign,
       (AircraftStopped == '1')       AS stopped,
       (GroundTrackValid == '1')      AS GroundTrackValid,
       (GroundHeadingProvided == '1') AS GroundHeadingProvided,
       (MagneticNorth == '1')         AS MagneticNorth,
       SurfaceGroundSpeed,
       SurfaceGroundTrack,
       site
    FROM acute.airplanes_raw AS f
)
    COMMENT 'View for airplanes data.'
```

```sql
CREATE OR REPLACE TABLE acute.drones_raw (
    journey            INT, 
    ident              VARCHAR,
    model              VARCHAR,
    source             VARCHAR,
    location           INT,
    timestamp          TIMESTAMP,
    time               INT,
    latitude           DOUBLE, 
    longitude          DOUBLE, 
    altitude           INTEGER,
    elevation          INTEGER,
    gps                INTEGER,
    rssi               INTEGER,
    home_lat           DOUBLE,
    home_lon           DOUBLE,
    home_height        INT,
    speed              INT,
    heading            INT,
    station_name       VARCHAR,
    station_latitude   DOUBLE, 
    station_longitude  DOUBLE
)
    ENGINE = MergeTree PRIMARY KEY (journey, timestamp)
    COMMENT 'Raw positions for drones on all sites.'
```

Initial data is loaded with:
```shell
clickhouse client -d acute -q "insert into acute.drones from infile 'data/drones/**/*.parquet' format parquet"
```

Updating the distances:
```sql
CREATE OR REPLACE VIEW acute.drones AS
(
    SELECT
        *,
        toUnixTimestamp(timestamp) as time,
        dist_2d(longitude,latitude,home_lon,home_lat) AS home_distance_2d, 
        dist_3d(longitude,latitude,elevation,home_lon,home_lat,home_height) AS home_distance_3d
    FROM acute.drones_raw
)
    COMMENT 'View for drones data.'
```
