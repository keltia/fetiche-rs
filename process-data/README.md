# process-data

This is a CLI utility to load csv file containing ADS-B data and save them as Parquet files.

It is used to load and convert CSV files from the EMIT surveillance network into smaller and more usable [Parquet]
files.

This utility uses DuckDB and the tables/SQL as explained in [GEO.md](../docs/GEO.md)

## USAGE

<details>
<summary>`process-data --help`</summary>

```text
```

</details>

# Available tasks

## Proximity calculation

Using drone data from ASD and ADS-B data from EMIT, calculate various metrics:

For a given site, process data and calculate

- 3D distance between a drone and nearby airplanes
- 2D distance
- Number of encounters below 1nm, between 1 and 3 nm
- Save data about each encounter for both drone & airplane

### Data selection

- drones

```sql

```

## Trajectory categorisation

Using an ML system to classify the different kind of trajectory we can expect from a drone. Requires binding to python.

## Performance

[datafusion]: https://crates.io/crates/arrow-datafusion

[DuckDB]: https://duckdb.org/
