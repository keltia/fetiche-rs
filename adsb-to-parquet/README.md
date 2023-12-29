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

[Parquet]: https://parquet.apache.org/docs/file-format/
