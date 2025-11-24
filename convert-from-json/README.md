# convert-from-json

A simple JSON-to-CSV or Parquet file converter utility using Polars, part of the **fetiche** ecosystem.

## Description

This tool converts JSON Lines (JSONL) formatted files to CSV or Parquet while preserving the column ordering. It uses
the [Polars](https://pola.rs/) data processing library to handle the conversion, which prevents the column reordering
issues that can occur with other tools or simple map-based converters.

It can replace `bdt(1)`  for this usage.

## Features

- Converts JSONL files to CSV & Parquet format.
- Preserves column ordering in output.
- Automatically infers schema from input data (checks first 10 records).
- Adds headers to the output CSV file.
- Uses minimal quoting (only when necessary).

## Usage

```shell
convert-from-json <input_file>
```

for a CSV file.

```shell
convert-from-json -P <input_file>
```

for a Parquet file.

It is assumed to be a json file. Schema will be inferred from the first 10 records and it will
generate a csv file with headers with the same basename as the input file.

## Benchmarks

json2parquet is using parquet-rs v57.0.1
convert-to-json is using polars v0.52

Using zstd for both cases, the latter uses Zstd compression factor of 8, not sure for the former.

```text
❯ hyperfine -w 3 -i "~/.cargo/bin/json2parquet -c zstd bigjson.json bigjson.parquet"
Benchmark 1: ~/.cargo/bin/json2parquet -c zstd bigjson.json bigjson.parquet
  Time (mean ± σ):      1.391 s ±  0.014 s    [User: 1.350 s, System: 0.034 s]
  Range (min … max):    1.375 s …  1.419 s    10 runs  

❯ ll bigjson*
-rw-r--r--@ 1 roberto  wheel  76287110 Nov 27 16:05 bigjson.json
-rw-r--r--@ 1 roberto  wheel   9150753 Nov 27 16:16 bigjson.parquet
  
  ❯ hyperfine -w 3 -i "~/.cargo/bin/convert-from-json -P bigtwo.json"
Benchmark 1: ~/.cargo/bin/convert-from-json -P bigtwo.json
  Time (mean ± σ):     193.2 ms ±   6.7 ms    [User: 685.7 ms, System: 55.5 ms]
  Range (min … max):   187.0 ms … 207.6 ms    15 runs
  
❯ ll bigtwo*
lrwxr-xr-x@ 1 roberto  wheel       12 Nov 27 16:13 bigtwo.json@ -> bigjson.json
-rw-r--r--@ 1 roberto  wheel  2523516 Nov 27 16:14 bigtwo.parquet
```

Polars-based version is 7.2 times faster.
File is 3.6 times smaller. (WHY?)
convert-to-json does *not* sort field names.





