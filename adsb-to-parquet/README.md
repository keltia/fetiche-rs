# adsb-to-parquet

This is a CLI utility to load csv file containing ADS-B data and save them as Parquet files.

It is used to load and convert CSV files from the EMIT surveillance network into smaller and more usable [Parquet]
files.

## USAGE

<details>
<summary>`adsb-to-parquet help`</summary>

```text
$ adsb-to-parquet --help
Load ADS-B data as CSV and save it as Parquet.

Usage: adsb-to-parquet [OPTIONS] <NAME>

Arguments:
  <NAME>  Filename, can be just the basename and .csv/.parquet are implied

Options:
  -N, --no-header        Has headers or not?
  -o, --output <OUTPUT>  Output file (default is stdout)
  -d <DELIM>             Delimiter for csv files  [default: ,]
  -h, --help             Print help
```

</details>

## Performance

The current version of `adsb-to-parquet` uses `arrow2` to read and write CSV/Parquets files. For small to medium
size files it is fast enough but for larger files (over a few dozens megabytes or 20k lines), the `datafusion`-based
version (see in `df-csv.rs`) is much faster, esp for reading the large input file.

## NOTE

An alternative way is to use [qsv] as a command-line utility for this although saving to parquet is using [DuckDB] for
the writing part which makes it much bigger.

[Parquet]: https://parquet.apache.org/docs/file-format/
