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
