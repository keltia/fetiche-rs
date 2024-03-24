# Description of the various tables & queries and their implementation

We consider two databases:

- DuckDB: currently used as an embedded database so it is fast as there is no latency due to network
- Clickhouse: coming from Yandex & Cloudflare
- Databend: was the first candidate, but it is lacking in Geo functions

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
CREATE VIEW airplanes AS
SELECT *
FROM read_parquet('adsb/**/*.parquet', hive_partitioning = true);
```

- View `drones` is created from the parquet files and updated for the new columns:

```sql
CREATE VIEW drones
AS
(
select *,
       dist_2d(longitude, latitude, home_lon, home_lat)                        as home_distance_2d,
       dist_3d(longitude, latitude, altitude, home_lon, home_lat, home_height) as home_distance_3d
FROM read_parquet('drones/**/*.parquet')
    );
```

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
    id        INTEGER PRIMARY KEY,
    name      VARCHAR NOT NULL,
    code      VARCHAR NOT NULL,
    latitude  FLOAT   NOT NULL,
    longitude FLOAT   NOT NULL,
) ENGINE MergeTree 
    COMMENT 'All sites with an antenna in time.';
```

```sql
-- Store one antenna
--
CREATE TABLE acute.antennas
(
    id          INTEGER PRIMARY KEY,
    type        VARCHAR,
    name        VARCHAR NOT NULL,
    owned       BOOLEAN,
    description VARCHAR,
) ENGINE MergeTree
    COMMENT 'All known antennas.';
```

```sql
-- Store one installation of one antenna on one site 1:1
--
CREATE TABLE acute.installations
(
    id         INTEGER PRIMARY KEY,
    site_id    INTEGER,
    antenna_id INTEGER,
    start_at   TIMESTAMP,
    end_at     TIMESTAMP,
    comment    VARCHAR,
    FOREIGN KEY (site_id) REFERENCES sites (id),
    FOREIGN KEY (antenna_id) REFERENCES antennas (id),
) ENGINE MergeTree 
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
  distance_home_m  INT,
)
    ENGINE = MergeTree PRIMARY KEY (time, journey)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance.';
```

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
) ENGINE = MergeTree PRIMARY KEY (date)
    COMMENT 'Statistics for a day run.'
```

