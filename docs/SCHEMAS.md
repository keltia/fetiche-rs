## List of tables, views and general DB-related

### Tables

- DuckDB tables for sites, antennas and time intervals for installations of each antenna

- `antennas.csv`

```text
┌─────────────┬─────────────┬─────────┬─────────┬─────────┬─────────┐
│ column_name │ column_type │  null   │   key   │ default │  extra  │
│   varchar   │   varchar   │ varchar │ varchar │ varchar │ varchar │
├─────────────┼─────────────┼─────────┼─────────┼─────────┼─────────┤
│ ID          │ BIGINT      │ NO      │ PRI     │         │         │
│ TYPE        │ VARCHAR     │ YES     │         │         │         │
│ NAME        │ VARCHAR     │ YES     │         │         │         │
│ OWNED       │ BOOLEAN     │ YES     │         │         │         │
│ DESCRIPTION │ VARCHAR     │ YES     │         │         │         │
└─────────────┴─────────────┴─────────┴─────────┴─────────┴─────────┘
```

- `sites.csv`

```text
┌─────────────┬─────────────┬─────────┬─────────┬─────────┬─────────┐
│ column_name │ column_type │  null   │   key   │ default │  extra  │
│   varchar   │   varchar   │ varchar │ varchar │ varchar │ varchar │
├─────────────┼─────────────┼─────────┼─────────┼─────────┼─────────┤
│ id          │ INTEGER     │ NO      │ PRI     │         │         │
│ name        │ VARCHAR     │ NO      │         │         │         │
│ code        │ VARCHAR     │ NO      │         │         │         │
│ latitude    │ FLOAT       │ NO      │         │         │         │
│ longitude   │ FLOAT       │ NO      │         │         │         │
└─────────────┴─────────────┴─────────┴─────────┴─────────┴─────────┘
```

- `installations.csv`

```text
┌─────────────┬─────────────┬─────────┬─────────┬─────────┬─────────┐
│ column_name │ column_type │  null   │   key   │ default │  extra  │
│   varchar   │   varchar   │ varchar │ varchar │ varchar │ varchar │
├─────────────┼─────────────┼─────────┼─────────┼─────────┼─────────┤
│ ID          │ BIGINT      │ YES     │         │         │         │
│ SITE_ID     │ BIGINT      │ YES     │         │         │         │
│ ANTENNA_ID  │ BIGINT      │ YES     │         │         │         │
│ START_AT    │ DATE        │ YES     │         │         │         │
│ END_AT      │ DATE        │ YES     │         │         │         │
│ COMMENT     │ VARCHAR     │ YES     │         │         │         │
└─────────────┴─────────────┴─────────┴─────────┴─────────┴─────────┘
```

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
       date_part('year', timestamp) as year, 
           date_part('month', timestamp) as month,
           dist_2d(longitude, latitude, home_lon, home_lat) as home_distance_2d,
           dist_3d(longitude, latitude, altitude, home_lon, home_lat, home_height) as home_distance_3d
FROM read_csv('drones/**/drones_*.parquet'));
```

We can re-create the whole Hive tree with this command:

```sql
COPY drones TO 'drones' (FORMAT parquet, PARTITION_BY (year, month), COMPRESSION 'zstd', FILENAME_PATTERN "drones_{i}");
```

> NOTE: See `process-data/src/tasks/setup.rs` for the wrappers.

The `encounters` table looks like this:

```sql
DROP SEQUENCE IF EXISTS id_encounter;
CREATE SEQUENCE id_encounter;
CREATE TABLE encounters
(
    id           INT DEFAULT nextval('id_encounter'),
    en_id        VARCHAR,
    dt           BIGINT,
    time         VARCHAR,
    journey      INT,
    drone_id     VARCHAR,
    model        VARCHAR,
    callsign     VARCHAR,
    addr         VARCHAR,
    site         VARCHAR,
    distance     FLOAT,
    distancelat  FLOAT,
    distancevert FLOAT,
    PRIMARY KEY (dt, journey)
)
```

The `en_id` field is a unique ID generated from the date and the sequence number with the `YYYYMMDD_(journey)_(id)`
format, and we use a macro for this, see below.

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

### Support Macros

2D (using `spatial`)

```sql
CREATE
MACRO dist_2d(px, py, dx, dy) AS
  ST_Distance_Spheroid(ST_Point(px, py), ST_Point(dx, dy));
```

> NOTE: ST_Distance_Spheroid() is undocumented, it was merged
> in [this PR](https://github.com/duckdb/duckdb_spatial/pull/74).

3D (classical Euclidean)

```sql
CREATE
MACRO dist_3d(px, py, pz, dx, dy, dz) AS
  sqrt(pow(dist_2d(px, py, dx, dy), 2) + pow((pz - dz), 2));
```

Conversion from nautical miles to deg

```sql
CREATE
MACRO nm_to_deg(nm) AS
  nm * 1.852 / 111111.11;
```

Conversion from deg to meters

```sql
CREATE
MACRO deg_to_m(deg) AS
  deg * 111111.11;
```

Generate a unique ID for a given encounter

```sql
CREATE
MACRO encounter(tm, journey, id) AS
  printf('%04d%02d%02d_%d_%d', datepart('year', CAST(tm AS DATE)), datepart('month', CAST(tm AS DATE)), datepart('day', CAST(tm AS DATE)), journey, id);
```
