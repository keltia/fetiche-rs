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

create the two additional columns:

```sql
ALTER TABLE drones
  ADD COLUMN home_distance_2d FLOAT;
ALTER TABLE drones
  ADD COLUMN home_distance_3d FLOAT;
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

