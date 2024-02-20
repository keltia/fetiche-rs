# adsb-to-parquet

This is a CLI utility to load csv file containing ADS-B data and save them as Parquet files.

It is used to load and convert CSV files from the EMIT surveillance network into smaller and more usable [Parquet]
files.

> NOTE: as much as I like my own code, [bdt] does this and more, although you will need my patch to align it with the
> current `adsb-to-parquet` in terms of control of the options and the output-as-a-single-file one.

> UPDATE: my pull-request was merged by Andy Grove on 2024/02/19 so github and next version will have it, yeah.

This means I might just replace `adsb-to-parquet` with bdt :)

## USAGE

<details>
<summary>`adsb-to-parquet help`</summary>

```text
$ adsb-to-parquet --help
Load ADS-B data as CSV and save it as Parquet.

Usage: adsb-to-parquet.exe [OPTIONS] <NAME>

Arguments:
  <NAME>  Filename, can be just the basename and .csv/.parquet are implied

Options:
  -A, --arrow2           Use arrow2 instead of datafusion?
  -N, --no-header        Has headers or not?
  -o, --output <OUTPUT>  Output file (default is stdout)
  -d <DELIM>             Delimiter for csv files [default: ,]
  -h, --help             Print help
```

</details>

## Performance

The current version of `adsb-to-parquet` uses either [datafusion] or [arrow2] to read and write CSV/Parquets files.
For small to medium size files [arrow2] is fast enough but for larger files (over a few dozens megabytes or 20k lines),
[datafusion] (the default) is much faster, esp for reading the large input file. Use `-A/--arrow2`  to shift to
[arrow2].

## Benchmark

Done on a PC running Windows 10 22H2, Intel i5-9600K @3.7 GHz, 16 GB RAM, Samsung M2 970 NVMe SSD.

Using Rust 1.77.0-nightly for SIMD instructions, release build. Compression for parquet file is Zstd at 8.

### 20,000 lines

```text
❯ hyperfine --warmup 3 'adsb-to-parquet -d : -o test-df.parquet test-bench.csv'
Benchmark 1: adsb-to-parquet -d : -o test-df.parquet test-bench.csv
  Time (mean ± σ):     360.3 ms ±   7.6 ms    [User: 145.3 ms, System: 225.0 ms]
  Range (min … max):   348.3 ms … 371.0 ms    10 runs

❯ hyperfine --warmup 3 'adsb-to-parquet -d : -A -o test-arw.parquet test-bench.csv'
Benchmark 1: adsb-to-parquet -d : -A -o test-arw.parquet test-bench.csv
  Time (mean ± σ):     403.1 ms ±   5.7 ms    [User: 224.4 ms, System: 190.6 ms]
  Range (min … max):   396.5 ms … 413.9 ms    10 runs
```

Pretty close times. Size are comparable too:

```text
-a----        30/12/2023     19:38         651711 test-arw.parquet
-a----        30/12/2023     19:38         620790 test-df.parquet
```

### 100,000 lines

```text
❯ hyperfine  --warmup 3 'adsb-to-parquet -d : -o test-df-100000.parquet test-100000.csv'
Benchmark 1: adsb-to-parquet -d : -o test-df-100000.parquet test-100000.csv
  Time (mean ± σ):     889.0 ms ±  10.8 ms    [User: 756.9 ms, System: 394.4 ms]
  Range (min … max):   879.0 ms … 908.7 ms    10 runs

❯ hyperfine  --warmup 3 'adsb-to-parquet -d : -A -o test-arw-100000.parquet test-100000.csv'
Benchmark 1: adsb-to-parquet -d : -A -o test-arw-100000.parquet test-100000.csv
  Time (mean ± σ):      1.542 s ±  0.058 s    [User: 1.907 s, System: 0.305 s]
  Range (min … max):    1.500 s …  1.685 s    10 runs
```

```text
-a----        30/12/2023     19:36        5343676 test-arw-100000.parquet
-a----        30/12/2023     19:36        4983255 test-df-100000.parquet
```

On the size: the current code uses 500,000 as the batch size for reading the csv data, which means you will have several
row groups in the resulting parquet file when reading larger files with [arrow2].  [datafusion] uses a different
threshold
so there will be less row groups. It will use less CPU time for reading as well.

On a humongous file (1,700,000+ lines) :

```text
❯ hyperfine --warmup 2 ' adsb-to-parquet -d : .\rec-2023-12-01.csv -o test-huge.parquet'
Benchmark 1:  adsb-to-parquet.exe -d : .\rec-2023-12-01.csv -o test-huge.parquet
  Time (mean ± σ):      7.632 s ±  0.100 s    [User: 12.829 s, System: 3.565 s]
  Range (min … max):    7.476 s …  7.843 s    10 runs

❯ hyperfine --warmup 2 ' adsb-to-parquet -A -d : .\rec-2023-12-01.csv -o test-huge.parquet'
Benchmark 1:  adsb-to-parquet.exe -A -d : .\rec-2023-12-01.csv -o test-huge.parquet
  Time (mean ± σ):     25.615 s ±  0.193 s    [User: 34.056 s, System: 3.809 s]
  Range (min … max):   25.417 s … 26.014 s    10 runs
```

```text
-a----        28/12/2023     13:17      563863172 rec-2023-12-01.csv
-a----        30/12/2023     19:57       99344641 test-huge-arw.parquet
-a----        30/12/2023     20:00       71679092 test-huge.parquet

❯ wc -l  rec-2023-12-01.csv
1731948 rec-2023-12-01.csv
```

There we begin to see the size differences.

## NOTE

An alternative way is to use [qsv] as a command-line utility for this although saving to parquet is using [DuckDB] for
the writing part which makes it much bigger.

[arrow2]: https://crates.io/crates/arrow2

[bdt]: https://github.com/datafusion-contrib/bdt

[datafusion]: https://crates.io/crates/arrow-datafusion

[qsv]: https://github.com/jqnatividad/qsv

[DuckDB]: https://duckdb.org/

[Parquet]: https://parquet.apache.org/docs/file-format/
