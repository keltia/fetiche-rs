## DuckDB

### Macros

```sql
CREATE
OR REPLACE MACRO nm_to_deg(nm) AS
  nm * 1.852 / 111111.11;
```

```sql
CREATE
OR REPLACE MACRO deg_to_m(deg) AS
  deg * 111111.11;
```

```sql
CREATE
OR REPLACE MACRO m_to_deg(m) AS
  m / 111111.11;
```

```sql

CREATE
OR REPLACE MACRO dist_2d(px, py, dx, dy) AS
  CEIL(ST_Distance_Spheroid(ST_Point(py, px), ST_Point(dy, dx)));
```

> NOTE: ST_Distance_Spheroid() is undocumented, it was merged
> in [this PR](https://github.com/duckdb/duckdb_spatial/pull/74).

> NOTE: ST_Distance_Spheroid() right now use (lat, lon) parameters, not the usual (x, y)  where x = lon and y = lat.

```sql
CREATE
OR REPLACE MACRO dist_3d(px, py, pz, dx, dy, dz) AS
  CEIL(SQRT(POW(dist_2d(px, py, dx, dy), 2) + POW((pz - dz), 2)));
```

### Views

We use views to avoid the import of the whole data tree.

- all ADS-B points from the parquet files into a specific `airplanes` view:

```sql
CREATE
OR REPLACE VIEW airplanes AS (
    SELECT EmitterCategory,
       GBS,
       ModeA,
       TimeRecPosition,
       AircraftAddress,
       Latitude,
       Longitude,
       GeometricAltitude,
       FlightLevel,
       BarometricVerticalRate,
       CAST(GeoVertRateExceeded AS DOUBLE)     AS GeoVertRateExceeded,
       CAST(GeometricVerticalRate AS DOUBLE)   AS GeometricVerticalRate,
       GroundSpeed,
       TrackAngle,
       regexp_extract(Callsign, '([0-9A-Z]+)') AS Callsign,
       AircraftStopped,
       GroundTrackValid,
       GroundHeadingProvided,
       MagneticNorth,
       SurfaceGroundSpeed,
       SurfaceGroundTrack,
       CAST(month AS INT) AS month,
       site,
       year,
    FROM read_parquet('{}/adsb/**/*.parquet', hive_partitioning = true)
);
```

with "{}" being replaced by the full path of the datalake.

- View `drones` is created from the parquet files and updated for the new columns:

```sql
CREATE
OR REPLACE VIEW drones
AS
(
    SELECT *,
            dist_2d(longitude, latitude, home_lon, home_lat)                        as home_distance_2d,
            dist_3d(longitude, latitude, altitude, home_lon, home_lat, home_height) as home_distance_3d
    FROM read_parquet('drones/**/*.parquet')
);
```

with "{}" being replaced by the full path of the datalake.

We can re-create the whole Hive tree with this command:

```sql
COPY drones TO 'drones' (FORMAT parquet, PARTITION_BY (year, month), COMPRESSION 'zstd', FILENAME_PATTERN "drones_{i}");
```

### Tables

Store data for the sites

```sql
CREATE TABLE sites
(
    id        INTEGER PRIMARY KEY,
    name      VARCHAR NOT NULL,
    code      VARCHAR NOT NULL,
    latitude  FLOAT   NOT NULL,
    longitude FLOAT   NOT NULL,
);
```

Store the various antennas

```sql
CREATE TABLE antennas
(
    id          INTEGER PRIMARY KEY,
    type        VARCHAR,
    name        VARCHAR NOT NULL,
    owned       BOOLEAN,
    description VARCHAR,
);
```

Store all installations of one antenna on one site (1:1)

```sql
CREATE TABLE installations
(
    id         INTEGER PRIMARY KEY,
    site_id    INTEGER,
    antenna_id INTEGER,
    start_at   TIMESTAMP_NS NOT NULL,
    end_at     TIMESTAMP_NS NOT NULL,
    FOREIGN KEY (site_id) REFERENCES sites (id),
    FOREIGN KEY (antenna_id) REFERENCES antennas (id),
);
```

The main table for the airplane-drone proximity data:

```sql
CREATE
OR REPLACE TABLE airplane_prox (
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
  distance_home_m  INT,
)
```

We keep a table for statistics, updated each time we do a calculation for any given day.

```sql
CREATE TABLE daily_stats
(
    date       DATE,
    planes     BIGINT,
    drones     BIGINT,
    potential  INT,
    encounters INT,
    distance   FLOAT,
    proximity  FLOAT,
)
```
