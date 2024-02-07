# Description of the calculations required

## Dataset

- Parquet files for ASD data from 2021/7 to 2024/1      (490k entries)
- Parquet files for ADS-B from 2023/1 to 2024/1 for different sites (2.9g entries)
- DuckDB tables for sites, antennas and time intervals for installations of each antenna

## Other tables

- all ADS-B points from the parquet files into a specific `airplanes` view:

```sql
CREATE VIEW airplanes AS
SELECT *
FROM read_parquet('/Users/acute/data/adsb/**/*.parquet', hive_partitioning = true);
```

- Table `drones` is create from the parquet files then updated by the distance to home task.

```sql
CREATE TABLE drones AS
SELECT *
FROM read_parquet('/Users/acute/data/drones/**/*.parquet', hive_partitioning = true);
```

## See `process-data/src/tasks/setup.rs` for the wrappers.

Create the two additional columns:

```sql
ALTER TABLE drones
  ADD COLUMN home_distance_2d FLOAT;
ALTER TABLE drones
  ADD COLUMN home_distance_3d FLOAT;
```

During the calculations, tables will be created to store the intermediate selections for drones and airplanes. These
will be named `today` for the planes and `candidates` for the drones. The results will be store into a general table
called `today_close` from which we will derive our final results for the minimal distance (`encounters`) and the list of
planes nearby each drone point.

The `encounters` table looks like this:

```sql
DROP SEQUENCE IF EXISTS id_encounter;
CREATE SEQUENCE id_encounter;
CREATE TABLE encounters
(
  id       INT DEFAULT nextval('id_encounter'),
  en_id    VARCHAR,
  dt       BIGINT,
  time VARCHAR,
  journey  INT,
  drone_id VARCHAR,
  model    VARCHAR,
  callsign VARCHAR,
  addr     VARCHAR,
  site VARCHAR,
  distance FLOAT,
  PRIMARY KEY (dt, journey)
)
```

The `en_id` field is a unique ID generated from the date and the sequence number with the `YYYYMMDD_(journey)_(id)`
format.

```sql
CREATE
MACRO encounter(tm, journey, id) AS
  printf("%04d%02d%02d_%d_%d", year(CAST(tm AS DATE)), month(CAST(tm AS DATE)), day(CAST(tm AS DATE)), journey, id);
```

We will add some more macros/functions as well

2D (without using `spatial`)

```sql
CREATE
MACRO dist_2d(px, py, dx, dy) AS
  sqrt(pow((px - dx),2) + pow((py - dy), 2));
```

3D

```sql
CREATE
MACRO dist_3d(px, py, pz, dx, dy, dz) AS
  sqrt(pow((px - dx),2) + pow((py - dy), 2) + pow((pz - dz), 2));
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

## List of calculations

- for a given day or week or month
  do calculations for each site

- for a site:
  get all drone points, grouped by journey
  filter all ADS-B positions with a BB of 75nm around site

- for a drone point:
  get all ADS-B positions with 3nm of said point +-2s out of the filtered list
  do calculation for plane/home/operator if any:
    * dist_3d(point_d, point_a)
    * dist_3d(point_d, point_h)
    * dist_3d(point_d, point_o)
      select minimum distance -> encounter

- for an encounter:
  record encounter data:
    * timestamp
    * distances
    * site

  record drone data:
    * ID
    * position
    * model ID
    * type

  record plane data:
    * callsign
    * mode-s
    * position

