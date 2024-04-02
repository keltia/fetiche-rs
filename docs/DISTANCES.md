# Description of the calculations required

## Dataset

See [SCHEMA.md](SCHEMA.md) for the description of the various tables & macros involved.

### Main static data (stored in parquet files)

- Parquet files for ASD data from 2021/7 to 2024/1      (490k entries)
- Parquet files for ADS-B from 2023/1 to 2024/1 for different sites (2.9g entries)

During the calculations, tables will be created to store the intermediate selections for drones and airplanes. These
will be named `today` for the planes and `candidates` for the drones. The results will be store into a general table
called `today_close` from which we will derive our final results for the minimal distance (`encounters`) and the list of
planes nearby each drone point.

> NOTE: This schema require all calculations to be made in sequence, not in parallel. A possible evolution could be to
> allow for parallel processing by creating temporary tables named from the "year/month/day".

## List of calculations

See [dist.rs](../process-data/src/cmds/distances/planes/compute.rs) for all details.

- when creating the view on the drone points, 2D and 3D distances between drone and operator are automatically
  calculated

- for a given day or week or month
  do calculations for each site on a given day

- get all plane points with a 70nm BB around site
- get all drone points, grouped by journey also in a 70nm BB around site

- for a drone point:
  get all ADS-B positions with 3nm of said point +-2s out of the filtered list
  do calculation for plane/home/operator if any:
    * dist_3d(point_d, point_a)
      now we have all planes in prox of a given drone
      select all such points where the 3d distance between the 2 points are below 1nm aka 1852m into airplane_prox
    * for all these points, group them to generate a unique id for the (drone-plane-journey) tuple and insert it
      into the airplane_prox table.
    * we can extract the summary of a given day with `export distances -S`

- for an encounter:
  record encounter data:
    * timestamp
    * distance
    * site

  record drone data:
    * ID
    * position
    * model ID
    * type

  record plane data:
    * callsign
    * mode-s addr
        * position

