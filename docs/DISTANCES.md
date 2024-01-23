# Description of the calculations required

## Dataset

- Parquet files for ASD data from 2021/7 to 2024/1      (490k entries)
- Parquet files for ADS-B from 2023/1 to 2024/1 for different sites (1.1g entries)
- DuckDB tables for sites, antennas and time intervals for installations of each antenna

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
      select minimum distance -> encouter

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

