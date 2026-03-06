# convert-from-json

A simple CLI interface to the airport data hosted by [OurAirports](https://ourairports.com/), part of the **fetiche**
ecosystem.
This utility fetch the csv file, and converts it into parquet. [Polars](https://pola.rs) is used to access the data.

## Description

## Features

- Converts JSONL files to CSV & Parquet format.
- Preserves column ordering in output.
- Automatically infers schema from input data (checks first 10 records).
- Adds headers to the output CSV file.
- Uses minimal quoting (only when necessary).

## Usage

```shell
find-airport --help
```
