# process-data

This is a CLI utility to load csv file containing ADS-B data and save them as Parquet files.

It is used to load and convert CSV files from the EMIT surveillance network into smaller and more usable [Parquet]
files.

## USAGE

<details>
<summary>`process-data --help`</summary>

```text
```

</details>

## Performance

The current version of `adsb-to-parquet` uses either [datafusion] to read and write CSV/Parquets files.
For small to medium size files [arrow2] is fast enough but for larger files (over a few dozens megabytes or 20k lines),
[datafusion] (the default) is much faster, esp for reading the large input file. 

[arrow2]: https://crates.io/crates/arrow2

[datafusion]: https://crates.io/crates/arrow-datafusion

[DuckDB]: https://duckdb.org/
